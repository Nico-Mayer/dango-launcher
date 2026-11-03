use crate::protocol::{
    Action, EmptyState, FieldKind, FieldOption, GridItem, ListItem, Metadata, Step, View,
};

use super::input::{Chord, Context, ResultItem};
use super::state::{
    Entry, Launcher, Mode, PushedView, ToastAction, ToastStyle, ViewError, FILTER_PREFIX,
};

macro_rules! snapshot_types {
    ($($item:item)*) => {
        $(
            #[derive(Clone, Debug, PartialEq)]
            $item
        )*
    };
}

snapshot_types! {
    /// Everything the renderer draws. Sent whole after every change.
    pub struct Snapshot {
        pub activation: Activation,
        pub focus: Focus,
        /// The chords the launcher wants right now. Any other key stays with the
        /// focused control.
        pub bindings: Vec<Binding>,
        pub screen: Screen,
        pub toast: Option<Toast>,
        /// Shown in its own window, only while the launcher is hidden.
        pub hud: Option<Hud>,
        pub footer: Footer,
        pub panel: Panel,
    }

    /// Bumped on every activation, so the renderer knows to take the cursor.
    pub struct Activation {
        pub count: u64,
        pub probe: Option<u64>,
    }

    /// Which surface owns the keyboard.
    pub enum Focus {
        RootInput,
        ViewInput,
        Detail,
        Form,
        Panel,
        Nothing,
    }

    pub struct Binding {
        pub chord: Chord,
        /// False when a multiline field keeps the chord, as it keeps Enter for
        /// a newline.
        pub in_multiline: bool,
    }

    pub enum Screen {
        /// A view tree this build cannot draw, shown as an empty view.
        Error(EmptyText),
        Root(RootScreen),
        View(ViewScreen),
    }

    pub struct RootScreen {
        pub query: Text,
        pub sections: Vec<Section>,
        pub selected: Option<String>,
        pub empty: Option<EmptyText>,
        /// More results may still arrive for the query.
        pub settling: bool,
    }

    /// A text field's value. The renderer writes `text` into the field only
    /// when `revision` changes, so an answer to an older keystroke never
    /// overwrites newer typing.
    pub struct Text {
        pub text: String,
        pub revision: u64,
    }

    pub struct Section {
        pub heading: Option<String>,
        pub items: Vec<ResultItem>,
    }

    pub struct ViewScreen {
        pub busy: bool,
        pub body: ViewBody,
    }

    pub enum ViewBody {
        List(ListBody),
        Grid(GridBody),
        Detail(DetailBody),
        Form(FormBody),
        Progress(ProgressBody),
    }

    pub struct ListBody {
        pub query: Text,
        pub filter: Option<FilterView>,
        pub sections: Vec<ListSection>,
        /// The selected item's detail, when the list shows a pane for it.
        pub pane: Option<DetailBody>,
        pub selected: Option<String>,
        pub empty: Option<EmptyText>,
        /// Loading with nothing declared yet: placeholder rows fill the body.
        pub skeleton: bool,
    }

    /// The view's filter dropdown with the option it shows.
    pub struct FilterView {
        pub options: Vec<FieldOption>,
        pub selected: String,
    }

    pub struct GridBody {
        pub query: Text,
        pub filter: Option<FilterView>,
        pub columns: u8,
        pub aspect: f32,
        pub sections: Vec<GridSection>,
        pub selected: Option<String>,
        pub empty: Option<EmptyText>,
    }

    pub struct GridSection {
        pub title: Option<String>,
        pub subtitle: Option<String>,
        pub items: Vec<GridItem>,
    }

    pub struct ListSection {
        pub title: Option<String>,
        pub subtitle: Option<String>,
        pub items: Vec<ListItem>,
    }

    pub struct EmptyText {
        pub icon: Option<String>,
        pub title: String,
        pub description: Option<String>,
        /// A title quoting the query, which can be any length.
        pub truncate: bool,
    }

    #[derive(Default)]
    pub struct DetailBody {
        pub image: Option<String>,
        pub markdown: String,
        pub text: Option<String>,
        pub metadata: Vec<Metadata>,
    }

    pub struct ProgressBody {
        pub title: String,
        pub subtitle: String,
        pub value: Option<f32>,
        pub detail: String,
        pub steps: Vec<Step>,
    }

    pub struct FormBody {
        /// Bumped when the form is seeded from a tree. The fields take their
        /// values from the snapshot only then.
        pub revision: u64,
        pub fields: Vec<Field>,
    }

    pub struct Field {
        pub id: String,
        pub label: String,
        pub kind: FieldKind,
        pub value: String,
        pub help: Option<String>,
        pub help_emphasized: bool,
        pub error: Option<String>,
        pub options: Vec<FieldOption>,
    }

    pub struct Toast {
        pub style: ToastStyle,
        pub text: String,
        pub action: Option<ToastAction>,
    }

    pub struct Hud {
        /// A new id is a new HUD, even with the same text.
        pub id: u64,
        pub style: ToastStyle,
        pub text: String,
    }

    pub struct Footer {
        /// In root search the selected result's extension, in a view its
        /// command.
        pub context: Context,
        /// What Enter runs. Nothing when there is nothing to confirm.
        pub primary_label: Option<String>,
        /// The confirm key is Cmd or Ctrl with Enter, because Enter makes a
        /// newline in a template field.
        pub confirm_with_command: bool,
        /// More than one action, so the panel is worth offering.
        pub many_actions: bool,
    }

    pub struct Panel {
        /// What the panel would offer. Empty disables its trigger.
        pub actions: Vec<Action>,
        pub open: bool,
        /// What the panel's own filter holds. Narrows `actions`.
        pub filter: String,
        /// The entry marked as chosen: the view's selected filter option.
        pub checked: Option<String>,
        pub selected: Option<String>,
    }
}

impl Launcher {
    pub fn snapshot(&self) -> Snapshot {
        let mode = self.mode();
        let selected = self.pick_entry(&self.entries());
        let actions = self.actions_for(selected);
        let panel_actions = self.panel_actions_for(actions);
        Snapshot {
            activation: Activation {
                count: self.activations,
                probe: self.probe,
            },
            focus: match mode {
                Mode::Error => Focus::Nothing,
                Mode::Panel => Focus::Panel,
                Mode::Root => Focus::RootInput,
                Mode::List | Mode::Grid => Focus::ViewInput,
                Mode::Detail => Focus::Detail,
                Mode::Form => Focus::Form,
            },
            bindings: self.bindings(mode, actions),
            screen: match (self.view_error, self.stack.last()) {
                (Some(error), _) => Screen::Error(error_text(error)),
                (None, None) => Screen::Root(self.root_screen()),
                (None, Some(view)) => Screen::View(self.view_screen(view, selected)),
            },
            toast: self.toast(),
            hud: self.hud.as_ref().filter(|_| !self.visible).map(|hud| Hud {
                id: hud.id,
                style: hud.style,
                text: hud.text.clone(),
            }),
            footer: self.footer(actions),
            panel: Panel {
                selected: self.panel_selected(&panel_actions),
                actions: panel_actions,
                checked: self
                    .filter_selected()
                    .map(|id| format!("{FILTER_PREFIX}{id}")),
                open: self.panel_open(),
                filter: self.panel_filter().to_string(),
            },
        }
    }

    fn bindings(&self, mode: Mode, actions: &[Action]) -> Vec<Binding> {
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
            // Left and Right move the selection, so they never reach the
            // SearchBar's text cursor.
            Mode::Grid => {
                let mut keys = [
                    "Escape",
                    "ArrowUp",
                    "ArrowDown",
                    "ArrowLeft",
                    "ArrowRight",
                    "Enter",
                ]
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
        // Only a form moves focus with Tab. Anywhere else it would take the
        // keyboard from the SearchBar or the panel's filter.
        if mode != Mode::Form {
            let shift_tab = Chord {
                shift: true,
                ..Chord::plain("Tab")
            };
            bindings.push(plain("Tab"));
            bindings.push(anywhere(shift_tab));
        }
        if let Some(action) = self.toast_action() {
            let chord = Chord::from(&action.shortcut);
            if !bindings.iter().any(|taken| taken.chord == chord) {
                bindings.push(anywhere(chord));
            }
        }
        if !matches!(mode, Mode::Error | Mode::Panel) {
            let shortcuts = actions.iter().filter_map(|a| a.shortcut.as_ref());
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
            // Said only once every provider has answered, so a slow one does
            // not flash it up between keystrokes.
            empty: (self.results.is_empty() && !self.query.is_empty() && self.results_complete)
                .then(|| EmptyText {
                    icon: Some(SEARCH_ICON.to_string()),
                    title: format!("No results for “{}”", self.query),
                    description: None,
                    truncate: true,
                }),
            settling: !self.results_complete,
        }
    }

    fn view_screen(&self, view: &PushedView, selected: Option<Entry>) -> ViewScreen {
        let list_selected = match selected {
            Some(Entry::List(item)) => Some(item),
            _ => None,
        };
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
                let empty = empty_text(
                    sections.is_empty(),
                    list.loading,
                    &view.query,
                    list.empty_state.as_ref(),
                );
                ViewBody::List(ListBody {
                    query: Text {
                        text: view.query.clone(),
                        revision: view.query_revision,
                    },
                    filter: self.filter_view(),
                    sections,
                    pane: list.detail_pane.then(|| {
                        list_selected
                            .and_then(|item| item.detail.as_ref())
                            .map(|detail| DetailBody {
                                image: detail.image.clone(),
                                markdown: detail.markdown.clone(),
                                text: detail.text.clone(),
                                metadata: detail.metadata.clone(),
                            })
                            .unwrap_or_default()
                    }),
                    selected: list_selected.map(|item| item.id.clone()),
                    empty,
                    skeleton: list.loading && list.items.is_empty(),
                })
            }
            View::Grid(grid) => {
                let sections: Vec<GridSection> = self
                    .grid_sections()
                    .into_iter()
                    .map(|(section, items)| GridSection {
                        title: section.map(|s| s.title.clone()),
                        subtitle: section.and_then(|s| s.subtitle.clone()),
                        items: items.into_iter().cloned().collect(),
                    })
                    .collect();
                let empty = empty_text(
                    sections.is_empty(),
                    grid.loading,
                    &view.query,
                    grid.empty_state.as_ref(),
                );
                ViewBody::Grid(GridBody {
                    query: Text {
                        text: view.query.clone(),
                        revision: view.query_revision,
                    },
                    filter: self.filter_view(),
                    columns: grid.columns,
                    aspect: grid.aspect,
                    sections,
                    selected: selected.map(|entry| entry.id().to_string()),
                    empty,
                })
            }
            View::Detail(detail) => ViewBody::Detail(DetailBody {
                image: detail.image.clone(),
                markdown: detail.markdown.clone(),
                text: detail.text.clone(),
                metadata: detail.metadata.clone(),
            }),
            View::Progress(progress) => ViewBody::Progress(ProgressBody {
                title: progress.title.clone(),
                subtitle: progress.subtitle.clone(),
                value: progress.value,
                detail: progress.detail.clone(),
                steps: progress.steps.clone(),
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
                            options: field.options.clone(),
                        }
                    })
                    .collect(),
            }),
        };
        ViewScreen {
            busy: self.busy_marks_shown().is_some() && !self.busy_shows_task(),
            body,
        }
    }

    fn filter_view(&self) -> Option<FilterView> {
        Some(FilterView {
            options: self.filter()?.options.clone(),
            selected: self.filter_selected()?,
        })
    }

    /// A held toast wins; otherwise the work in progress shows as busy.
    fn toast(&self) -> Option<Toast> {
        if let Some(toast) = &self.toast {
            return Some(Toast {
                style: toast.style,
                text: toast.text.clone(),
                action: toast.action.clone(),
            });
        }
        self.busy_marks_shown().map(|text| Toast {
            style: ToastStyle::Busy,
            text: text.to_string(),
            action: self.busy_toast_action(),
        })
    }

    fn footer(&self, actions: &[Action]) -> Footer {
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
                many_actions: actions.len() > 1,
            };
        };
        Footer {
            context: view.context.clone(),
            primary_label: actions.first().map(|action| action.title.clone()),
            confirm_with_command: matches!(&view.tree.view, View::Form(form)
                if form.fields.iter().any(|field| field.kind.is_multiline())),
            many_actions: actions.len() > 1,
        }
    }
}

const SEARCH_ICON: &str = "icon:search";

/// What a list or grid with nothing to show says. A loading view says so in
/// its busy toast, not in its body.
fn empty_text(
    nothing: bool,
    loading: bool,
    query: &str,
    state: Option<&EmptyState>,
) -> Option<EmptyText> {
    if !nothing || loading {
        return None;
    }
    let icon = state
        .and_then(|state| state.icon.clone())
        .or_else(|| Some(SEARCH_ICON.to_string()));
    Some(if !query.is_empty() {
        EmptyText {
            icon,
            title: format!("No results for “{query}”"),
            description: None,
            truncate: true,
        }
    } else if let Some(state) = state {
        EmptyText {
            icon,
            title: state.title.clone(),
            description: state.description.clone(),
            truncate: false,
        }
    } else {
        EmptyText::plain("Nothing to show")
    })
}

fn error_text(error: ViewError) -> EmptyText {
    let title = match error {
        ViewError::Unsupported => "This view needs a newer version of Dango",
        ViewError::Invalid => "This view can't be shown",
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
