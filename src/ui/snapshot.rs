use serde::Serialize;

use crate::protocol::{Action, FieldKind, ListItem, Metadata, View};

use super::input::{Chord, Context, ResultItem};
use super::state::{Launcher, Mode, PushedView, ToastStyle, ViewError};

macro_rules! snapshot_types {
    ($($item:item)*) => {
        $(
            #[derive(Clone, Debug, PartialEq, Serialize)]
            $item
        )*
    };
}

snapshot_types! {
    /// Everything the renderer draws. Sent whole after every change.
    #[serde(rename_all = "camelCase")]
    pub struct Snapshot {
        pub activation: Activation,
        pub focus: Focus,
        /// The chords the launcher wants right now. Any other key stays with the
        /// focused control.
        pub bindings: Vec<Binding>,
        pub screen: Screen,
        pub toast: Option<Toast>,
        pub footer: Footer,
        pub panel: Panel,
    }

    /// Bumped on every activation, so the renderer knows to take the cursor.
    #[serde(rename_all = "camelCase")]
    pub struct Activation {
        pub count: u64,
        pub probe: Option<u64>,
    }

    /// Which surface owns the keyboard.
    #[serde(rename_all = "camelCase")]
    pub enum Focus {
        RootInput,
        ViewInput,
        Detail,
        Form,
        Panel,
        Nothing,
    }

    #[serde(rename_all = "camelCase")]
    pub struct Binding {
        pub chord: Chord,
        /// False when a multiline field keeps the chord, as it keeps Enter for
        /// a newline.
        pub in_multiline: bool,
    }

    #[serde(tag = "kind", rename_all = "camelCase")]
    pub enum Screen {
        /// A view tree this build cannot draw, shown as an empty view.
        Error(EmptyText),
        Root(RootScreen),
        View(ViewScreen),
    }

    #[serde(rename_all = "camelCase")]
    pub struct RootScreen {
        pub query: Text,
        pub sections: Vec<Section>,
        pub selected: Option<String>,
        pub empty: Option<EmptyText>,
        pub busy: bool,
    }

    /// A text field's value. The renderer writes `text` into the field only
    /// when `revision` changes, so an answer to an older keystroke never
    /// overwrites newer typing.
    #[serde(rename_all = "camelCase")]
    pub struct Text {
        pub text: String,
        pub revision: u64,
    }

    #[serde(rename_all = "camelCase")]
    pub struct Section {
        pub heading: Option<String>,
        pub items: Vec<ResultItem>,
    }

    #[serde(rename_all = "camelCase")]
    pub struct ViewScreen {
        pub busy: bool,
        pub body: ViewBody,
    }

    #[serde(tag = "kind", rename_all = "camelCase")]
    pub enum ViewBody {
        List(ListBody),
        Detail(DetailBody),
        Form(FormBody),
    }

    #[serde(rename_all = "camelCase")]
    pub struct ListBody {
        pub query: Text,
        pub sections: Vec<ListSection>,
        /// The selected item's detail, when the list shows a pane for it.
        pub pane: Option<DetailBody>,
        pub selected: Option<String>,
        pub empty: Option<EmptyText>,
    }

    #[serde(rename_all = "camelCase")]
    pub struct ListSection {
        pub title: Option<String>,
        pub subtitle: Option<String>,
        pub items: Vec<ListItem>,
    }

    #[serde(rename_all = "camelCase")]
    pub struct EmptyText {
        pub icon: Option<String>,
        pub title: String,
        pub description: Option<String>,
        /// A title quoting the query, which can be any length.
        pub truncate: bool,
    }

    #[serde(rename_all = "camelCase")]
    #[derive(Default)]
    pub struct DetailBody {
        pub markdown: String,
        pub metadata: Vec<Metadata>,
    }

    #[serde(rename_all = "camelCase")]
    pub struct FormBody {
        /// Bumped when the form is seeded from a tree. The fields take their
        /// values from the snapshot only then.
        pub revision: u64,
        pub fields: Vec<Field>,
    }

    #[serde(rename_all = "camelCase")]
    pub struct Field {
        pub id: String,
        pub label: String,
        pub kind: FieldKind,
        pub value: String,
        pub help: Option<String>,
        pub help_emphasized: bool,
        pub error: Option<String>,
    }

    #[serde(rename_all = "camelCase")]
    pub struct Toast {
        pub style: ToastStyle,
        pub text: String,
    }

    #[serde(rename_all = "camelCase")]
    pub struct Footer {
        /// In root search the selected result's extension, in a view its
        /// command.
        pub context: Context,
        /// What Enter runs. Nothing when there is nothing to confirm.
        pub primary_label: Option<String>,
        /// The confirm key is Cmd or Ctrl with Enter, because Enter makes a
        /// newline in a template field.
        pub confirm_with_command: bool,
    }

    #[serde(rename_all = "camelCase")]
    pub struct Panel {
        /// What the panel would offer. Empty disables its trigger.
        pub actions: Vec<Action>,
        pub open: bool,
        pub selected: Option<String>,
    }
}

impl Launcher {
    pub fn snapshot(&self) -> Snapshot {
        let mode = self.mode();
        Snapshot {
            activation: Activation {
                count: self.activations,
                probe: self.probe,
            },
            focus: match mode {
                Mode::Error => Focus::Nothing,
                Mode::Panel => Focus::Panel,
                Mode::Root => Focus::RootInput,
                Mode::List => Focus::ViewInput,
                Mode::Detail => Focus::Detail,
                Mode::Form => Focus::Form,
            },
            bindings: self.bindings(mode),
            screen: match (self.view_error, self.stack.last()) {
                (Some(error), _) => Screen::Error(error_text(error)),
                (None, None) => Screen::Root(self.root_screen()),
                (None, Some(view)) => Screen::View(self.view_screen(view)),
            },
            toast: self.toast(),
            footer: self.footer(),
            panel: Panel {
                actions: self.panel_actions().into_iter().cloned().collect(),
                open: self.panel_open(),
                selected: self.panel_selected(),
            },
        }
    }

    fn bindings(&self, mode: Mode) -> Vec<Binding> {
        let anywhere = |chord| Binding {
            chord,
            in_multiline: true,
        };
        let plain = |key| anywhere(Chord::plain(key));
        let mut bindings = match mode {
            Mode::Error => vec![plain("Escape")],
            Mode::Panel => ["ArrowUp", "ArrowDown", "Enter", "Escape"]
                .map(plain)
                .to_vec(),
            Mode::Root | Mode::List => {
                let mut keys = ["Escape", "ArrowUp", "ArrowDown", "Enter"]
                    .map(plain)
                    .to_vec();
                keys.push(anywhere(Chord::command("k")));
                keys
            }
            Mode::Detail => vec![
                plain("Escape"),
                plain("Enter"),
                anywhere(Chord::command("k")),
            ],
            Mode::Form => vec![
                plain("Escape"),
                Binding {
                    chord: Chord::plain("Enter"),
                    in_multiline: false,
                },
                anywhere(Chord::command("Enter")),
                anywhere(Chord::command("k")),
            ],
        };
        if !matches!(mode, Mode::Error | Mode::Panel) {
            let shortcuts = self
                .available_actions()
                .iter()
                .filter_map(|a| a.shortcut.as_ref());
            for chord in shortcuts.map(Chord::from) {
                if !bindings.iter().any(|taken| taken.chord == chord) {
                    bindings.push(anywhere(chord));
                }
            }
        }
        bindings
    }

    fn root_screen(&self) -> RootScreen {
        let pick = |keep: fn(&ResultItem) -> bool| -> Vec<ResultItem> {
            self.results.iter().filter(|r| keep(r)).cloned().collect()
        };
        let pinned = pick(|r| r.pinned);
        let suggested = pick(|r| !r.pinned && r.suggested);
        let rest = pick(|r| !r.pinned && !r.suggested);
        let section = |heading: Option<&str>, items| Section {
            heading: heading.map(str::to_string),
            items,
        };
        let mut sections = Vec::new();
        let grouped = !pinned.is_empty() || !suggested.is_empty();
        if !pinned.is_empty() {
            sections.push(section(Some("Needs attention"), pinned));
        }
        if !suggested.is_empty() {
            sections.push(section(Some("Suggestions"), suggested));
        }
        sections.push(section(grouped.then_some("Everything else"), rest));
        sections.retain(|section| !section.items.is_empty());
        RootScreen {
            query: Text {
                text: self.query.clone(),
                revision: self.query_revision,
            },
            sections,
            selected: self.root_selected().map(|item| item.id.clone()),
            empty: (self.results.is_empty() && !self.query.is_empty()).then(|| EmptyText {
                icon: Some(SEARCH_ICON.to_string()),
                title: format!("No results for “{}”", self.query),
                description: None,
                truncate: true,
            }),
            busy: self.working.is_some(),
        }
    }

    fn view_screen(&self, view: &PushedView) -> ViewScreen {
        let body = match &view.tree.view {
            View::List(list) => {
                let sections: Vec<ListSection> = self
                    .view_sections()
                    .into_iter()
                    .map(|(section, items)| ListSection {
                        title: section.map(|s| s.title.clone()),
                        subtitle: section.and_then(|s| s.subtitle.clone()),
                        items: items.into_iter().cloned().collect(),
                    })
                    .collect();
                // A loading list says so in its busy toast, not in its body.
                let empty = (sections.is_empty() && !list.loading).then(|| {
                    if !view.query.is_empty() {
                        EmptyText {
                            icon: list
                                .empty_state
                                .as_ref()
                                .and_then(|s| s.icon.clone())
                                .or_else(|| Some(SEARCH_ICON.to_string())),
                            title: format!("No results for “{}”", view.query),
                            description: None,
                            truncate: true,
                        }
                    } else if let Some(state) = &list.empty_state {
                        EmptyText {
                            icon: state.icon.clone().or_else(|| Some(SEARCH_ICON.to_string())),
                            title: state.title.clone(),
                            description: state.description.clone(),
                            truncate: false,
                        }
                    } else {
                        EmptyText::plain("Nothing to show")
                    }
                });
                ViewBody::List(ListBody {
                    query: Text {
                        text: view.query.clone(),
                        revision: view.query_revision,
                    },
                    sections,
                    pane: list.detail_pane.then(|| {
                        self.view_selected()
                            .and_then(|item| item.detail.as_ref())
                            .map(|detail| DetailBody {
                                markdown: detail.markdown.clone(),
                                metadata: detail.metadata.clone(),
                            })
                            .unwrap_or_default()
                    }),
                    selected: self.view_selected().map(|item| item.id.clone()),
                    empty,
                })
            }
            View::Detail(detail) => ViewBody::Detail(DetailBody {
                markdown: detail.markdown.clone(),
                metadata: detail.metadata.clone(),
            }),
            View::Form(form) => ViewBody::Form(FormBody {
                revision: view.form.revision,
                fields: form
                    .fields
                    .iter()
                    .map(|field| {
                        let template = field.kind == FieldKind::Template;
                        let inspection = view.form.inspections.get(&field.id);
                        let arguments = inspection.map_or(&[][..], |i| &i.arguments);
                        Field {
                            id: field.id.clone(),
                            label: field.label.clone(),
                            kind: field.kind.clone(),
                            value: view.form.values.get(&field.id).cloned().unwrap_or_default(),
                            help: template.then(|| {
                                if arguments.is_empty() {
                                    "Nothing to fill in".to_string()
                                } else {
                                    format!("Will ask for: {}", arguments.join(", "))
                                }
                            }),
                            help_emphasized: !arguments.is_empty(),
                            error: inspection
                                .filter(|_| template)
                                .and_then(|i| i.error.clone()),
                        }
                    })
                    .collect(),
            }),
        };
        ViewScreen {
            busy: is_busy(&view.tree.view),
            body,
        }
    }

    /// A held toast wins; otherwise the work in progress shows as busy.
    fn toast(&self) -> Option<Toast> {
        if let Some(toast) = &self.toast {
            return Some(Toast {
                style: toast.style,
                text: toast.text.clone(),
            });
        }
        let busy = |text: String| {
            Some(Toast {
                style: ToastStyle::Busy,
                text,
            })
        };
        if let Some(title) = &self.working {
            return busy(format!("Running {title}…"));
        }
        match self.stack.last() {
            Some(view) if is_busy(&view.tree.view) => busy("Loading…".to_string()),
            _ => None,
        }
    }

    fn footer(&self) -> Footer {
        let Some(view) = self.stack.last() else {
            // A command result carries no actions and is invoked instead.
            return Footer {
                context: self
                    .root_selected()
                    .map(|item| item.context.clone())
                    .unwrap_or_default(),
                primary_label: self.root_selected().map(|item| {
                    item.actions
                        .first()
                        .map_or_else(|| "Open".to_string(), |action| action.title.clone())
                }),
                confirm_with_command: false,
            };
        };
        Footer {
            context: view.context.clone(),
            primary_label: self
                .available_actions()
                .first()
                .map(|action| action.title.clone()),
            confirm_with_command: matches!(&view.tree.view, View::Form(form)
                if form.fields.iter().any(|field| field.kind == FieldKind::Template)),
        }
    }
}

const SEARCH_ICON: &str = "icon:search";

fn error_text(error: ViewError) -> EmptyText {
    let title = match error {
        ViewError::Unsupported => "This view needs a newer version of Dango",
    };
    EmptyText {
        icon: Some("icon:triangle-alert".to_string()),
        title: title.to_string(),
        description: Some("Press Escape to go back".to_string()),
        truncate: false,
    }
}

impl EmptyText {
    fn plain(title: &str) -> Self {
        Self {
            icon: Some(SEARCH_ICON.to_string()),
            title: title.to_string(),
            description: None,
            truncate: false,
        }
    }
}

fn is_busy(view: &View) -> bool {
    match view {
        View::List(list) => list.loading,
        View::Detail(detail) => detail.loading,
        View::Form(_) => false,
    }
}
