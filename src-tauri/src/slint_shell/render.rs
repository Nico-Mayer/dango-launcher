//! Draws a snapshot. Every snapshot replaces what is on screen; the only state
//! kept here decides when to write into a field the user may be typing in, and
//! when to move focus or scroll.

use std::cell::RefCell;
use std::rc::Rc;

use slint::{ComponentHandle, Model, ModelRc, SharedString, StyledText, VecModel};

use super::generated::{
    ActionData, FieldData, FieldKind, IconKind, LauncherWindow, RowData, ScreenKind,
};
use super::icons::{Icon, Icons};
use super::keys::{self, Held};
use crate::protocol::{self, ListItem};
use crate::ui::snapshot::{Focus, Screen, Snapshot, ViewBody};
use crate::ui::{Chord, Intent, ResultItem};

const ROW_HEIGHT: f32 = 56.0;
const HEADING_HEIGHT: f32 = 28.0;

#[derive(Default)]
pub struct Renderer {
    icons: Icons,
    last: Option<Snapshot>,
    rows: Rc<VecModel<RowData>>,
    actions: Rc<VecModel<ActionData>>,
    fields: Rc<VecModel<FieldData>>,
    query: Option<(ScreenKind, u64)>,
    form: Option<u64>,
    activation: u64,
    focus: Option<Focus>,
    selected: Option<String>,
    panel_selected: Option<String>,
    /// The activation probe still waiting for its frame.
    painting: Option<u64>,
}

impl Renderer {
    pub fn new(ui: &LauncherWindow) -> Self {
        let renderer = Self::default();
        ui.set_rows(ModelRc::from(renderer.rows.clone()));
        ui.set_actions(ModelRc::from(renderer.actions.clone()));
        ui.set_fields(ModelRc::from(renderer.fields.clone()));
        renderer
    }

    /// Whether the launcher wants the chord, by the bindings it last drew.
    pub fn binds(&self, chord: &Chord, in_multiline: bool) -> bool {
        self.last
            .as_ref()
            .is_some_and(|snapshot| keys::bound(&snapshot.bindings, chord, in_multiline))
    }

    /// The probe of an activation that has now been painted.
    pub fn painted(&mut self) -> Option<u64> {
        self.painting.take()
    }

    pub fn render(&mut self, ui: &LauncherWindow, snapshot: Snapshot) {
        ui.set_development(snapshot.development);
        ui.set_failure(snapshot.failure.clone().unwrap_or_default().into());
        let footer = &snapshot.footer;
        ui.set_primary_label(footer.primary_label.as_str().into());
        ui.set_confirm_key(keys::confirm_label(footer.confirm_with_command).into());
        ui.set_status(footer.status.as_str().into());
        ui.set_hide_status(footer.hide_status);
        ui.set_panel_key(keys::panel_label().into());
        ui.set_has_actions(!snapshot.panel.actions.is_empty());

        let kind = match &snapshot.screen {
            Screen::Error => ScreenKind::Error,
            Screen::Root(_) => ScreenKind::Root,
            Screen::View(view) => match view.body {
                ViewBody::List(_) => ScreenKind::List,
                ViewBody::Detail(_) => ScreenKind::Detail,
                ViewBody::Form(_) => ScreenKind::Form,
            },
        };
        ui.set_screen(kind);

        let mut selected = None;
        match &snapshot.screen {
            Screen::Error => self.rows.set_vec(Vec::new()),
            Screen::Root(root) => {
                self.write_query(ui, kind, &root.query.text, root.query.revision);
                let rows = root.sections.iter().flat_map(|section| {
                    let heading = section.heading.iter().map(|heading| heading_row(heading));
                    let items = section
                        .items
                        .iter()
                        .map(|item| self.result_row(item, root.selected.as_deref()));
                    heading.chain(items).collect::<Vec<_>>()
                });
                self.rows.set_vec(rows.collect::<Vec<_>>());
                ui.set_empty(root.empty.clone().unwrap_or_default().into());
                ui.set_empty_description(SharedString::new());
                selected = root.selected.clone();
            }
            Screen::View(view) => match &view.body {
                ViewBody::List(list) => {
                    self.write_query(ui, kind, &list.query.text, list.query.revision);
                    let rows = list
                        .items
                        .iter()
                        .map(|item| self.list_row(item, list.selected.as_deref()));
                    self.rows.set_vec(rows.collect::<Vec<_>>());
                    let empty = list.empty.as_ref();
                    ui.set_empty(empty.map(|e| e.title.as_str()).unwrap_or("").into());
                    ui.set_empty_description(
                        empty
                            .and_then(|e| e.description.as_deref())
                            .unwrap_or("")
                            .into(),
                    );
                    selected = list.selected.clone();
                }
                ViewBody::Detail(detail) => ui.set_detail(detail.markdown.as_str().into()),
                ViewBody::Form(form) => self.write_form(form),
            },
        }
        if !matches!(kind, ScreenKind::Root | ScreenKind::List) {
            self.query = None;
        }
        if kind != ScreenKind::Form {
            self.form = None;
        }

        let actions = snapshot.panel.actions.iter().map(|action| ActionData {
            id: action.id.as_str().into(),
            title: action.title.as_str().into(),
            shortcut: action
                .shortcut
                .as_ref()
                .map(keys::shortcut_label)
                .unwrap_or_default()
                .into(),
            selected: snapshot.panel.selected.as_deref() == Some(action.id.as_str()),
        });
        self.actions.set_vec(actions.collect::<Vec<_>>());
        ui.set_panel_open(snapshot.panel.open);

        if selected != self.selected {
            if let Some(id) = &selected {
                self.reveal(ui, id);
            }
            self.selected = selected;
        }
        if snapshot.panel.selected != self.panel_selected {
            if let Some(index) = snapshot
                .panel
                .actions
                .iter()
                .position(|action| Some(&action.id) == snapshot.panel.selected.as_ref())
            {
                ui.invoke_reveal_action(index as i32);
            }
            self.panel_selected = snapshot.panel.selected.clone();
        }

        self.place_focus(ui, &snapshot);
        self.last = Some(snapshot);
    }

    /// Writes the field only when the revision moved, so an answer to an older
    /// keystroke never overwrites newer typing.
    fn write_query(&mut self, ui: &LauncherWindow, kind: ScreenKind, text: &str, revision: u64) {
        if self.query != Some((kind, revision)) {
            ui.invoke_set_query(text.into());
            self.query = Some((kind, revision));
        }
    }

    /// The fields own their text while the user edits. A new revision seeds
    /// them again; otherwise only their help and errors move.
    fn write_form(&mut self, form: &crate::ui::snapshot::FormBody) {
        let fields: Vec<FieldData> = form.fields.iter().map(field).collect();
        if self.form != Some(form.revision) || self.fields.row_count() != fields.len() {
            self.fields.set_vec(fields);
            self.form = Some(form.revision);
            return;
        }
        for (index, field) in fields.into_iter().enumerate() {
            if self.fields.row_data(index).as_ref() != Some(&field) {
                self.fields.set_row_data(index, field);
            }
        }
    }

    fn place_focus(&mut self, ui: &LauncherWindow, snapshot: &Snapshot) {
        let activated = snapshot.activation.count != self.activation;
        self.activation = snapshot.activation.count;
        if activated {
            self.painting = snapshot.activation.probe;
        }
        let moved = self.focus.as_ref() != Some(&snapshot.focus);
        self.focus = Some(snapshot.focus.clone());
        if !activated && !moved {
            return;
        }
        match snapshot.focus {
            Focus::RootInput | Focus::ViewInput => ui.invoke_focus_prompt(activated),
            // A form puts the cursor in its first field as it is seeded.
            Focus::Form => {}
            Focus::Detail | Focus::Panel | Focus::Nothing => ui.invoke_focus_surface(),
        }
    }

    /// Rows have fixed heights, so the selected row's place is known without
    /// asking the list. A group's first row brings its heading into view too.
    fn reveal(&self, ui: &LauncherWindow, id: &str) {
        let mut top = 0.0;
        let mut heading_above = false;
        for row in self.rows.iter() {
            if !row.heading.is_empty() {
                top += HEADING_HEIGHT;
                heading_above = true;
                continue;
            }
            if row.id == id {
                let (top, height) = if heading_above {
                    (top - HEADING_HEIGHT, ROW_HEIGHT + HEADING_HEIGHT)
                } else {
                    (top, ROW_HEIGHT)
                };
                ui.invoke_reveal_row(top, height);
                return;
            }
            top += ROW_HEIGHT;
            heading_above = false;
        }
    }

    fn result_row(&self, item: &ResultItem, selected: Option<&str>) -> RowData {
        let mut row = self.icon_row(item.icon.as_deref(), item.tint.as_deref(), false);
        row.id = item.id.as_str().into();
        row.title = highlighted(&item.title, &item.match_positions);
        row.subtitle = item.subtitle.clone().unwrap_or_default().into();
        row.selected = selected == Some(item.id.as_str());
        row
    }

    fn list_row(&self, item: &ListItem, selected: Option<&str>) -> RowData {
        let mut row = self.icon_row(item.icon.as_deref(), None, true);
        row.id = item.id.as_str().into();
        row.title = StyledText::from_plain_text(&item.title);
        row.subtitle = item.subtitle.clone().unwrap_or_default().into();
        row.selected = selected == Some(item.id.as_str());
        row
    }

    fn icon_row(&self, icon: Option<&str>, tint: Option<&str>, content: bool) -> RowData {
        let (icon, icon_kind) = match self.icons.get(icon) {
            Icon::Named(image) => (image, IconKind::Named),
            Icon::File(image) => (image, IconKind::File),
            Icon::Missing => (Default::default(), IconKind::None),
        };
        RowData {
            icon,
            icon_kind,
            tint: tint.and_then(tint_index).unwrap_or(-1),
            content,
            ..Default::default()
        }
    }
}

/// Connects the component's callbacks to whatever takes the launcher's
/// intents: the core in the app, the state machine alone in tests.
pub fn connect(ui: &LauncherWindow, renderer: Rc<RefCell<Renderer>>, dispatch: Rc<dyn Fn(Intent)>) {
    let weak = ui.as_weak();
    let send = dispatch.clone();
    ui.on_key(move |text, control, alt, shift, meta| {
        let held = Held {
            control,
            alt,
            shift,
            meta,
        };
        let Some(chord) = keys::chord(&text, held) else {
            return false;
        };
        let in_multiline = weak.upgrade().is_some_and(|ui| ui.get_multiline_focused());
        if !renderer.borrow().binds(&chord, in_multiline) {
            return false;
        }
        send(Intent::Key { chord });
        true
    });
    let send = dispatch.clone();
    ui.on_query_edited(move |text| send(Intent::QueryChanged { text: text.into() }));
    let send = dispatch.clone();
    ui.on_field_edited(move |id, value| {
        send(Intent::FieldChanged {
            id: id.into(),
            value: value.into(),
        })
    });
    let send = dispatch.clone();
    ui.on_activate(move |id| send(Intent::Activate { id: id.into() }));
    let send = dispatch.clone();
    ui.on_toggle_panel(move || send(Intent::TogglePanel));
    ui.on_close_panel(move || dispatch(Intent::ClosePanel));
}

fn heading_row(heading: &str) -> RowData {
    RowData {
        heading: heading.into(),
        ..Default::default()
    }
}

fn field(field: &crate::ui::snapshot::Field) -> FieldData {
    FieldData {
        id: field.id.as_str().into(),
        label: field.label.as_str().into(),
        kind: match field.kind {
            protocol::FieldKind::Text => FieldKind::Text,
            protocol::FieldKind::Password => FieldKind::Password,
            protocol::FieldKind::Toggle => FieldKind::Toggle,
            protocol::FieldKind::Template => FieldKind::Template,
        },
        value: field.value.as_str().into(),
        help: field.help.clone().unwrap_or_default().into(),
        help_emphasized: field.help_emphasized,
        error: field.error.clone().unwrap_or_default().into(),
    }
}

/// The title with the characters that matched the query underlined. Not
/// emphasis, because emphasis markers stop working next to spaces and
/// punctuation and a tag does not.
fn highlighted(title: &str, positions: &[usize]) -> StyledText {
    if positions.is_empty() {
        return StyledText::from_plain_text(title);
    }
    let mut markdown = String::new();
    let mut open = false;
    for (index, c) in title.chars().enumerate() {
        let matched = positions.contains(&index);
        if matched != open {
            markdown.push_str(if matched { "<u>" } else { "</u>" });
            open = matched;
        }
        if c.is_ascii_punctuation() {
            markdown.push('\\');
        }
        markdown.push(c);
    }
    if open {
        markdown.push_str("</u>");
    }
    StyledText::from_markdown(&markdown).unwrap_or_else(|_| StyledText::from_plain_text(title))
}

/// Where an extension's tint sits in the palette's tint list. Nothing for a
/// name this palette does not hold.
fn tint_index(name: &str) -> Option<i32> {
    ["red", "amber", "green", "blue", "purple", "pink"]
        .iter()
        .position(|tint| *tint == name)
        .map(|index| index as i32)
}
