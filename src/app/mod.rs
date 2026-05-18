//! `App` — the central UI state struct. Field is per-feature, not per-tab;
//! everything the renderer needs and every handler mutates lands here.
//!
//! The actual *behavior* (event handling, stream processing, slash
//! commands, workspace/trust state, etc.) lives in submodules; this
//! file is just the struct + constructor + module wiring + a couple of
//! tiny shared helpers (`fresh_textarea`, `seed_sidebar_top_level`).

use std::collections::HashSet;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tui_textarea::TextArea;

use crate::api::{self, ApiOp};
use crate::config::ExtraModel;
use crate::ollama::{ChatMessage, Client};
use crate::tools;

mod backend;
mod commands;
pub mod event;
mod heuristics;
pub mod inline;
mod input;
mod state;
mod stream;
mod stream_msg;
mod viewer;
pub mod workspace;

pub use backend::LlmBackend;
pub use inline::{InlinePopup, SLASH_COMMANDS};
pub use state::{AddModelStep, AppAction, DisconnectEntry, Mode, PickerEntry};
pub use stream_msg::StreamMsg;
pub use viewer::OpenFile;

use crate::api::Session;

/// Build a TextArea with no current-line underline (tui-textarea's default
/// behavior is to underline the cursor row, which looks like a stray line in
/// our chat input).
pub(super) fn fresh_textarea() -> TextArea<'static> {
    let mut ta = TextArea::default();
    ta.set_cursor_line_style(ratatui::style::Style::default());
    ta
}

impl App {
    /// Reset the sidebar state to defaults for the current workspace: clear
    /// any user expansion + reset scroll, then re-seed the expanded set with
    /// the workspace root and its immediate visible directories. Called once
    /// at startup (from `main`) and again whenever `/workspace` switches.
    pub fn seed_sidebar_top_level(&mut self) {
        self.expanded_dirs = crate::ui::initial_expanded(&self.workspace, self.workspace_trusted);
        self.sidebar_scroll = 0;
    }
}

pub struct App {
    pub client: Client,
    pub model: String,
    pub models: Vec<String>,
    pub messages: Vec<ChatMessage>,
    pub mode: Mode,
    pub picker_index: usize,
    pub input: TextArea<'static>,
    pub scroll: u16,
    pub follow: bool,
    pub status: String,
    pub generating: bool,
    pub workspace: PathBuf,
    pub pending_confirm: Option<tools::ConfirmRequest>,
    /// Armed after the last assistant turn ended with one of the Y/N trigger
    /// phrases. Pressing Y or N silently injects a hidden user reply.
    pub yn_pending: bool,
    /// True for the one turn after a Y-injection. If that turn produces only
    /// an intent announcement ("I'll look at…") with no tool calls, we
    /// auto-inject a continuation prompt. Capped to one retry per Y.
    pub awaiting_yn_followup: bool,
    /// BYOK extras the user has added (z.ai, etc.). Mirrors config.extra_models.
    pub extra_models: Vec<ExtraModel>,
    /// Active extra-provider model, if any. `None` means we're on Ollama.
    /// Tracked separately from `model` so two providers can list a model
    /// with the same name (e.g. `glm-4.7` on both z.ai plans) without
    /// the picker / routing getting confused about which one is active.
    pub selected_extra: Option<ExtraModel>,
    /// z.ai subscription (coding plan) key.
    pub zai_api_key: Option<String>,
    /// z.ai usage-based (pay-per-token) key.
    pub zai_usage_api_key: Option<String>,
    /// Ollama Cloud API key (Bearer to https://ollama.com). Independent of
    /// the local Ollama daemon at `client.base`.
    pub ollama_cloud_api_key: Option<String>,
    /// OpenCode Zen / Go API key (Bearer to opencode.ai/zen/v1).
    pub opencode_api_key: Option<String>,
    /// OpenRouter API key (Bearer to openrouter.ai/api/v1). Meta-provider
    /// for OpenAI / Anthropic / Google / Llama / DeepSeek / Qwen et al.
    pub openrouter_api_key: Option<String>,
    /// Entries rendered by the picker, built each time `open_picker` runs.
    pub picker_entries: Vec<PickerEntry>,
    pub add_model_step: AddModelStep,
    /// Provider being added in the current AddModel flow.
    pub add_model_provider: String,
    /// Free-text input for the AddModel modal (key or name).
    pub add_model_input: TextArea<'static>,
    pub session_picker_items: Vec<Session>,
    pub session_picker_index: usize,
    /// Rows shown by the `/disconnect` picker — one per currently-
    /// connected BYOK provider. Rebuilt by `open_disconnect_picker`.
    pub disconnect_entries: Vec<DisconnectEntry>,
    pub disconnect_index: usize,
    /// Set when `/load` brings in a saved session, so /more knows where to page from.
    pub loaded_session_id: Option<String>,
    pub oldest_loaded_msg_id: Option<i64>,
    /// True while a `/more` (manual or auto) request is in flight. Debounces
    /// scroll-triggered auto-loads so a single scroll gesture fires at most
    /// one request, and prevents `/more` typed during a load from queueing
    /// a duplicate.
    pub loading_more: bool,
    /// Set after a `MoreLoaded` response arrived empty — there's nothing
    /// older on the server for this session. Stops the auto-loader from
    /// hammering the API every time the user lands on `scroll == 0`. Reset
    /// on `/load`, `/new`, `/clear`.
    pub no_more_history: bool,
    /// Indices of tool messages currently shown expanded. Tool messages collapse
    /// by default to keep the chat readable; Ctrl+T toggles all of them.
    pub expanded_tools: HashSet<usize>,
    /// Indices of assistant messages whose `<think>` reasoning block is shown
    /// expanded. Like `expanded_tools`, collapsed by default. Ctrl+T toggles
    /// these alongside tool blocks; clicking a thinking row toggles just one.
    pub expanded_thoughts: HashSet<usize>,
    /// In-app text selection (since we capture the mouse to get an arrow cursor,
    /// native drag-select is disabled — we re-implement it).
    pub sel_start: Option<(u16, u16)>,
    pub sel_end: Option<(u16, u16)>,
    pub selecting: bool,
    /// Chat inner geometry from the last render — used for hit testing and
    /// selection clamping.
    pub chat_x: u16,
    pub chat_y: u16,
    pub chat_w: u16,
    pub chat_h: u16,
    /// Plain-text version of each rendered chat line, populated each frame by
    /// ui.rs so the copy routine can extract the selection.
    pub rendered_text_lines: Vec<String>,
    /// Logical line range per message (for click-to-expand on tools).
    pub message_line_ranges: Vec<(usize, u16, u16)>,
    /// Running token tally for the current session (resets on /new and /clear).
    pub total_prompt_tokens: u64,
    pub total_completion_tokens: u64,
    /// Prompt-token count from the most recent assistant turn. Drives the
    /// auto-compaction trigger in `send_to_llm` — when this exceeds
    /// [`compact::AUTO_COMPACT_THRESHOLD`], the next user message is
    /// queued behind a compaction pass.
    pub last_prompt_tokens: u32,
    /// True while a compaction call is in flight. Blocks concurrent
    /// generation, prevents re-entry, and gates UI affordances.
    pub compacting: bool,
    /// JoinHandle for the in-flight compaction task — abort target for
    /// `cancel()` and the cleanup point after CompactionDone.
    pub(super) compact_task: Option<JoinHandle<()>>,
    /// If auto-compaction was triggered by a user message, that message is
    /// stored here while the compaction runs. After `CompactionDone` the
    /// stream handler re-issues it via `send_to_llm` so the user's intent
    /// isn't lost.
    pub pending_after_compact: Option<String>,
    /// Monotonic counter incremented by the ~120 ms ticker in `main::run` while
    /// the agent is generating or a tool is running. Drives the breathing-color
    /// animation in `ui::chat`; stays still when the app is idle.
    pub anim_tick: u64,
    /// Index into `messages` of the tool placeholder currently executing —
    /// `Some` between the `ToolStart` and matching `ToolResult` stream events.
    /// Used by the renderer to apply the breathing style to the running row.
    pub active_tool_msg_idx: Option<usize>,
    /// File the user opened from the sidebar. While `Some`, the file viewer
    /// occupies the chat column and intercepts keys (Esc closes; PgUp/Down
    /// scroll). Cleared when the user closes or quits.
    pub open_file: Option<OpenFile>,
    /// Sidebar inner geometry stashed each frame so the mouse handler can
    /// hit-test clicks against the rendered tree.
    pub sidebar_x: u16,
    pub sidebar_y: u16,
    pub sidebar_w: u16,
    pub sidebar_h: u16,
    /// One row per visible sidebar entry — the **logical line index** (NOT
    /// screen Y), the absolute path, and whether it's a directory. Populated
    /// by `ui::sidebar` each frame. The click handler converts a screen row
    /// into the matching logical line via `(screen_y - sidebar_y) +
    /// sidebar_scroll` before looking up an entry here.
    pub sidebar_targets: Vec<(u16, PathBuf, bool)>,
    /// Directories the user has expanded in the sidebar. Workspace root is
    /// pre-seeded so the walker can use a single membership check at every
    /// level. Cleared and re-seeded on `/workspace`.
    pub expanded_dirs: HashSet<PathBuf>,
    /// Logical-line scroll offset for the sidebar (0 = top). Clamped to a
    /// valid range each frame by the renderer.
    pub sidebar_scroll: u16,
    /// JoinHandle for the agent task. Submodules need access to abort/clear it
    /// when the user cancels or a turn finishes; package-private so we can.
    pub(super) current_task: Option<JoinHandle<()>>,
    pub api: Option<api::Client>,
    pub api_tx: Option<mpsc::UnboundedSender<ApiOp>>,
    /// Newer hmanlab version advertised by npm, if the background
    /// update check found one. Cleared until the check completes.
    pub update_available: Option<String>,
    /// Inline autocomplete popup overlaying the chat surface, if any.
    /// `Slash` when the user is typing `/<command>`, `File` when they're
    /// typing `@<path>`. Mutually exclusive; `None` otherwise.
    pub inline_popup: InlinePopup,
    /// Index of the placeholder `info` message pushed by `/settings` while
    /// the background `fetch_me` request is in flight. When the resolved
    /// `StreamMsg::Settings` arrives, the stream handler edits the message
    /// at this index in place instead of appending a second card — so
    /// "refresh" actually refreshes rather than stacking placeholders.
    pub pending_settings_msg_idx: Option<usize>,
    /// Absolute workspace paths the user has explicitly authorised via
    /// `/trust`. Persisted in `~/.config/hmanlab/config.json`.
    pub trusted_workspaces: Vec<PathBuf>,
    /// Cached "is `self.workspace` in `trusted_workspaces`" — recomputed
    /// whenever either side changes (startup, `/workspace`, `/trust`,
    /// `/untrust`). Used by the confirm interceptor in `app::stream`.
    pub workspace_trusted: bool,
    /// Scroll offset (in rendered lines) for the confirm popup body.
    /// Reset to 0 on each new ConfirmRequest; ↑↓/PgUp/PgDn move it in
    /// `handle_confirm`; clamped to a valid max by the renderer.
    pub confirm_scroll: u16,
    /// Last mouse cursor position observed from `MouseEventKind::Moved`
    /// events. Used by the chat renderer to highlight the hovered "reading
    /// N files" card row so users see it's clickable without needing a
    /// chevron or arrow icon. `(0, 0)` until the first Move event arrives.
    pub hover_x: u16,
    pub hover_y: u16,
    /// One row per card file entry rendered this frame: logical line
    /// index (NOT screen Y) + the message index it represents. Populated
    /// by `ui::chat` each frame; consumed by the same renderer after the
    /// paragraph is laid out to paint the hover overlay.
    pub card_row_targets: Vec<(u16, usize)>,
    /// Inner content width of the input box (cols), populated each frame
    /// by `chat::render_input`. The input event handler reads this to
    /// know when a typed character would push the current line past the
    /// visible edge and a soft-wrap should kick in.
    pub input_inner_w: u16,
}

impl App {
    pub fn new(
        client: Client,
        model: String,
        models: Vec<String>,
        workspace: PathBuf,
        api: Option<api::Client>,
        api_tx: Option<mpsc::UnboundedSender<ApiOp>>,
    ) -> Self {
        let mut input = fresh_textarea();
        input.set_placeholder_text(
            "Type a message, or /help for commands.  (Enter=send, Alt+Enter / Ctrl+J=newline)",
        );
        let db_state = if api.is_some() { "API on" } else { "API off" };
        let status = if models.is_empty() {
            format!(
                "No models — try /host <url> or check Ollama  ·  {db_state}  ·  ws={}",
                workspace.display()
            )
        } else {
            format!(
                "Ready — {} model(s)  ·  {db_state}  ·  ws={}  ·  /help for commands",
                models.len(),
                workspace.display()
            )
        };
        Self {
            client,
            model,
            models,
            messages: Vec::new(),
            mode: Mode::Chat,
            picker_index: 0,
            input,
            scroll: 0,
            follow: true,
            status,
            generating: false,
            workspace,
            pending_confirm: None,
            yn_pending: false,
            awaiting_yn_followup: false,
            extra_models: Vec::new(),
            selected_extra: None,
            zai_api_key: None,
            zai_usage_api_key: None,
            ollama_cloud_api_key: None,
            opencode_api_key: None,
            openrouter_api_key: None,
            picker_entries: Vec::new(),
            add_model_step: AddModelStep::Key,
            add_model_provider: crate::config::ZAI_SUBSCRIPTION_PROVIDER.to_string(),
            add_model_input: fresh_textarea(),
            session_picker_items: Vec::new(),
            session_picker_index: 0,
            disconnect_entries: Vec::new(),
            disconnect_index: 0,
            loaded_session_id: None,
            oldest_loaded_msg_id: None,
            loading_more: false,
            no_more_history: false,
            expanded_tools: HashSet::new(),
            expanded_thoughts: HashSet::new(),
            sel_start: None,
            sel_end: None,
            selecting: false,
            chat_x: 0,
            chat_y: 0,
            chat_w: 0,
            chat_h: 0,
            rendered_text_lines: Vec::new(),
            message_line_ranges: Vec::new(),
            total_prompt_tokens: 0,
            total_completion_tokens: 0,
            last_prompt_tokens: 0,
            compacting: false,
            compact_task: None,
            pending_after_compact: None,
            anim_tick: 0,
            active_tool_msg_idx: None,
            open_file: None,
            sidebar_x: 0,
            sidebar_y: 0,
            sidebar_w: 0,
            sidebar_h: 0,
            sidebar_targets: Vec::new(),
            expanded_dirs: HashSet::new(),
            sidebar_scroll: 0,
            current_task: None,
            api,
            api_tx,
            update_available: None,
            inline_popup: InlinePopup::None,
            pending_settings_msg_idx: None,
            trusted_workspaces: Vec::new(),
            workspace_trusted: false,
            confirm_scroll: 0,
            hover_x: 0,
            hover_y: 0,
            card_row_targets: Vec::new(),
            input_inner_w: 0,
        }
    }
}
