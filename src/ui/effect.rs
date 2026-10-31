use crate::extension::FormValues;

/// Work the shell does on the launcher's behalf. What comes of it returns as an
/// `Event`, so the launcher itself never waits.
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    Search(String),
    RunAction(ActionRequest),
    InvokeCommand { command_id: String, query: String },
    CancelInvocation,
    Dismiss,
    CopyText(String),
    InspectTemplate { field: String, source: String },
    FinishWarmup,
    ReportPaint(u64),
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
