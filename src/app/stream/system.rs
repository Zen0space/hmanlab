//! System-level stream handlers: host change (new model list), `/update`
//! result, `/settings` edit-in-place.

use super::super::App;

impl App {
    pub(super) fn on_models(&mut self, models: Vec<String>, base: String) {
        let n = models.len();
        self.models = models;
        if !self.models.iter().any(|m| m == &self.model) {
            if let Some(first) = self.models.first() {
                self.model = first.clone();
            }
        }
        self.status = format!("Connected to {base} — {n} model(s)");
        self.push_info(format!(
            "Connected to {base}\nModels available: {n}\nCurrent: {}",
            self.model
        ));
        // /host succeeded — remember the URL across restarts so the
        // user doesn't have to re-add Ollama every session.
        let mut cfg = crate::config::load().ok().flatten().unwrap_or_default();
        cfg.ollama_host = Some(base);
        let _ = crate::config::save(&cfg);
    }

    pub(super) fn on_update_result(&mut self, ok: bool, text: String) {
        self.push_info(text);
        self.status = if ok {
            "Update finished — restart hmanlab to use the new version".into()
        } else {
            "Update failed".into()
        };
        if ok {
            // Clear the header notice: whatever version was advertised
            // has now been installed, even if we can't detect the new
            // version until the user restarts.
            self.update_available = None;
        }
    }

    pub(super) fn on_settings(&mut self, text: String) {
        // Edit-in-place: if the placeholder card is still where we
        // left it (an `info` message at the stashed index), overwrite
        // its content instead of appending. Falls back to push_info
        // if the index drifted (e.g. /clear or another /settings
        // raced ours).
        match self.pending_settings_msg_idx.take() {
            Some(idx)
                if self
                    .messages
                    .get(idx)
                    .map(|m| m.role == "info")
                    .unwrap_or(false) =>
            {
                self.messages[idx].content = text;
                self.follow = true;
            }
            _ => self.push_info(text),
        }
        self.status = "Settings loaded".into();
    }
}
