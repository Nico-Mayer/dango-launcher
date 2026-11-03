use crate::extension::ActionOutcome;
use crate::protocol::{Action, Modifier, Shortcut, ViewTree};

use super::effect::ActionRequest;

/// Everything the launcher reacts to: what the user did, or what the backend
/// answered.
#[derive(Debug)]
pub enum Input {
    Intent(Intent),
    Event(Event),
}

/// What the user did, in terms of the launcher rather than the DOM. Keys arrive
/// only for a chord the snapshot declared in `bindings`.
#[derive(Clone, Debug, PartialEq)]
pub enum Intent {
    /// The renderer is mounted and listening.
    Ready,
    /// The activation carrying this probe has been painted.
    Painted {
        probe: u64,
    },
    QueryChanged {
        text: String,
    },
    FieldChanged {
        id: String,
        value: String,
    },
    Key {
        chord: Chord,
    },
    /// A click on a row or an action.
    Activate {
        id: String,
    },
    TogglePanel,
    /// A press outside the open action panel.
    ClosePanel,
    /// The SearchBar's back control.
    GoBack,
    /// The action panel's own filter input.
    PanelFilterChanged {
        text: String,
    },
    /// An option of the view's filter dropdown.
    FilterChosen {
        id: String,
    },
}

/// A key with its modifiers. `command` is Cmd on macOS and Ctrl on Windows,
/// and either one counts, as both always have. `key` is lowercase.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chord {
    pub key: String,
    pub command: bool,
    pub alt: bool,
    pub shift: bool,
}

impl Chord {
    pub fn plain(key: &str) -> Self {
        Self {
            key: key.to_lowercase(),
            command: false,
            alt: false,
            shift: false,
        }
    }

    pub fn command(key: &str) -> Self {
        Self {
            command: true,
            ..Self::plain(key)
        }
    }
}

impl From<&Shortcut> for Chord {
    fn from(shortcut: &Shortcut) -> Self {
        let has = |wanted: &[Modifier]| shortcut.modifiers.iter().any(|m| wanted.contains(m));
        Self {
            key: shortcut.key.to_lowercase(),
            command: has(&[Modifier::Cmd, Modifier::Ctrl]),
            alt: has(&[Modifier::Alt]),
            shift: has(&[Modifier::Shift]),
        }
    }
}

/// What the backend reports back.
#[derive(Debug)]
pub enum Event {
    Results {
        query: String,
        items: Vec<ResultItem>,
        /// Every provider has answered, or been given up on, for this query.
        complete: bool,
    },
    Rendered {
        owner: String,
        context: Context,
        invocation: u64,
        tree: Box<ViewTree>,
    },
    ActionFinished {
        request: ActionRequest,
        outcome: ActionOutcome,
    },
    /// A command failed, or refused to start.
    Failed(String),
    /// A hotkey press failed. Unlike `Failed`, a command still running from
    /// the launcher keeps running.
    HotkeyFailed(String),
    TemplateInspected {
        field: String,
        source: String,
        inspection: TemplateInspection,
    },
    Activated {
        held: bool,
        probe: Option<u64>,
    },
    Reset,
    /// The launcher is off screen, for whatever reason it was hidden.
    Hidden,
    ToastExpired {
        id: u64,
    },
    BusyMarksWoke {
        id: u64,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResultItem {
    pub extension_id: String,
    /// The contributing extension, named in the ActionBar while selected.
    pub context: Context,
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub icon: Option<String>,
    pub actions: Vec<Action>,
    pub match_positions: Vec<usize>,
    pub suggested: bool,
    pub pinned: bool,
}

/// A name and icon the ActionBar shows for where the user is.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Context {
    pub title: String,
    pub icon: Option<String>,
}

/// What a template field's text will ask for.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TemplateInspection {
    pub arguments: Vec<String>,
    pub error: Option<String>,
}
