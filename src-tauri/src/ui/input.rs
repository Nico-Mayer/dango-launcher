use serde::{Deserialize, Serialize};

#[cfg(test)]
use ts_rs::TS;

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
#[cfg_attr(test, derive(TS))]
#[cfg_attr(test, ts(export, export_to = "../../src/protocol/"))]
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Intent {
    /// The renderer is mounted and listening.
    Ready,
    /// The activation carrying this probe has been painted.
    Painted {
        #[cfg_attr(test, ts(type = "number"))]
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
    Blur,
}

/// A key with its modifiers. `command` is Cmd on macOS and Ctrl on Windows,
/// and either one counts, as both always have. `key` is lowercase.
#[cfg_attr(test, derive(TS))]
#[cfg_attr(test, ts(export, export_to = "../../src/protocol/"))]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
    },
    Rendered {
        owner: String,
        invocation: u64,
        tree: ViewTree,
    },
    ActionFinished {
        request: ActionRequest,
        outcome: ActionOutcome,
    },
    /// A command failed, or refused to start.
    Failed(String),
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
}

#[cfg_attr(test, derive(TS))]
#[cfg_attr(test, ts(export, export_to = "../../src/protocol/"))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultItem {
    pub extension_id: String,
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub icon: Option<String>,
    pub tint: Option<String>,
    pub actions: Vec<Action>,
    pub match_positions: Vec<usize>,
    pub suggested: bool,
    pub pinned: bool,
}

/// What a template field's text will ask for.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct TemplateInspection {
    pub arguments: Vec<String>,
    pub error: Option<String>,
}
