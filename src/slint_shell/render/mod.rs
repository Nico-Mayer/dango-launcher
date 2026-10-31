//! Draws a snapshot. Every snapshot replaces what is on screen; the only state
//! kept here decides when to write into a field the user may be typing in, and
//! when to move focus or scroll. Each area of the launcher has a global, and
//! each global has one writer here.

mod detail;
mod empty;
mod form;
mod list;
mod panel;
mod shell;

use std::cell::RefCell;
use std::rc::Rc;

use slint::{ComponentHandle, Image};

use super::generated::{IconKind, Intents, LauncherWindow, ScreenKind};
use super::icons::{Icon, Icons};
use super::keys::{self, Held};
use crate::ui::snapshot::{Focus, Screen, Snapshot, ViewBody};
use crate::ui::{Chord, Intent};

pub struct Renderer {
    icons: Icons,
    last: Option<Snapshot>,
    list: list::Writer,
    panel: panel::Writer,
    form: form::Writer,
    detail: detail::Writer,
    query: Option<(ScreenKind, u64)>,
    activation: u64,
    focus: Option<Focus>,
    /// The activation probe still waiting for its frame.
    painting: Option<u64>,
}

impl Renderer {
    pub fn new(ui: &LauncherWindow) -> Self {
        let icons = Icons::default();
        shell::init(ui, &icons);
        Self {
            list: list::Writer::new(ui),
            panel: panel::Writer::new(ui),
            form: form::Writer::new(ui),
            detail: detail::Writer::new(ui),
            icons,
            last: None,
            query: None,
            activation: 0,
            focus: None,
            painting: None,
        }
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
        let kind = screen_kind(&snapshot.screen);
        shell::write(ui, &snapshot, kind, &self.icons);

        let mut selected = None;
        match &snapshot.screen {
            Screen::Error(text) => {
                self.list.clear();
                empty::write(ui, text, &self.icons);
            }
            Screen::Root(root) => {
                self.write_query(ui, kind, &root.query.text, root.query.revision);
                self.list.root(ui, root, &self.icons);
                if let Some(text) = &root.empty {
                    empty::write(ui, text, &self.icons);
                }
                selected = root.selected.clone();
            }
            Screen::View(view) => match &view.body {
                ViewBody::List(body) => {
                    self.write_query(ui, kind, &body.query.text, body.query.revision);
                    self.list.view(ui, body, &self.icons);
                    if let Some(pane) = &body.pane {
                        self.detail.write(pane);
                    }
                    if let Some(text) = &body.empty {
                        empty::write(ui, text, &self.icons);
                    }
                    selected = body.selected.clone();
                }
                ViewBody::Detail(body) => self.detail.write(body),
                ViewBody::Form(body) => self.form.write(body),
            },
        }
        if !matches!(
            kind,
            ScreenKind::Root | ScreenKind::List | ScreenKind::Empty
        ) || matches!(snapshot.screen, Screen::Error(_))
        {
            self.query = None;
        }
        if kind != ScreenKind::Form {
            self.form.forget();
        }

        self.list.reveal(ui, selected);
        self.panel.write(ui, &snapshot.panel);
        self.place_focus(ui, &snapshot);
        self.last = Some(snapshot);
    }

    /// Writes the field only when the revision moved, so an answer to an older
    /// keystroke never overwrites newer typing.
    fn write_query(&mut self, ui: &LauncherWindow, kind: ScreenKind, text: &str, revision: u64) {
        let kind = if kind == ScreenKind::Empty {
            ScreenKind::List
        } else {
            kind
        };
        if self.query != Some((kind, revision)) {
            ui.invoke_set_query(text.into());
            self.query = Some((kind, revision));
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
            Focus::Panel => ui.invoke_focus_panel_filter(),
            Focus::Detail | Focus::Nothing => ui.invoke_focus_surface(),
        }
    }
}

/// What the body of the launcher shows. An empty list is the empty view.
fn screen_kind(screen: &Screen) -> ScreenKind {
    match screen {
        Screen::Error(_) => ScreenKind::Empty,
        Screen::Root(root) if root.empty.is_some() => ScreenKind::Empty,
        Screen::Root(_) => ScreenKind::Root,
        Screen::View(view) => match &view.body {
            ViewBody::List(list) if list.empty.is_some() => ScreenKind::Empty,
            ViewBody::List(_) => ScreenKind::List,
            ViewBody::Detail(_) => ScreenKind::Detail,
            ViewBody::Form(_) => ScreenKind::Form,
        },
    }
}

fn icon(icons: &Icons, icon: Option<&str>) -> (Image, IconKind) {
    match icons.get(icon) {
        Icon::Named(image) => (image, IconKind::Named),
        Icon::File(image) => (image, IconKind::File),
        Icon::Missing => (Image::default(), IconKind::None),
    }
}

/// Connects the component's callbacks to whatever takes the launcher's
/// intents: the core in the app, the state machine alone in tests.
pub fn connect(ui: &LauncherWindow, renderer: Rc<RefCell<Renderer>>, dispatch: Rc<dyn Fn(Intent)>) {
    let intents = ui.global::<Intents>();
    let weak = ui.as_weak();
    let send = dispatch.clone();
    intents.on_key(move |text, control, alt, shift, meta| {
        let held = Held {
            control,
            alt,
            shift,
            meta,
        };
        let Some(chord) = keys::chord(&text, held) else {
            return false;
        };
        let in_multiline = weak.upgrade().is_some_and(|ui| {
            ui.global::<super::generated::Shell>()
                .get_multiline_focused()
        });
        if !renderer.borrow().binds(&chord, in_multiline) {
            return false;
        }
        send(Intent::Key { chord });
        true
    });
    let send = dispatch.clone();
    intents.on_query_edited(move |text| send(Intent::QueryChanged { text: text.into() }));
    let send = dispatch.clone();
    intents.on_field_edited(move |id, value| {
        send(Intent::FieldChanged {
            id: id.into(),
            value: value.into(),
        })
    });
    let send = dispatch.clone();
    intents.on_activate(move |id| send(Intent::Activate { id: id.into() }));
    let send = dispatch.clone();
    intents.on_toggle_panel(move || send(Intent::TogglePanel));
    let send = dispatch.clone();
    intents.on_close_panel(move || send(Intent::ClosePanel));
    let send = dispatch.clone();
    intents.on_go_back(move || send(Intent::GoBack));
    intents.on_panel_filter_edited(move |text| {
        dispatch(Intent::PanelFilterChanged { text: text.into() })
    });
}
