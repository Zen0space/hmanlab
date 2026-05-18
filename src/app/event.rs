//! Top-level event dispatcher + the helpers that turn slash commands and
//! agent IO into chat state and outbound LLM calls.
//!
//! Input handling proper lives in `app/input/` — this file only routes
//! events to the right handler based on `self.mode` and `event` kind.
//! The slash-command dispatch (`handle_command`) and the chat-state
//! plumbing (`push_info`, `reset_input`, `inject_hidden_user`,
//! `send_to_llm`, `cancel`, `start_compact`) live here too because they
//! straddle the input/output boundary — they're called from input but
//! kick off the agent loop and stream-event pipeline.

use anyhow::Result;
use crossterm::event::{Event, KeyEventKind};
use tokio::sync::mpsc;

use crate::api::ApiOp;
use crate::ollama::ChatMessage;

use super::commands::Command;
use super::{fresh_textarea, App, AppAction, Mode, StreamMsg};

impl App {
    pub async fn handle_event(
        &mut self,
        event: Event,
        tx: &mpsc::UnboundedSender<StreamMsg>,
    ) -> Result<AppAction> {
        match event {
            Event::Mouse(m) => {
                self.handle_mouse(m, tx);
                Ok(AppAction::Continue)
            }
            Event::Key(key) => {
                if key.kind != KeyEventKind::Press {
                    return Ok(AppAction::Continue);
                }
                match self.mode {
                    Mode::ModelPicker => Ok(self.handle_picker(key)),
                    Mode::Confirm => Ok(self.handle_confirm(key)),
                    Mode::AddModel => Ok(self.handle_add_model(key)),
                    Mode::SessionPicker => Ok(self.handle_session_picker(key, tx)),
                    Mode::DisconnectPicker => Ok(self.handle_disconnect_picker(key)),
                    Mode::Chat => Ok(self.handle_chat(key, tx)),
                }
            }
            _ => Ok(AppAction::Continue),
        }
    }

    pub(in crate::app) fn handle_command(
        &mut self,
        cmd: Command,
        tx: &mpsc::UnboundedSender<StreamMsg>,
    ) -> AppAction {
        match cmd {
            Command::Model(None) => self.open_picker(),
            Command::Model(Some(name)) => self.switch_model(&name),
            Command::ListModels => self.list_models_inline(),
            Command::Clear => self.clear_history(),
            Command::Quit => return AppAction::Quit,
            Command::Help => self.show_help_inline(),
            Command::Host(url) => self.switch_host(url, tx),
            Command::New => self.new_session(),
            Command::ListSessions => self.list_sessions_inline(tx),
            Command::Load(prefix) => self.load_session(prefix, tx),
            Command::More => self.load_more(tx),
            Command::Workspace(path) => self.switch_workspace(path),
            Command::Compact => self.start_compact(tx, None),
            Command::Disconnect(name) => self.handle_disconnect(&name),
            Command::Update => self.start_update(tx),
            Command::Settings => self.show_settings(tx),
            Command::Trust => self.trust_current_workspace(),
            Command::Untrust => self.untrust_current_workspace(),
            Command::Unknown(name) => {
                self.push_info(format!(
                    "Unknown command: /{name}\nType /help to see available commands."
                ));
                self.status = format!("Unknown: /{name}");
            }
        }
        AppAction::Continue
    }

    pub(super) fn push_info(&mut self, content: String) {
        self.messages.push(ChatMessage {
            role: "info".into(),
            content,
            ..Default::default()
        });
        self.follow = true;
    }

    pub(in crate::app) fn reset_input(&mut self) {
        let mut fresh = fresh_textarea();
        fresh.set_placeholder_text(
            "Type a message, or /help for commands.  (Enter=send, Alt+Enter / Ctrl+J=newline)",
        );
        self.input = fresh;
    }

    /// Send a user message that goes to the model but is NOT rendered in the
    /// chat UI. Used by the Y/N quick-reply so accept/deny doesn't pollute the
    /// visible transcript.
    pub(super) fn inject_hidden_user(&mut self, text: &str, tx: &mpsc::UnboundedSender<StreamMsg>) {
        if (self.models.is_empty() && self.extra_models.is_empty()) || self.generating {
            return;
        }
        if let Some(api_tx) = &self.api_tx {
            // Persist the silent reply too — the session record stays coherent.
            let _ = api_tx.send(ApiOp::UserMessage {
                content: text.to_string(),
                model: self.model.clone(),
            });
        }
        self.messages.push(ChatMessage {
            role: "user".into(),
            content: text.into(),
            hidden: true,
            ..Default::default()
        });
        self.messages.push(ChatMessage {
            role: "assistant".into(),
            content: String::new(),
            ..Default::default()
        });
        self.generating = true;
        self.follow = true;
        self.status = format!("Generating with {}…", self.model);
        let history: Vec<ChatMessage> = self.messages[..self.messages.len() - 1]
            .iter()
            .filter(|m| matches!(m.role.as_str(), "user" | "assistant" | "tool"))
            .cloned()
            .collect();
        let Some(backend) = self.make_backend() else {
            self.generating = false;
            self.status = format!("No API key configured for model {}", self.model);
            return;
        };
        let model = self.model.clone();
        let workspace = self.workspace.clone();
        let tx = tx.clone();
        let handle = tokio::spawn(async move {
            crate::agent::agent_loop(backend, model, history, workspace, tx).await;
        });
        self.current_task = Some(handle);
    }

    pub(super) fn send_to_llm(&mut self, text: String, tx: &mpsc::UnboundedSender<StreamMsg>) {
        if self.models.is_empty() && self.extra_models.is_empty() {
            self.push_info(
                "Not connected to a model. Use /host <url> for Ollama, or /model to add a BYOK provider.".into(),
            );
            self.status = "No model".into();
            return;
        }

        // Auto-compaction: if the last assistant turn's prompt was over
        // threshold, fold the visible history into a summary first, then
        // re-issue this user message once compaction completes. Bail out
        // if we're already compacting (avoid re-entry) or generating.
        if !self.compacting
            && !self.generating
            && self.last_prompt_tokens > crate::compact::AUTO_COMPACT_THRESHOLD
            && self
                .messages
                .iter()
                .any(|m| !m.hidden && m.role == "assistant")
        {
            self.push_info(format!(
                "Context at {} tokens — compacting before sending so your next turn has room.",
                self.last_prompt_tokens
            ));
            self.start_compact(tx, Some(text));
            return;
        }

        if let Some(api_tx) = &self.api_tx {
            let _ = api_tx.send(ApiOp::UserMessage {
                content: text.clone(),
                model: self.model.clone(),
            });
        }

        self.messages.push(ChatMessage {
            role: "user".into(),
            content: text,
            ..Default::default()
        });
        self.messages.push(ChatMessage {
            role: "assistant".into(),
            content: String::new(),
            ..Default::default()
        });
        self.generating = true;
        self.follow = true;
        self.status = format!("Generating with {}…", self.model);

        // History sent to the model: prior user/assistant/tool turns plus
        // any compaction summary translated to a `system` role. The
        // trailing empty assistant placeholder is dropped.
        let history: Vec<ChatMessage> = self.messages[..self.messages.len() - 1]
            .iter()
            .filter(|m| matches!(m.role.as_str(), "user" | "assistant" | "tool" | "summary"))
            .map(|m| {
                if m.role == "summary" {
                    ChatMessage {
                        role: "system".into(),
                        content: format!("(Compacted summary of earlier turns:)\n\n{}", m.content),
                        ..m.clone()
                    }
                } else {
                    m.clone()
                }
            })
            .collect();

        let Some(backend) = self.make_backend() else {
            self.generating = false;
            self.status = format!("No API key configured for model {}", self.model);
            return;
        };
        let model = self.model.clone();
        let workspace = self.workspace.clone();
        let tx = tx.clone();
        let handle = tokio::spawn(async move {
            crate::agent::agent_loop(backend, model, history, workspace, tx).await;
        });
        self.current_task = Some(handle);
    }

    pub(in crate::app) fn cancel(&mut self) {
        if let Some(h) = self.current_task.take() {
            h.abort();
        }
        if let Some(h) = self.compact_task.take() {
            h.abort();
            self.compacting = false;
            self.pending_after_compact = None;
        }
        self.persist_assistant_if_any();
        self.generating = false;
        self.active_tool_msg_idx = None;
        self.status = "Cancelled".into();
    }

    /// Kick off an asynchronous compaction. Sends the current visible
    /// history (minus hidden / info / system entries) to the active model
    /// with a summarization system prompt. The reply lands as
    /// `StreamMsg::CompactionDone`, where `app::stream` replaces the
    /// visible history with a `summary`-role message and, if a pending
    /// user message was buffered via `pending_after_compact`, re-issues it.
    pub(super) fn start_compact(
        &mut self,
        tx: &mpsc::UnboundedSender<StreamMsg>,
        pending_user_message: Option<String>,
    ) {
        if self.compacting {
            self.push_info("A compaction is already running.".into());
            return;
        }
        if self.generating {
            self.push_info("Wait for the current turn to finish, then /compact.".into());
            return;
        }
        let to_compact_count = self
            .messages
            .iter()
            .filter(|m| !m.hidden && matches!(m.role.as_str(), "user" | "assistant" | "tool"))
            .count();
        if to_compact_count < 2 {
            self.push_info("Nothing meaningful to compact yet.".into());
            return;
        }
        let Some(backend) = self.make_backend() else {
            self.push_info(format!(
                "Can't compact — no backend configured for model {}.",
                self.model
            ));
            return;
        };

        // Snapshot the visible history for the task. Hidden user messages
        // (Y/N injections) are dropped — they're not real conversation
        // turns the summary should preserve.
        let snapshot: Vec<ChatMessage> = self
            .messages
            .iter()
            .filter(|m| !m.hidden)
            .cloned()
            .collect();
        let model = self.model.clone();
        let tx2 = tx.clone();
        self.compacting = true;
        self.pending_after_compact = pending_user_message;
        self.status = "Compacting conversation…".into();
        self.follow = true;
        self.push_info("/compact — summarising prior turns into a single context briefing.".into());

        let handle = tokio::spawn(async move {
            match crate::compact::compact_history(&backend, &model, snapshot).await {
                Ok((summary, prompt_tokens, completion_tokens)) => {
                    let _ = tx2.send(StreamMsg::CompactionDone {
                        summary,
                        prompt_tokens,
                        completion_tokens,
                    });
                }
                Err(e) => {
                    let _ = tx2.send(StreamMsg::CompactionError(e.to_string()));
                }
            }
        });
        self.compact_task = Some(handle);
    }
}
