//! Stream-message handler. The agent loop and other background tasks
//! emit `StreamMsg` events; `handle_stream` is the single dispatcher,
//! routing each variant to a focused per-category handler:
//!
//!   - `chat`       — assistant chunks, turn lifecycle, errors.
//!   - `tools`      — tool start/result + the confirm popup intercept.
//!   - `sessions`   — `/sessions`, `/load`, `/more` results.
//!   - `compaction` — `/compact` done/error + memory persistence.
//!   - `system`     — host change, update notifications, `/settings`.
//!
//! `persist_assistant_if_any` and `api_message_to_chat` are the small
//! shared helpers used by more than one of the above; they live here
//! because they sit between the chat / sessions / compaction modules.

use tokio::sync::mpsc;

use crate::api::ApiOp;
use crate::ollama::ChatMessage;

use super::{App, StreamMsg};

mod chat;
mod compaction;
mod sessions;
mod system;
mod tools;

impl App {
    pub fn handle_stream(&mut self, msg: StreamMsg, tx: &mpsc::UnboundedSender<StreamMsg>) {
        match msg {
            StreamMsg::Chunk(text) => self.on_chunk(text),
            StreamMsg::AssistantTurnEnded { tool_calls } => {
                self.on_assistant_turn_ended(tool_calls)
            }
            StreamMsg::ToolStart { name, args } => self.on_tool_start(name, args),
            StreamMsg::ToolResult { output } => self.on_tool_result(output),
            StreamMsg::NewAssistantTurn => self.on_new_assistant_turn(),
            StreamMsg::ConfirmRequest(req) => self.on_confirm_request(req),
            StreamMsg::Done {
                prompt_tokens,
                completion_tokens,
            } => self.on_done(prompt_tokens, completion_tokens, tx),
            StreamMsg::Error(e) => self.on_error(e),
            StreamMsg::CompactionDone {
                summary,
                prompt_tokens,
                completion_tokens,
            } => self.on_compaction_done(summary, prompt_tokens, completion_tokens, tx),
            StreamMsg::CompactionError(e) => self.on_compaction_error(e),
            StreamMsg::UpdateAvailable(latest) => {
                self.update_available = Some(latest);
            }
            StreamMsg::UpdateInfo(text) => {
                self.push_info(text);
            }
            StreamMsg::Settings(text) => self.on_settings(text),
            StreamMsg::UpdateResult { ok, text } => self.on_update_result(ok, text),
            StreamMsg::Models { models, base } => self.on_models(models, base),
            StreamMsg::SessionList(rows) => self.on_session_list(rows),
            StreamMsg::Loaded { session, messages } => self.on_loaded(session, messages),
            StreamMsg::MoreLoaded { messages } => self.on_more_loaded(messages),
            StreamMsg::OpenRouterModelsRefreshed(models) => {
                self.on_openrouter_models_refreshed(models)
            }
        }
    }

    /// Persist the trailing assistant message if it's the final reply
    /// (no tool_calls) and non-empty. Otherwise drop empties.
    pub(super) fn persist_assistant_if_any(&mut self) {
        if let Some(last) = self.messages.last() {
            if last.role != "assistant" {
                return;
            }
            let has_tool_calls = last
                .tool_calls
                .as_ref()
                .map(|tc| !tc.is_empty())
                .unwrap_or(false);
            if last.content.trim().is_empty() && !has_tool_calls {
                self.messages.pop();
            } else if !has_tool_calls && !last.content.trim().is_empty() {
                // Strip the `<think>…</think>` reasoning block before persisting.
                // It's useful in-session as a foldable block but is in-flight
                // scratch — durable storage should hold only the visible answer.
                let raw = &last.content;
                let content = match raw.find("</think>") {
                    Some(idx) => raw[idx + "</think>".len()..]
                        .trim_start_matches(['\n', '\r'])
                        .to_string(),
                    None => raw.clone(),
                };
                if content.trim().is_empty() {
                    return;
                }
                let model = self.model.clone();
                if let Some(api_tx) = &self.api_tx {
                    let _ = api_tx.send(ApiOp::AssistantMessage { content, model });
                }
            }
        }
    }
}

/// Convert a persisted `api::Message` (DB shape) into the in-memory
/// `ChatMessage` the renderer uses. Carries `name` and `tool_calls`
/// across the boundary — without this, `/load` and `/more` would drop
/// both fields and tool rows would render as `tool({})` because the
/// renderer couldn't find their function name or look up their args.
pub(super) fn api_message_to_chat(m: &crate::api::Message) -> ChatMessage {
    ChatMessage {
        role: m.role.clone(),
        content: m.content.clone(),
        name: m.name.clone(),
        // `api::Message` stores `tool_calls` as raw JSON (it's pass-through
        // from whatever the model emitted). Best-effort parse into the
        // typed shape — if a legacy row has a malformed value, we drop it
        // rather than crash; the user will see `tool({})` for that single
        // message, same as before this fix.
        tool_calls: m
            .tool_calls
            .as_ref()
            .and_then(|v| serde_json::from_value::<Vec<crate::ollama::ToolCall>>(v.clone()).ok()),
        ..Default::default()
    }
}
