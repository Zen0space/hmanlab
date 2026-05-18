//! `StreamMsg` — every event the background tasks send back to the UI.
//!
//! The agent loop, the persistence writer, the `/update` task, etc.
//! all hold an `mpsc::UnboundedSender<StreamMsg>` and push variants
//! through it. `app::stream::handle_stream_msg` consumes them.

use crate::api::{Message, Session};
use crate::ollama::ToolCall;
use crate::tools;

pub enum StreamMsg {
    Chunk(String),
    Done {
        prompt_tokens: u32,
        completion_tokens: u32,
    },
    Error(String),
    Models {
        models: Vec<String>,
        base: String,
    },
    SessionList(Vec<Session>),
    Loaded {
        session: Session,
        messages: Vec<Message>,
    },
    MoreLoaded {
        messages: Vec<Message>,
    },
    /// Assistant turn just ended and produced tool calls (the assistant message
    /// content has already been streamed via `Chunk`).
    AssistantTurnEnded {
        tool_calls: Vec<ToolCall>,
    },
    /// Compaction (manual `/compact` or auto-triggered) finished — the
    /// model returned a summary that should replace the visible history.
    CompactionDone {
        summary: String,
        prompt_tokens: u32,
        completion_tokens: u32,
    },
    /// Compaction failed — surface the error and leave the existing
    /// history untouched.
    CompactionError(String),
    /// Background update check found a newer hmanlab on npm. Renders
    /// as a one-line notice in the header — never blocks anything.
    UpdateAvailable(String),
    /// `/update` finished. `ok` is the exit status; `text` is the
    /// message to surface inline (success summary or failure cause).
    UpdateResult {
        ok: bool,
        text: String,
    },
    /// `/update` interim progress line (e.g., "0.1.4 → 0.1.5, installing…").
    /// Pushed to the chat as an info message so the user can see what
    /// the background task is doing without blocking.
    UpdateInfo(String),
    /// `/settings` finished gathering account + version info. The text
    /// is a pre-formatted multi-line block ready to render verbatim.
    Settings(String),
    /// Begin executing a tool.
    ToolStart {
        name: String,
        args: serde_json::Value,
    },
    /// Tool finished — its output replaces the placeholder content on the
    /// trailing `tool` message.
    ToolResult {
        output: String,
    },
    /// Start a fresh assistant placeholder for the next agent turn.
    NewAssistantTurn,
    /// The agent wants the user to confirm a risky action.
    ConfirmRequest(tools::ConfirmRequest),
}
