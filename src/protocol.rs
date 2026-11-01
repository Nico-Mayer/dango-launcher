//! The view protocol: the versioned, declarative description of what a command
//! puts on screen.

use serde::{Deserialize, Serialize};

/// Bumped when a tree from outside this build could be read wrongly, which no
/// tree can be yet: every producer is a built-in. Dango refuses a tree whose
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
        Grid(GridView),
        Detail(DetailView),
        Form(FormView),
    }

    #[derive(Default)]
    #[serde(rename_all = "camelCase")]
    pub struct ListView {
        #[serde(default)]
        pub loading: bool,
        #[serde(default)]
        pub empty_state: Option<EmptyState>,
        /// In the order they are shown. Items name the one they belong to.
        #[serde(default)]
        pub sections: Vec<Section>,
        /// Shows the selected item's detail beside the list.
        #[serde(default)]
        pub detail_pane: bool,
        #[serde(default)]
        pub filter: Option<Filter>,
        pub items: Vec<ListItem>,
    }

    /// Tiles in a fixed number of columns. Shares the list's sections,
    /// empty state, and actions.
    #[derive(Default)]
    #[serde(rename_all = "camelCase")]
    pub struct GridView {
        /// From 4 to 8; any other count is refused.
        pub columns: u8,
        /// A tile's width over its height.
        #[serde(default = "square")]
        pub aspect: f32,
        #[serde(default)]
        pub loading: bool,
        #[serde(default)]
        pub empty_state: Option<EmptyState>,
        #[serde(default)]
        pub sections: Vec<Section>,
        #[serde(default)]
        pub filter: Option<Filter>,
        pub items: Vec<GridItem>,
    }

    #[derive(Default)]
    #[serde(rename_all = "camelCase")]
    pub struct GridItem {
        pub id: String,
        pub title: String,
        #[serde(default)]
        pub subtitle: Option<String>,
        pub content: TileContent,
        #[serde(default)]
        pub section: Option<String>,
        #[serde(default)]
        pub keywords: Vec<String>,
        #[serde(default)]
        pub actions: Vec<Action>,
    }

    #[derive(Default)]
    #[serde(tag = "kind", rename_all = "camelCase")]
    pub enum TileContent {
        /// `icon:<name>` or a path to an image file.
        Image { image: String },
        /// `#rrggbb`.
        Color { color: String },
        Glyph { glyph: String },
        #[default]
        Empty,
    }

    /// A dropdown at the SearchBar's end. Choosing an option runs
    /// `FILTER_ACTION` on the view's command with the option's id as the item.
    #[serde(rename_all = "camelCase")]
    pub struct Filter {
        pub options: Vec<FieldOption>,
        pub selected: String,
    }

    #[serde(rename_all = "camelCase")]
    pub struct Section {
        pub id: String,
        pub title: String,
        #[serde(default)]
        pub subtitle: Option<String>,
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
        /// Shown in the list's detail pane while the item is selected.
        #[serde(default)]
        pub detail: Option<ItemDetail>,
        /// The selected item's action panel. The first action is primary.
        #[serde(default)]
        pub actions: Vec<Action>,
    }

    #[derive(Default)]
    #[serde(rename_all = "camelCase")]
    pub struct ItemDetail {
        /// `icon:<name>` or a path to an image file, shown above the body.
        #[serde(default)]
        pub image: Option<String>,
        #[serde(default)]
        pub markdown: String,
        /// Shown as written in place of `markdown`.
        #[serde(default)]
        pub text: Option<String>,
        #[serde(default)]
        pub metadata: Vec<Metadata>,
    }

    /// A label and value pair shown below a detail's body.
    #[serde(rename_all = "camelCase")]
    pub struct Metadata {
        pub label: String,
        pub value: String,
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
    #[derive(Default)]
    pub struct DetailView {
        /// `icon:<name>` or a path to an image file, shown above the body.
        #[serde(default)]
        pub image: Option<String>,
        pub markdown: String,
        /// Shown as written in place of `markdown`.
        #[serde(default)]
        pub text: Option<String>,
        #[serde(default)]
        pub metadata: Vec<Metadata>,
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

    #[derive(Default)]
    #[serde(rename_all = "camelCase")]
    pub struct FormField {
        pub id: String,
        #[serde(default)]
        pub label: String,
        pub kind: FieldKind,
        #[serde(default)]
        pub value: Option<String>,
        /// What a dropdown offers. Its value is the chosen option's id.
        #[serde(default)]
        pub options: Vec<FieldOption>,
    }

    #[derive(Default)]
    #[serde(rename_all = "camelCase")]
    pub struct FieldOption {
        pub id: String,
        pub title: String,
    }

    #[derive(Default)]
    #[serde(rename_all = "camelCase")]
    pub enum FieldKind {
        #[default]
        Text,
        Password,
        /// Several lines. Enter makes a newline, as in a template.
        Textarea,
        Toggle,
        Dropdown,
        /// A field holding a template. Renders with the arguments it will ask
        /// for shown beneath it, so text that became a placeholder by accident
        /// is visible while it is still being edited.
        Template,
        /// A line between fields. Holds no value.
        Separator,
    }

    #[derive(Default)]
    #[serde(rename_all = "camelCase")]
    pub struct EmptyState {
        pub title: String,
        #[serde(default)]
        pub description: Option<String>,
        #[serde(default)]
        pub icon: Option<String>,
        /// Offered while the view shows nothing. The first runs on Enter.
        #[serde(default)]
        pub actions: Vec<Action>,
    }

    /// The first action in a view's list is primary and runs on Enter. Others
    /// are reached through the action panel. Each may declare a shortcut.
    #[derive(Default)]
    #[serde(rename_all = "camelCase")]
    pub struct Action {
        pub id: String,
        pub title: String,
        #[serde(default)]
        pub shortcut: Option<Shortcut>,
        /// The heading it is listed under in the action panel.
        #[serde(default)]
        pub section: Option<String>,
        #[serde(default)]
        pub style: ActionStyle,
    }

    /// A danger action is listed after all others, in the danger role.
    #[derive(Default)]
    #[serde(rename_all = "camelCase")]
    pub enum ActionStyle {
        #[default]
        Normal,
        Danger,
    }

    /// With Shift or Alt, `key` must be one letter or a named key such as
    /// `Enter` or `ArrowUp`: key events carry the character a key produced,
    /// so Shift+1 arrives as `!`. Dango refuses a tree that breaks this. On
    /// macOS, Alt with a letter and without Cmd or Ctrl types a composed
    /// character and does not fire.
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
}

fn square() -> f32 {
    1.0
}

/// The action a view's command receives when the user chooses a filter
/// option. The option's id arrives as the item.
pub const FILTER_ACTION: &str = "filter";

/// The column counts a grid may declare.
pub const GRID_COLUMNS: std::ops::RangeInclusive<u8> = 4..=8;

impl FieldKind {
    /// Keeps Enter for a newline, so the form submits on Cmd or Ctrl with Enter.
    pub fn is_multiline(&self) -> bool {
        matches!(self, Self::Textarea | Self::Template)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_detail_without_image_or_text_still_reads() {
        let detail: ItemDetail = serde_json::from_str(r#"{"markdown":"Hi"}"#).unwrap();
        assert_eq!(detail.image, None);
        assert_eq!(detail.text, None);
        let view: DetailView = serde_json::from_str(r#"{"markdown":"Hi"}"#).unwrap();
        assert_eq!(view.image, None);
        assert_eq!(view.text, None);
    }
}
