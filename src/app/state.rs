//! UI mode + picker-row types. Kept out of `app/mod.rs` so that file
//! stays focused on the `App` struct itself; everything in here is a
//! small, mostly-enum value the input handlers and renderers reach for.

use crate::config::ExtraModel;

/// Returned by event handlers to tell the main loop whether to keep
/// running or shut down cleanly.
#[derive(PartialEq)]
pub enum AppAction {
    Continue,
    Quit,
}

/// Which keymap is active. The dispatcher in `event.rs::handle_event`
/// reads this to route key events to the right handler; the popup
/// renderers in `ui::popups` use it to decide what to draw on top of
/// the chat.
#[derive(Clone, PartialEq)]
pub enum Mode {
    Chat,
    ModelPicker,
    Confirm,
    /// Asking for a BYOK API key (e.g. z.ai). Use `add_step` to track which
    /// step of the add-model flow we're on.
    AddModel,
    /// Listing saved chat sessions; Up/Down navigate, Enter loads the
    /// highlighted session.
    SessionPicker,
    /// Listing currently-connected BYOK providers for removal; Up/Down
    /// navigate, Enter disconnects the highlighted provider, Esc cancels.
    DisconnectPicker,
}

/// AddModel is a single-step flow now (key entry only). The model list per
/// provider is hardcoded — see `config::ZAI_MODELS`.
#[derive(Clone, Copy, PartialEq)]
pub enum AddModelStep {
    Key,
}

/// One row in the `/disconnect` picker — a currently-connected BYOK
/// provider plus a short preview of the models that will be removed
/// alongside its API key.
#[derive(Clone)]
pub struct DisconnectEntry {
    /// Provider identifier (e.g. `"zai-subscription"`).
    pub provider: String,
    /// Pretty label shown in the popup (e.g. `"z.ai subscription"`).
    pub label: String,
    /// Three-or-fewer model names + a "+N more" suffix when the provider
    /// seeds a longer catalog. Lets the user see what they're about to
    /// drop before pressing Enter.
    pub preview: String,
}

/// What the `/model` picker can display. The picker mixes Ollama-discovered
/// models with BYOK extras and trailing "Add …" action rows (one per
/// unconfigured provider).
#[derive(Clone)]
pub enum PickerEntry {
    Ollama(String),
    Extra(ExtraModel),
    /// "+ Add z.ai (subscription) key" — appears only if the subscription
    /// key isn't already configured.
    AddZaiSubscription,
    /// "+ Add z.ai (usage-based) key" — appears only if the usage key isn't
    /// already configured.
    AddZaiUsage,
    /// "+ Add Ollama Cloud key" — appears only if the cloud key isn't set.
    AddOllamaCloud,
    /// "+ Add OpenCode key" — appears only if the OpenCode Zen / Go key
    /// isn't already configured.
    AddOpenCode,
    /// "+ Add OpenRouter key" — appears only if the OpenRouter key isn't
    /// already configured.
    AddOpenRouter,
}

impl PickerEntry {
    pub fn display(&self) -> String {
        match self {
            PickerEntry::Ollama(name) => name.clone(),
            PickerEntry::Extra(m) => format!("[{}] {}", m.provider, m.name),
            PickerEntry::AddZaiSubscription => "+ Add z.ai (subscription) key".to_string(),
            PickerEntry::AddZaiUsage => "+ Add z.ai (usage-based) key".to_string(),
            PickerEntry::AddOllamaCloud => "+ Add Ollama Cloud key".to_string(),
            PickerEntry::AddOpenCode => "+ Add OpenCode Go key".to_string(),
            PickerEntry::AddOpenRouter => "+ Add OpenRouter key".to_string(),
        }
    }
}
