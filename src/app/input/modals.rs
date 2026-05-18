//! Key handlers for modes that briefly take over the screen: the model
//! picker (Ctrl+M), the confirm dialog (Y/N + diff scroll), the session
//! picker (/sessions), and the in-chat file viewer overlay.
//!
//! Each handler is registered in `event.rs::handle_event` and called
//! while its corresponding `Mode::*` is active. They share no state
//! beyond what's on `App`; placing them together keeps the keymap for
//! "what does Enter mean right now" co-located.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc;

use super::super::commands::model::persist_last_model;
use super::super::{App, AppAction, Mode, PickerEntry, StreamMsg};

impl App {
    /// Key routing while the file viewer is open. Esc dismisses; arrow /
    /// page / home / end keys move through the file. Everything else is
    /// swallowed so the chat input doesn't pick up stray characters and the
    /// user can't accidentally fire a command (e.g. Ctrl+N) while reading.
    pub(in crate::app) fn handle_viewer_key(&mut self, key: KeyEvent) -> AppAction {
        let Some(file) = self.open_file.as_mut() else {
            return AppAction::Continue;
        };
        match key.code {
            KeyCode::Esc => {
                self.open_file = None;
            }
            KeyCode::PageDown | KeyCode::Char(' ') => {
                file.scroll = file.scroll.saturating_add(10);
            }
            KeyCode::PageUp => {
                file.scroll = file.scroll.saturating_sub(10);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                file.scroll = file.scroll.saturating_add(1);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                file.scroll = file.scroll.saturating_sub(1);
            }
            KeyCode::Home | KeyCode::Char('g') => {
                file.scroll = 0;
            }
            KeyCode::End | KeyCode::Char('G') => {
                file.scroll = u16::MAX;
            }
            // Ctrl+C remains an escape hatch so the user can always close.
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.open_file = None;
            }
            _ => {}
        }
        AppAction::Continue
    }

    pub(in crate::app) fn handle_session_picker(
        &mut self,
        key: KeyEvent,
        tx: &mpsc::UnboundedSender<StreamMsg>,
    ) -> AppAction {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Chat;
                self.status = "Cancelled".into();
            }
            KeyCode::Up | KeyCode::Char('k') if self.session_picker_index > 0 => {
                self.session_picker_index -= 1;
            }
            KeyCode::Down | KeyCode::Char('j')
                if self.session_picker_index + 1 < self.session_picker_items.len() =>
            {
                self.session_picker_index += 1;
            }
            KeyCode::Enter => {
                if let Some(s) = self
                    .session_picker_items
                    .get(self.session_picker_index)
                    .cloned()
                {
                    self.mode = Mode::Chat;
                    // Reuse the existing load-by-prefix path: just pass the
                    // full id so load_session resolves it cleanly.
                    self.load_session(s.id, tx);
                }
            }
            _ => {}
        }
        AppAction::Continue
    }

    pub(in crate::app) fn handle_picker(&mut self, key: KeyEvent) -> AppAction {
        match key.code {
            KeyCode::Esc => self.mode = Mode::Chat,
            KeyCode::Up | KeyCode::Char('k') if self.picker_index > 0 => {
                self.picker_index -= 1;
            }
            KeyCode::Down | KeyCode::Char('j')
                if self.picker_index + 1 < self.picker_entries.len() =>
            {
                self.picker_index += 1;
            }
            KeyCode::Enter => {
                if let Some(entry) = self.picker_entries.get(self.picker_index).cloned() {
                    match entry {
                        PickerEntry::Ollama(name) => {
                            self.model = name.clone();
                            self.selected_extra = None;
                            self.status = format!("Switched to {}", name);
                            self.mode = Mode::Chat;
                            let _ = persist_last_model(&self.model, None);
                        }
                        PickerEntry::Extra(m) => {
                            self.model = m.name.clone();
                            self.status = format!("Switched to [{}] {}", m.provider, m.name);
                            let provider = m.provider.clone();
                            self.selected_extra = Some(m);
                            self.mode = Mode::Chat;
                            let _ = persist_last_model(&self.model, Some(&provider));
                        }
                        PickerEntry::AddZaiSubscription => {
                            self.begin_add_model(crate::config::ZAI_SUBSCRIPTION_PROVIDER);
                        }
                        PickerEntry::AddZaiUsage => {
                            self.begin_add_model(crate::config::ZAI_USAGE_PROVIDER);
                        }
                        PickerEntry::AddOllamaCloud => {
                            self.begin_add_model(crate::config::OLLAMA_CLOUD_PROVIDER);
                        }
                        PickerEntry::AddOpenCode => {
                            self.begin_add_model(crate::config::OPENCODE_PROVIDER);
                        }
                    }
                } else {
                    self.mode = Mode::Chat;
                }
            }
            _ => {}
        }
        AppAction::Continue
    }

    pub(in crate::app) fn handle_confirm(&mut self, key: KeyEvent) -> AppAction {
        match key.code {
            // Scroll the diff body. ↑↓ for fine-grained, PgUp/PgDn for
            // a page at a time. The renderer clamps to a valid max so
            // saturating_add never runs past the end visibly.
            KeyCode::Up => {
                self.confirm_scroll = self.confirm_scroll.saturating_sub(1);
                return AppAction::Continue;
            }
            KeyCode::Down => {
                self.confirm_scroll = self.confirm_scroll.saturating_add(1);
                return AppAction::Continue;
            }
            KeyCode::PageUp => {
                self.confirm_scroll = self.confirm_scroll.saturating_sub(10);
                return AppAction::Continue;
            }
            KeyCode::PageDown => {
                self.confirm_scroll = self.confirm_scroll.saturating_add(10);
                return AppAction::Continue;
            }
            KeyCode::Home => {
                self.confirm_scroll = 0;
                return AppAction::Continue;
            }
            KeyCode::End => {
                self.confirm_scroll = u16::MAX;
                return AppAction::Continue;
            }
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                if let Some(req) = self.pending_confirm.take() {
                    // Attach the authorised diff DIRECTLY to the running
                    // tool placeholder right now — `active_tool_msg_idx`
                    // already points at it. Attaching here means the diff
                    // is in place before the file even gets written, so
                    // click-to-expand on the finished tool row always
                    // shows the diff. Empty-diff tools (run_command etc.)
                    // skip the attach.
                    if !req.diff.is_empty() {
                        if let Some(idx) = self.active_tool_msg_idx {
                            if let Some(msg) = self.messages.get_mut(idx) {
                                msg.diff = Some(req.diff.clone());
                            }
                        }
                    }
                    let _ = req.responder.send(true);
                    self.push_info(format!("✓ Allowed: {}", req.prompt));
                }
                self.mode = Mode::Chat;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                if let Some(req) = self.pending_confirm.take() {
                    let _ = req.responder.send(false);
                    self.push_info(format!("✗ Denied: {}", req.prompt));
                }
                self.mode = Mode::Chat;
            }
            _ => {}
        }
        AppAction::Continue
    }
}
