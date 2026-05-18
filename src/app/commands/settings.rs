//! `/settings` and `/update`.
//!
//! Both commands kick a background task and edit-in-place (or append) when
//! the result lands:
//!
//! - **`/settings`** drops a placeholder card synchronously (so the user
//!   sees their local config right away), stashes its index in
//!   `pending_settings_msg_idx`, then resolves the account block from
//!   the auth API + the latest npm version. The stream handler in
//!   `stream::StreamMsg::Settings` overwrites the placeholder so the
//!   refresh looks atomic.
//! - **`/update`** is a self-upgrade via `npm install -g hmanlab@latest`,
//!   guarded by a cargo-install detector — if the binary lives under
//!   `.cargo/bin` or a `target/` build dir we surface the cargo upgrade
//!   command instead of stomping on it with npm.

use tokio::sync::mpsc;

use super::super::{App, StreamMsg};

impl App {
    /// `/settings` — show what the user has set: hmanlab version, active
    /// model, Ollama host, configured BYOK providers (presence only,
    /// never the key), workspace, plus the authenticated user's profile.
    /// The profile + latest-version look-up run in the background — the
    /// prompt returns instantly with the locally-known fields and the
    /// account block fills in when the request resolves.
    ///
    /// Backend URL / "where this came from" is intentionally not shown —
    /// users care about their account and configuration, not plumbing.
    pub(in crate::app) fn show_settings(&mut self, tx: &mpsc::UnboundedSender<StreamMsg>) {
        let current = env!("CARGO_PKG_VERSION");
        let mut byok = Vec::new();
        if self.zai_api_key.is_some() {
            byok.push("z.ai (subscription)");
        }
        if self.zai_usage_api_key.is_some() {
            byok.push("z.ai (usage)");
        }
        if self.ollama_cloud_api_key.is_some() {
            byok.push("Ollama Cloud");
        }
        if self.opencode_api_key.is_some() {
            byok.push("OpenCode");
        }
        let byok_line = if byok.is_empty() {
            "none".to_string()
        } else {
            byok.join(", ")
        };
        let upstream = self.update_available.as_deref();
        let version_line = match upstream {
            Some(latest) if crate::update_check::newer(current, latest) => {
                format!("{current}  (npm has {latest} — run /update)")
            }
            _ => current.to_string(),
        };
        // Shared header — used verbatim both for the placeholder card
        // (rendered synchronously) and for the resolved card the spawn
        // sends back. Keeping the local block identical means the
        // edit-in-place looks like a true refresh.
        let local = format!(
            "Settings\n\
             \x20 hmanlab version  : {version_line}\n\
             \x20 model            : {model}\n\
             \x20 ollama host      : {host}\n\
             \x20 BYOK providers   : {byok_line}\n\
             \x20 workspace        : {ws}",
            model = self.model,
            host = self.client.base,
            ws = self.workspace.display(),
        );
        self.push_info(format!("{local}\n\nAccount: loading…"));
        // Stash the placeholder card's index so the resolved reply can
        // overwrite it in place (see stream::StreamMsg::Settings).
        self.pending_settings_msg_idx = Some(self.messages.len().saturating_sub(1));
        self.status = "Loading account info…".into();

        let Some(api) = self.api.clone() else {
            // No auth client → nothing to fetch. The placeholder above is
            // all we'll have; drop the pending index so a later /settings
            // call doesn't try to edit it.
            self.pending_settings_msg_idx = None;
            return;
        };
        let current_owned = current.to_string();
        let local_owned = local;
        let tx = tx.clone();
        tokio::spawn(async move {
            let me = api.fetch_me().await;
            let latest = crate::update_check::fetch_latest_npm().await.ok();
            let account = match me {
                Ok(me) => {
                    let name = me.name.as_deref().unwrap_or("(no display name set)");
                    let admin = if me.is_admin { " · admin" } else { "" };
                    let opt = if me.training_opt_in {
                        "opted in"
                    } else {
                        "opted out"
                    };
                    format!(
                        "Account\n\
                         \x20 name             : {name}{admin}\n\
                         \x20 email            : {email}\n\
                         \x20 training data    : {opt}",
                        email = me.email,
                    )
                }
                Err(_) => "Account\n\x20 (could not load — try /settings again later)".to_string(),
            };
            let version_tail = match latest {
                Some(l) if crate::update_check::newer(&current_owned, &l) => {
                    format!("\n\nnpm latest: {l} — run /update to install.")
                }
                Some(l) => format!("\n\nnpm latest: {l} (you're up to date)."),
                None => String::new(),
            };
            // Send the full resolved card; the handler decides whether
            // to edit-in-place (pending_settings_msg_idx still set) or
            // append (e.g. user re-ran /settings in the meantime).
            let _ = tx.send(StreamMsg::Settings(format!(
                "{local_owned}\n\n{account}{version_tail}"
            )));
        });
    }

    /// `/update` — shell out to `npm install -g hmanlab@latest` in the
    /// background and report the outcome inline. The currently running
    /// process keeps serving the chat; npm replaces the on-disk binary,
    /// and the user picks it up on next launch.
    ///
    /// If the binary was installed via cargo (path under `.cargo/bin` or
    /// a `target/` build dir), we don't even try npm — surface the right
    /// `cargo install` command instead so the user upgrades through the
    /// channel they actually used.
    pub(in crate::app) fn start_update(&mut self, tx: &mpsc::UnboundedSender<StreamMsg>) {
        let current = env!("CARGO_PKG_VERSION");

        if let Some(hint) = cargo_install_hint() {
            self.push_info(format!(
                "hmanlab looks like a cargo install ({hint}).\n\
                 Run this in another terminal to upgrade:\n\
                 \x20 cargo install hmanlab --force"
            ));
            self.status = "Cargo install detected — see message".into();
            return;
        }

        self.push_info(format!(
            "Checking npm for a newer hmanlab (current {current})…"
        ));
        self.status = "Checking latest version…".into();

        let tx = tx.clone();
        let current_owned = current.to_string();
        tokio::spawn(async move {
            // Step 1: ask npm what's published. If the lookup fails we still
            // proceed to install — the user explicitly asked, and a flaky
            // registry shouldn't block them. If it succeeds and the current
            // version is already latest, bail out without spawning npm.
            match crate::update_check::fetch_latest_npm().await {
                Ok(latest) if !crate::update_check::newer(&current_owned, &latest) => {
                    let _ = tx.send(StreamMsg::UpdateResult {
                        ok: true,
                        text: format!(
                            "Already up to date — hmanlab {current_owned} matches the latest \
                             on npm ({latest}). No install needed."
                        ),
                    });
                    return;
                }
                Ok(latest) => {
                    let _ = tx.send(StreamMsg::UpdateInfo(format!(
                        "Update available: {current_owned} → {latest}. \
                         Running: npm install -g hmanlab@latest"
                    )));
                }
                Err(e) => {
                    let _ = tx.send(StreamMsg::UpdateInfo(format!(
                        "Couldn't reach npm registry ({e}). Trying install anyway…"
                    )));
                }
            }

            let result = tokio::process::Command::new("npm")
                .args(["install", "-g", "hmanlab@latest"])
                .output()
                .await;
            let msg = match result {
                Ok(out) if out.status.success() => StreamMsg::UpdateResult {
                    ok: true,
                    text: "Update complete. Restart hmanlab to use the new version.".into(),
                },
                Ok(out) => {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    let tail = stderr.lines().rev().take(8).collect::<Vec<_>>();
                    let tail = tail.into_iter().rev().collect::<Vec<_>>().join("\n");
                    StreamMsg::UpdateResult {
                        ok: false,
                        text: format!(
                            "npm install failed (exit {}).\n{}",
                            out.status.code().unwrap_or(-1),
                            if tail.is_empty() {
                                "No stderr output.".into()
                            } else {
                                tail
                            }
                        ),
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => StreamMsg::UpdateResult {
                    ok: false,
                    text: "Couldn't run `npm` — it's not on PATH.\n\
                           Install Node.js (https://nodejs.org) and try again, or grab a\n\
                           prebuilt binary from https://github.com/rekabytes/hmanlab/releases."
                        .into(),
                },
                Err(e) => StreamMsg::UpdateResult {
                    ok: false,
                    text: format!("Failed to launch npm: {e}"),
                },
            };
            let _ = tx.send(msg);
        });
    }
}

/// If the current binary's path looks like a cargo-managed install,
/// return a short identifier (the matched path fragment) so `/update`
/// can suggest the right upgrade channel instead of running npm.
fn cargo_install_hint() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let s = exe.to_string_lossy().to_string();
    // `.cargo/bin/hmanlab` covers `cargo install`; `target/release` and
    // `target/debug` cover devs running from a local checkout.
    for needle in [".cargo/bin", "target/release", "target/debug"] {
        if s.contains(needle) {
            return Some(needle.to_string());
        }
    }
    None
}
