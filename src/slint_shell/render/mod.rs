//! Draws a snapshot. Every snapshot replaces what is on screen; the only state
//! kept here decides when to write into a field the user may be typing in, and
//! when to move focus or scroll. Each area of the launcher has a global, and
//! each global has one writer here.

mod answer;
mod detail;
mod empty;
mod form;
mod grid;
mod height;
mod list;
pub mod notice;
mod panel;
mod progress;
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
    grid: grid::Writer,
    detail: detail::Writer,
    /// Whether root or a view holds the query, and the revision last written.
    query: Option<(bool, u64)>,
    activation: u64,
    focus: Option<Focus>,
    /// The activation probe still waiting for its frame.
    painting: Option<u64>,
    height: f32,
}

impl Renderer {
    pub fn new(ui: &LauncherWindow) -> Self {
        let icons = Icons::default();
        shell::init(ui, &icons);
        Self {
            list: list::Writer::new(ui),
            panel: panel::Writer::new(ui),
            form: form::Writer::new(ui),
            grid: grid::Writer::new(ui),
            detail: detail::Writer::new(ui),
            icons,
            last: None,
            query: None,
            activation: 0,
            focus: None,
            painting: None,
            height: height::MAX,
        }
    }

    /// Whether the launcher wants the chord, by the bindings it last drew.
    pub fn binds(&self, chord: &Chord, in_multiline: bool) -> bool {
        self.last
            .as_ref()
            .is_some_and(|snapshot| keys::bound(&snapshot.bindings, chord, in_multiline))
    }

    /// How tall the window is for this snapshot. While root search is still
    /// receiving results it grows but does not shrink, so a fast provider's
    /// partial answer does not shrink the window just before the rest arrive.
    fn height(&mut self, snapshot: &Snapshot) -> f32 {
        let wanted = height::height(snapshot);
        let settling =
            matches!(&snapshot.screen, Screen::Root(root) if root.settling && !root.bare);
        self.height = if settling {
            wanted.max(self.height)
        } else {
            wanted
        };
        self.height
    }

    /// The probe of an activation that has now been painted.
    pub fn painted(&mut self) -> Option<u64> {
        self.painting.take()
    }

    pub fn render(&mut self, ui: &LauncherWindow, snapshot: Snapshot) {
        let kind = screen_kind(&snapshot.screen);
        shell::write(
            ui,
            &snapshot,
            kind,
            typing(&snapshot.screen),
            self.height,
            &self.icons,
        );

        let mut selected = None;
        let mut detail = false;
        match &snapshot.screen {
            Screen::Error(text) => {
                self.list.clear();
                empty::write(ui, text, &self.icons);
            }
            Screen::Root(root) => {
                self.write_query(ui, true, &root.query.text, root.query.revision);
                self.list.root(ui, root, &self.icons);
                answer::write(ui, Some(root), &self.icons);
                if let Some(text) = &root.empty {
                    empty::write(ui, text, &self.icons);
                }
                selected = root.selected.clone();
            }
            Screen::View(view) => match &view.body {
                ViewBody::List(body) => {
                    self.write_query(ui, false, &body.query.text, body.query.revision);
                    self.list.view(ui, body, &self.icons);
                    if let Some(pane) = &body.pane {
                        self.detail.write(pane, &self.icons);
                        detail = true;
                    }
                    if let Some(text) = &body.empty {
                        empty::write(ui, text, &self.icons);
                    }
                    selected = body.selected.clone();
                }
                ViewBody::Grid(body) => {
                    self.write_query(ui, false, &body.query.text, body.query.revision);
                    self.grid.write(ui, body, &self.icons);
                    if let Some(text) = &body.empty {
                        empty::write(ui, text, &self.icons);
                    }
                }
                ViewBody::Detail(body) => {
                    self.detail.write(body, &self.icons);
                    detail = true;
                }
                ViewBody::Form(body) => self.form.write(body),
                ViewBody::Progress(body) => progress::write(ui, body),
            },
        }
        let root = match &snapshot.screen {
            Screen::Root(root) => Some(root),
            _ => None,
        };
        answer::write(ui, root, &self.icons);
        if !typing(&snapshot.screen) {
            self.query = None;
        }
        if kind != ScreenKind::Form {
            self.form.forget();
        }
        if !detail {
            self.detail.clear();
        }

        self.list.reveal(ui, selected);
        self.panel.write(ui, &snapshot.panel);
        self.place_focus(ui, &snapshot);
        self.icons.rendered();
        self.last = Some(snapshot);
    }

    pub fn icons_mut(&mut self) -> &mut Icons {
        &mut self.icons
    }

    /// Draws the last snapshot again, as when an image it shows is ready.
    pub fn refresh(&mut self, ui: &LauncherWindow) {
        if let Some(snapshot) = self.last.clone() {
            self.render(ui, snapshot);
        }
    }

    /// The list and grid models and the detail hold their images too, so they
    /// go with the cache.
    pub fn forget_images(&mut self) {
        self.list.clear();
        self.grid.clear();
        self.detail.clear();
        self.icons.forget_files();
    }

    #[cfg(test)]
    pub fn holds_images(&self) -> bool {
        self.icons.holds_files()
    }

    #[cfg(test)]
    pub fn decoded_images(&self) -> usize {
        self.icons.decoded()
    }

    /// Writes the field only when the revision moved, so an answer to an older
    /// keystroke never overwrites newer typing.
    fn write_query(&mut self, ui: &LauncherWindow, root: bool, text: &str, revision: u64) {
        if self.query != Some((root, revision)) {
            ui.invoke_set_query(text.into());
            self.query = Some((root, revision));
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

/// Sizes the window for the snapshot, then draws it, in one main-thread turn
/// so no frame shows the old size with the new content or the other way round.
pub fn publish(
    renderer: &mut Renderer,
    ui: &LauncherWindow,
    snapshot: Snapshot,
    resize: impl FnOnce(f32),
) {
    resize(renderer.height(&snapshot));
    renderer.render(ui, snapshot);
}

/// Whether the SearchBar holds a query, which root, a list, and a grid do,
/// empty or not.
fn typing(screen: &Screen) -> bool {
    match screen {
        Screen::Root(_) => true,
        Screen::View(view) => matches!(view.body, ViewBody::List(_) | ViewBody::Grid(_)),
        Screen::Error(_) => false,
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
            ViewBody::Grid(grid) if grid.empty.is_some() => ScreenKind::Empty,
            ViewBody::Grid(_) => ScreenKind::Grid,
            ViewBody::Detail(_) => ScreenKind::Detail,
            ViewBody::Form(_) => ScreenKind::Form,
            ViewBody::Progress(_) => ScreenKind::Progress,
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
    let send = dispatch.clone();
    intents.on_clicked_outside(move || send(Intent::ClickedOutside));
    let send = dispatch.clone();
    intents.on_filter_chosen(move |id| send(Intent::FilterChosen { id: id.into() }));
    intents.on_panel_filter_edited(move |text| {
        dispatch(Intent::PanelFilterChanged { text: text.into() })
    });
}
