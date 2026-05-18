//! Chat-flow stream handlers: assistant token chunks, end-of-turn
//! bookkeeping, fresh-turn placeholders, the `Done` finalizer (with
//! auto-continue-after-Y logic), and the error finaliser.

use tokio::sync::mpsc;

use crate::api::ApiOp;
use crate::ollama::ToolCall;

use super::super::{App, StreamMsg};

impl App {
    pub(super) fn on_chunk(&mut self, text: String) {
        if let Some(last) = self.messages.last_mut() {
            if last.role == "assistant" {
                last.content.push_str(&text);
            }
        }
    }

    pub(super) fn on_assistant_turn_ended(&mut self, tool_calls: Vec<ToolCall>) {
        // Snapshot the assistant content + tool_calls before mutation,
        // then persist this intermediate turn so future fine-tunes can
        // see the model's tool-calling behavior, not just the final
        // text. Without this we'd only ever capture the closing reply.
        let snapshot: Option<(String, serde_json::Value)> =
            if let Some(last) = self.messages.last_mut() {
                if last.role == "assistant" && !tool_calls.is_empty() {
                    last.tool_calls = Some(tool_calls.clone());
                    let tc_value = serde_json::to_value(&tool_calls)
                        .unwrap_or_else(|_| serde_json::Value::Array(Vec::new()));
                    Some((last.content.clone(), tc_value))
                } else {
                    if last.role == "assistant" {
                        last.tool_calls = Some(tool_calls);
                    }
                    None
                }
            } else {
                None
            };
        if let (Some((content, tc_value)), Some(api_tx)) = (snapshot, self.api_tx.as_ref()) {
            let _ = api_tx.send(ApiOp::AssistantToolCalls {
                content,
                tool_calls: tc_value,
                model: self.model.clone(),
            });
        }
    }

    pub(super) fn on_new_assistant_turn(&mut self) {
        self.messages.push(crate::ollama::ChatMessage {
            role: "assistant".into(),
            content: String::new(),
            ..Default::default()
        });
        self.follow = true;
    }

    pub(super) fn on_done(
        &mut self,
        prompt_tokens: u32,
        completion_tokens: u32,
        tx: &mpsc::UnboundedSender<StreamMsg>,
    ) {
        self.persist_assistant_if_any();
        self.total_prompt_tokens = self
            .total_prompt_tokens
            .saturating_add(prompt_tokens as u64);
        self.total_completion_tokens = self
            .total_completion_tokens
            .saturating_add(completion_tokens as u64);
        // Track this turn's prompt size for the next auto-compact
        // check in `send_to_llm`.
        self.last_prompt_tokens = prompt_tokens;
        self.generating = false;
        self.current_task = None;
        self.active_tool_msg_idx = None;
        self.status = format!(
            "Ready  ·  this turn: {} in / {} out",
            prompt_tokens, completion_tokens
        );

        // Auto-continue: if this turn was the reply to a Y-injection
        // and the model only announced intent (no tool calls + 'I'll
        // do X' text), nudge it to actually do the thing. One retry.
        if self.awaiting_yn_followup {
            self.awaiting_yn_followup = false;
            if self.no_tools_since_last_user() && self.looks_like_intent_announcement() {
                self.inject_hidden_user(
                    "You announced intent but didn't act. Call the necessary tools and \
                     actually do the work now — don't restate the plan.",
                    tx,
                );
                return;
            }
        }

        self.yn_pending = self.last_assistant_invites_yn();
    }

    pub(super) fn on_error(&mut self, e: String) {
        self.persist_assistant_if_any();
        self.generating = false;
        self.current_task = None;
        self.active_tool_msg_idx = None;
        self.status = format!("Error: {e}");
    }
}
