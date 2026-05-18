//! Tool-execution stream handlers: appending the placeholder when a tool
//! starts, replacing it with the output when the tool returns, and the
//! confirm-popup intercept (with the workspace-trust short-circuit).

use crate::api::ApiOp;
use crate::ollama::ChatMessage;
use crate::tools;

use super::super::{App, Mode};

impl App {
    pub(super) fn on_tool_start(&mut self, name: String, args: serde_json::Value) {
        let args_str = serde_json::to_string(&args).unwrap_or_else(|_| "{}".into());
        self.messages.push(ChatMessage {
            role: "tool".into(),
            name: Some(name),
            content: format!("(running… args: {args_str})"),
            ..Default::default()
        });
        self.active_tool_msg_idx = Some(self.messages.len() - 1);
        self.follow = true;
    }

    pub(super) fn on_tool_result(&mut self, output: String) {
        // Walk backwards to find the most recent tool placeholder.
        // We can't just look at `last()` because confirmed tools
        // (run_command / edit_file / write_file) sit through the
        // user's y/n decision — the handler for that decision calls
        // `push_info(...)` which appends a system message between the
        // tool placeholder and the eventual ToolResult. Trusting
        // `last_mut()` silently drops the tool result on the floor
        // (and from the DB, which breaks training data for any tool
        // that requires confirmation).
        let mut to_persist: Option<(String, String)> = None;
        for msg in self.messages.iter_mut().rev() {
            if msg.role == "tool" {
                msg.content = output.clone();
                // NOTE: msg.diff is set earlier by handle_confirm-Y
                // (attached to active_tool_msg_idx the moment the
                // user approves). We DON'T overwrite it here —
                // doing so would clobber the diff with None for
                // tools that didn't go through confirm.
                if let Some(n) = msg.name.clone() {
                    to_persist = Some((n, output));
                }
                break;
            }
        }
        if let (Some((name, output)), Some(api_tx)) = (to_persist, self.api_tx.as_ref()) {
            let _ = api_tx.send(ApiOp::ToolResult { name, output });
        }
        self.active_tool_msg_idx = None;
    }

    pub(super) fn on_confirm_request(&mut self, req: tools::ConfirmRequest) {
        if !self.workspace_trusted {
            // Trust gate: short-circuit before showing the popup.
            // Sending `false` through the oneshot makes the tool
            // return "user denied" to the agent loop, which then
            // surfaces it as a normal tool error in the chat.
            let _ = req.responder.send(false);
            self.push_info(format!(
                "Blocked: {}\nWorkspace not trusted. Run /trust to authorise this folder, \
                 or /workspace <path> to switch.",
                req.prompt
            ));
            self.status = "Blocked — workspace not trusted".into();
        } else {
            self.pending_confirm = Some(req);
            self.mode = Mode::Confirm;
            // Fresh prompt → start at the top. Without this, a long
            // first diff scrolled to its bottom would still be
            // scrolled when the next, possibly-short prompt opens.
            self.confirm_scroll = 0;
            self.status = "Confirmation needed — y/n".into();
        }
    }
}
