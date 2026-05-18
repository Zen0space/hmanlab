//! Slash command parsing.
//!
//! The actual command *implementations* (switch_model, switch_workspace, …)
//! live as methods on `App` in `event.rs` and will move into per-domain
//! files under this directory in subsequent refactor steps. For now this
//! module owns:
//!   - the `Command` enum (the parser's output type)
//!   - `parse_command` (text → `Option<Command>`)
//!
//! The dispatcher (`App::handle_command`) stays in `event.rs` until the
//! command impls are colocated here.

mod disconnect;
mod help;
mod host;
pub(super) mod model;
mod session;
mod settings;

/// One typed slash command. Output of [`parse_command`]; consumed by
/// `App::handle_command` in `event.rs`.
pub(super) enum Command {
    Model(Option<String>),
    ListModels,
    Clear,
    Quit,
    Help,
    Host(String),
    New,
    ListSessions,
    Load(String),
    More,
    Workspace(String),
    Compact,
    Disconnect(String),
    Update,
    Settings,
    Trust,
    Untrust,
    Unknown(String),
}

/// Parse a textarea line into a [`Command`], or `None` if the line isn't
/// a slash command at all. Recognises every alias the user might type
/// (e.g. `/q` / `/quit` / `/exit` / `/bye` all → `Command::Quit`); the
/// canonical names are surfaced in the autocomplete via `SLASH_COMMANDS`
/// in `app::inline`.
pub(super) fn parse_command(text: &str) -> Option<Command> {
    let t = text.trim();
    if !t.starts_with('/') {
        return None;
    }
    let body = &t[1..];
    let (head, rest) = match body.split_once(char::is_whitespace) {
        Some((h, r)) => (h.to_ascii_lowercase(), r.trim().to_string()),
        None => (body.to_ascii_lowercase(), String::new()),
    };
    Some(match head.as_str() {
        "model" | "m" => Command::Model(if rest.is_empty() { None } else { Some(rest) }),
        "models" | "ls" => Command::ListModels,
        "clear" | "cls" | "reset" => Command::Clear,
        "quit" | "exit" | "q" | "bye" => Command::Quit,
        "help" | "?" | "h" => Command::Help,
        "host" | "connect" => Command::Host(rest),
        "new" | "n" => Command::New,
        "sessions" | "history" | "hist" => Command::ListSessions,
        "load" | "open" => Command::Load(rest),
        "more" | "older" => Command::More,
        "workspace" | "ws" | "cwd" => Command::Workspace(rest),
        "compact" | "compress" | "summarize" => Command::Compact,
        "disconnect" | "logout" | "signout" => Command::Disconnect(rest),
        "update" | "upgrade" | "selfupdate" => Command::Update,
        "settings" | "whoami" | "account" | "me" => Command::Settings,
        "trust" | "authorize" | "authorise" => Command::Trust,
        "untrust" | "unauthorize" | "unauthorise" => Command::Untrust,
        other => Command::Unknown(other.to_string()),
    })
}
