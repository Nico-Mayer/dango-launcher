//! The view protocol: the versioned, declarative description of what a command
//! puts on screen.

use serde::{Deserialize, Serialize};

/// Bumped when the protocol changes shape. Dango refuses a tree whose
/// version it does not recognise.
pub const PROTOCOL_VERSION: u32 = 2;

macro_rules! protocol_types {
    ($($item:item)*) => {
        $(
            #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
            $item
        )*
    };
}

protocol_types! {
    #[serde(rename_all = "camelCase")]
    pub struct ViewTree {
        pub protocol_version: u32,
        pub view: View,
    }

    #[serde(tag = "kind", rename_all = "camelCase")]
    pub enum View {
        List(ListView),
        Detail(DetailView),
        Form(FormView),
    }

    #[derive(Default)]
    #[serde(rename_all = "camelCase")]
    pub struct ListView {
        pub filtering: Filtering,
        #[serde(default)]
        pub loading: bool,
        #[serde(default)]
        pub empty_state: Option<EmptyState>,
        /// In the order they are shown. Items name the one they belong to.
        #[serde(default)]
        pub sections: Vec<Section>,
        pub items: Vec<ListItem>,
    }

    #[serde(rename_all = "camelCase")]
    pub struct Section {
        pub id: String,
        pub title: String,
        #[serde(default)]
        pub subtitle: Option<String>,
    }

    /// Who narrows a list as the user types. `Launcher` filters the given items
    /// without re-invoking the command; `Command` means the command owns
    /// filtering and receives each query change.
    #[derive(Default)]
    #[serde(rename_all = "camelCase")]
    pub enum Filtering {
        #[default]
        Launcher,
        Command,
    }

    #[derive(Default)]
    #[serde(rename_all = "camelCase")]
    pub struct ListItem {
        pub id: String,
        pub title: String,
        #[serde(default)]
        pub subtitle: Option<String>,
        #[serde(default)]
        pub icon: Option<String>,
        /// The id of the section it is shown under. Items without one come
        /// first, under no heading.
        #[serde(default)]
        pub section: Option<String>,
        /// A row shows the first three.
        #[serde(default)]
        pub accessories: Vec<Accessory>,
        /// Matched by the launcher's filter as the title is, never shown.
        #[serde(default)]
        pub keywords: Vec<String>,
        /// The selected item's action panel. The first action is primary.
        #[serde(default)]
        pub actions: Vec<Action>,
    }

    #[serde(tag = "kind", rename_all = "camelCase")]
    pub enum Accessory {
        Text { text: String },
        Icon { icon: String },
        Tag {
            text: String,
            #[serde(default)]
            role: TagRole,
        },
        Kbd { shortcut: Shortcut },
    }

    #[derive(Default)]
    #[serde(rename_all = "camelCase")]
    pub enum TagRole {
        Accent,
        #[default]
        Muted,
        Danger,
    }

    #[serde(rename_all = "camelCase")]
    pub struct DetailView {
        pub markdown: String,
        #[serde(default)]
        pub loading: bool,
        #[serde(default)]
        pub actions: Vec<Action>,
    }

    #[serde(rename_all = "camelCase")]
    pub struct FormView {
        #[serde(default)]
        pub item_id: Option<String>,
        pub fields: Vec<FormField>,
        #[serde(default)]
        pub actions: Vec<Action>,
    }

    #[serde(rename_all = "camelCase")]
    pub struct FormField {
        pub id: String,
        pub label: String,
        pub kind: FieldKind,
        #[serde(default)]
        pub value: Option<String>,
    }

    #[serde(rename_all = "camelCase")]
    pub enum FieldKind {
        Text,
        Password,
        Toggle,
        /// A field holding a template. Renders with the arguments it will ask
        /// for shown beneath it, so text that became a placeholder by accident
        /// is visible while it is still being edited.
        Template,
    }

    #[serde(rename_all = "camelCase")]
    pub struct EmptyState {
        pub title: String,
        #[serde(default)]
        pub description: Option<String>,
    }

    /// The first action in a view's list is primary and runs on Enter. Others
    /// are reached through the action panel. Each may declare a shortcut.
    #[serde(rename_all = "camelCase")]
    pub struct Action {
        pub id: String,
        pub title: String,
        #[serde(default)]
        pub shortcut: Option<Shortcut>,
    }

    #[serde(rename_all = "camelCase")]
    pub struct Shortcut {
        pub key: String,
        #[serde(default)]
        pub modifiers: Vec<Modifier>,
    }

    #[serde(rename_all = "camelCase")]
    pub enum Modifier {
        Cmd,
        Ctrl,
        Alt,
        Shift,
    }

    /// The result of a `no-view` command. On success the launcher hides; on
    /// failure it stays open and shows the message.
    #[serde(tag = "status", rename_all = "camelCase")]
    pub enum CommandOutcome {
        Success {
            #[serde(default)]
            message: Option<String>,
        },
        Failure {
            message: String,
        },
    }
}

impl ViewTree {
    pub fn new(view: View) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            view,
        }
    }
}
