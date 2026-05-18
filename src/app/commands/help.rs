//! `/help` — print the inline command + tool + keys cheat-sheet.
//!
//! Kept as a single hard-coded string rather than generated from
//! `SLASH_COMMANDS` so the help text can group, indent, and annotate
//! freely. Update both when adding a command.

use super::super::App;

impl App {
    pub(in crate::app) fn show_help_inline(&mut self) {
        let help = "Commands:\n\
            \x20 /new                start a fresh session\n\
            \x20 /sessions, /hist    list recent saved sessions\n\
            \x20 /load <id-prefix>   load a saved session (10 most recent messages)\n\
            \x20 /more, /older       load 10 older messages in the current loaded session\n\
            \x20 /model              open model picker\n\
            \x20 /model <name>       switch model (partial match works)\n\
            \x20 /models, /ls        list available models\n\
            \x20 /host <url>         change Ollama host\n\
            \x20 /workspace <path>   change agent workspace\n\
            \x20 /clear              clear visible chat (current session keeps going)\n\
            \x20 /compact            summarise prior turns into a single context briefing\n\
            \x20 /disconnect [name]  drop a BYOK provider key (zai, zai-usage, ollama-cloud, opencode)\n\
            \x20 /settings, /whoami  show account info, version, configured providers\n\
            \x20 /update             update hmanlab to the latest npm release\n\
            \x20 /help, /?           show this help\n\
            \x20 /quit, /exit        quit\n\
            \n\
            Tools (agent uses these on its own — needs a tool-capable model like qwen2.5):\n\
            \x20 read_file, list_dir, find_files, git_status, git_log, git_diff,\n\
            \x20 git_show, run_command (shell — you confirm each call).\n\
            \n\
            Keys:\n\
            \x20 Enter         send  ·  Alt+Enter / Ctrl+J  newline\n\
            \x20 Ctrl+N        new session  ·  Ctrl+T  fold/unfold all tool + thinking blocks\n\
            \x20 Wheel         scroll chat  ·  PgUp/PgDn  Home/End  also scroll\n\
            \x20 Ctrl+C        cancel/quit  ·  Esc  interrupt generation / clear draft\n\
            \n\
            Drag with your mouse to select text — copy with your terminal's normal\n\
            shortcut (Ctrl+Shift+C / Cmd+C). The wheel scrolls the chat in single-line\n\
            input mode; when composing multi-line input (Alt+Enter / Ctrl+J), use PgUp/PgDn.";
        self.push_info(help.to_string());
    }
}
