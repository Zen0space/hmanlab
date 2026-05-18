//! Session lifecycle commands: `/new`, `/clear`, `/sessions`, `/load`, `/more`.
//!
//! Two flavours of "wipe and start over":
//! - **`/clear`** drops the visible history but stays on the same DB
//!   session — token tallies reset, but the next message you send still
//!   appends to whatever session you were in. Useful for context resets
//!   without losing the audit trail.
//! - **`/new`** ends the current DB session entirely and starts a fresh
//!   one. The next message opens a brand-new row in the sessions table.
//!
//! `/load` + `/more` are the read side: they hit the persistence API to
//! pull a saved session back into the visible history. Both fire
//! background `tokio::spawn` tasks and report back via `StreamMsg`.

use tokio::sync::mpsc;

use crate::api::ApiOp;

use super::super::{App, StreamMsg};

impl App {
    /// `/clear` — drop visible history, keep the underlying DB session.
    /// Resets token tallies + scroll but doesn't touch `loaded_session_id`,
    /// so the next message you send still appends to the same session.
    pub(in crate::app) fn clear_history(&mut self) {
        self.messages.clear();
        self.expanded_tools.clear();
        self.expanded_thoughts.clear();
        self.total_prompt_tokens = 0;
        self.total_completion_tokens = 0;
        self.last_prompt_tokens = 0;
        self.pending_after_compact = None;
        self.scroll = 0;
        self.follow = true;
        self.status = "History cleared (current session continues)".into();
    }

    /// `/new` — end the current DB session and start fresh. Notifies the
    /// persistence writer so it rolls over to a new row on the next
    /// message instead of appending to the old one.
    pub(in crate::app) fn new_session(&mut self) {
        self.messages.clear();
        self.expanded_tools.clear();
        self.expanded_thoughts.clear();
        self.total_prompt_tokens = 0;
        self.total_completion_tokens = 0;
        self.last_prompt_tokens = 0;
        self.pending_after_compact = None;
        self.scroll = 0;
        self.follow = true;
        self.loaded_session_id = None;
        self.oldest_loaded_msg_id = None;
        if let Some(tx) = &self.api_tx {
            let _ = tx.send(ApiOp::EndSession);
            self.push_info("New session started. Previous chat saved.".into());
        } else {
            self.push_info("New session started (not persisted — API off).".into());
        }
        self.status = "New session".into();
    }

    /// `/sessions` — fire off a background fetch for the 20 most-recent
    /// sessions. The stream handler turns the response into a picker
    /// popup; this method just kicks the request.
    pub(in crate::app) fn list_sessions_inline(&mut self, tx: &mpsc::UnboundedSender<StreamMsg>) {
        let Some(client) = self.api.clone() else {
            self.push_info(
                "API is off — set HMANLAB_API_KEY or pass --api-key to enable persistence.".into(),
            );
            return;
        };
        let tx = tx.clone();
        tokio::spawn(async move {
            match client.list_sessions(20).await {
                Ok(rows) => {
                    let _ = tx.send(StreamMsg::SessionList(rows));
                }
                Err(e) => {
                    let _ = tx.send(StreamMsg::Error(format!("list sessions: {e}")));
                }
            }
        });
    }

    /// `/load <id-prefix>` — find a session by the first few chars of its
    /// id and pull its 10 most-recent messages into the chat. Two-step
    /// async because the API needs the resolved id before it can load
    /// messages — we collapse both into one StreamMsg::Loaded so the UI
    /// only re-renders once.
    pub(in crate::app) fn load_session(
        &mut self,
        prefix: String,
        tx: &mpsc::UnboundedSender<StreamMsg>,
    ) {
        let Some(client) = self.api.clone() else {
            self.push_info("API is off — cannot load saved sessions.".into());
            return;
        };
        if prefix.trim().is_empty() {
            self.push_info("Usage: /load <id-prefix>  (run /sessions for the list)".into());
            return;
        }
        let tx = tx.clone();
        tokio::spawn(async move {
            let session = match client.find_session_by_prefix(&prefix).await {
                Ok(s) => s,
                Err(e) => {
                    let _ = tx.send(StreamMsg::Error(format!("load: {e}")));
                    return;
                }
            };
            match client.load_recent_messages(&session.id, 10).await {
                Ok(messages) => {
                    let _ = tx.send(StreamMsg::Loaded { session, messages });
                }
                Err(e) => {
                    let _ = tx.send(StreamMsg::Error(format!("load messages: {e}")));
                }
            }
        });
    }

    /// `/more` — page 10 older messages into a previously-loaded session.
    /// Only meaningful after `/load`; otherwise nudges the user with
    /// usage info instead of silently no-op'ing.
    pub(in crate::app) fn load_more(&mut self, tx: &mpsc::UnboundedSender<StreamMsg>) {
        let Some(client) = self.api.clone() else {
            self.push_info("API is off — nothing to page.".into());
            return;
        };
        let Some(session_id) = self.loaded_session_id.clone() else {
            self.push_info(
                "/more works inside a /load'd session. Run /sessions and /load <id> first.".into(),
            );
            return;
        };
        let Some(before_id) = self.oldest_loaded_msg_id else {
            self.push_info("No more messages to load.".into());
            return;
        };
        self.status = "Loading older messages…".into();
        let tx = tx.clone();
        tokio::spawn(async move {
            match client.load_older_messages(&session_id, before_id, 10).await {
                Ok(messages) => {
                    let _ = tx.send(StreamMsg::MoreLoaded { messages });
                }
                Err(e) => {
                    let _ = tx.send(StreamMsg::Error(format!("/more: {e}")));
                }
            }
        });
    }
}
