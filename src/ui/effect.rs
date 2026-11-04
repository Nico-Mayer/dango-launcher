use crate::extension::FormValues;

use super::state::ToastStyle;

/// Work the shell does on the launcher's behalf. What comes of it returns as an
/// `Event`, so the launcher itself never waits.
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    Search(String),
    RunAction(ActionRequest),
    InvokeCommand {
        command_id: String,
        query: String,
    },
    CancelInvocation,
    Dismiss,
    /// Hide and hold the state, as the hotkey does.
    HideAndHold,
    CopyText(String),
    InspectTemplate {
        field: String,
        source: String,
    },
    ReportPaint(u64),
    /// Answered with `Event::ToastExpired` once a success toast's or a HUD's
    /// time is up. The style decides how long that is.
    ExpireToast {
        id: u64,
        style: ToastStyle,
    },
    /// Answered with `Event::BusyMarksWoke` once the stage's time is up.
    WakeBusyMarks {
        id: u64,
        stage: BusyStage,
    },
}

/// How long the busy marks wait: before they appear, or before they may go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BusyStage {
    Delay,
    Hold,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActionRequest {
    pub origin: Origin,
    pub extension_id: String,
    pub item_id: String,
    pub action_id: String,
    pub values: Option<FormValues>,
    /// What was typed to reach the item, so the shell can learn it. Only a
    /// root result has one.
    pub query: Option<String>,
}

/// Where an action was chosen, which decides what a view it answers with
/// does: from root it opens, inside a view it replaces that view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    Root,
    View,
}
