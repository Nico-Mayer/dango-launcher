//! The renderer mounted headless on Slint's testing backend, fed by the real
//! state machine.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use i_slint_backend_testing::ElementHandle;
use slint::platform::{PointerEventButton, WindowEvent};
use slint::{ComponentHandle, LogicalPosition, Model};

use super::generated::{
    GridData, Intents, LauncherWindow, ListData, Panel, ScreenKind, Shell, Tray,
};
use super::render::{self, Renderer};
use crate::protocol::{
    Action, DetailView, FieldKind, FormField, FormView, ListItem, ListView, View, ViewTree,
};
use crate::shell::TrayStatus;
use crate::ui::{Context, Effect, Event, Input, Intent, Launcher, ResultItem, TemplateInspection};

const MARGIN: f32 = crate::platform::LAUNCHER_SHADOW_MARGIN as f32;
/// The SearchBar pill and the gap below it, above the card.
const PILL: f32 = 52.0 + 8.0;
const ACTION_BAR: f32 = 44.0;
const LIST_PADDING: f32 = 8.0;
const ROW: f32 = 44.0;
const LIST_TOP: f32 = MARGIN + PILL + LIST_PADDING;
const LIST_BOTTOM: f32 = MARGIN + 480.0 - ACTION_BAR - LIST_PADDING;

struct Harness {
    ui: LauncherWindow,
    launcher: Rc<RefCell<Launcher>>,
    renderer: Rc<RefCell<Renderer>>,
    intents: Rc<RefCell<Vec<Intent>>>,
    effects: Rc<RefCell<Vec<Effect>>>,
    /// Records intents without feeding them to the state machine, so what is
    /// on screen can only change through a snapshot.
    frozen: Rc<RefCell<bool>>,
    height: Rc<Cell<Option<f32>>>,
}

impl Harness {
    fn new() -> Self {
        i_slint_backend_testing::init_no_event_loop();
        let ui = LauncherWindow::new().unwrap();
        ui.window().set_size(slint::LogicalSize::new(
            720.0 + 2.0 * MARGIN,
            480.0 + 2.0 * MARGIN,
        ));
        ui.show().unwrap();
        let renderer = Rc::new(RefCell::new(Renderer::new(&ui)));
        let launcher = Rc::new(RefCell::new(Launcher::new()));
        let intents = Rc::new(RefCell::new(Vec::new()));
        let effects = Rc::new(RefCell::new(Vec::new()));
        let frozen = Rc::new(RefCell::new(false));
        let height = Rc::new(Cell::new(None));
        let dispatch = {
            let height = height.clone();
            let weak = ui.as_weak();
            let launcher = launcher.clone();
            let renderer = renderer.clone();
            let intents = intents.clone();
            let effects = effects.clone();
            let frozen = frozen.clone();
            Rc::new(move |intent: Intent| {
                intents.borrow_mut().push(intent.clone());
                if *frozen.borrow() {
                    return;
                }
                let fx = launcher.borrow_mut().apply(Input::Intent(intent));
                effects.borrow_mut().extend(fx);
                let snapshot = launcher.borrow().snapshot();
                render::publish(&mut renderer.borrow_mut(), &weak.unwrap(), snapshot, |h| {
                    height.set(Some(h))
                });
            }) as Rc<dyn Fn(Intent)>
        };
        render::connect(&ui, renderer.clone(), dispatch);
        let harness = Self {
            ui,
            launcher,
            renderer,
            intents,
            effects,
            frozen,
            height,
        };
        harness.event(Event::Activated {
            held: false,
            probe: None,
            asked: false,
        });
        harness
    }

    /// Root with the empty-query list opened, as most tests want it.
    fn root(items: Vec<ResultItem>) -> Self {
        let harness = Self::new();
        harness.event(Event::Results {
            query: String::new(),
            items,
            complete: true,
        });
        harness.arrow_down();
        harness.intents.borrow_mut().clear();
        harness.effects.borrow_mut().clear();
        harness
    }

    /// Lets the busy marks' next wait pass, as the shell's timer would.
    fn wake_busy(&self) {
        let pending = self
            .effects
            .borrow()
            .iter()
            .rev()
            .find_map(|effect| match effect {
                Effect::WakeBusyMarks { id, .. } => Some(*id),
                _ => None,
            });
        if let Some(id) = pending {
            self.event(Event::BusyMarksWoke { id });
        }
    }

    fn event(&self, event: Event) {
        let fx = self.launcher.borrow_mut().apply(Input::Event(event));
        self.effects.borrow_mut().extend(fx);
        let snapshot = self.launcher.borrow().snapshot();
        self.publish(snapshot);
    }

    /// Draws a snapshot the state machine could not make on its own.
    fn publish(&self, snapshot: crate::ui::Snapshot) {
        render::publish(
            &mut self.renderer.borrow_mut(),
            &self.ui,
            snapshot,
            |height| self.height.set(Some(height)),
        );
    }

    fn height(&self) -> f32 {
        self.height.get().expect("a height was set")
    }

    fn view(&self, tree: ViewTree) {
        self.view_of("fixture", tree);
    }

    fn view_of(&self, owner: &str, tree: ViewTree) {
        self.event(Event::Rendered {
            owner: owner.into(),
            context: Context {
                title: "Fixture Command".into(),
                icon: None,
            },
            invocation: 1,
            tree: Box::new(tree),
        });
    }

    fn shell(&self) -> Shell<'_> {
        self.ui.global::<Shell>()
    }

    fn panel(&self) -> Panel<'_> {
        self.ui.global::<Panel>()
    }

    fn freeze(&self) {
        *self.frozen.borrow_mut() = true;
    }

    fn intents(&self) -> Vec<Intent> {
        self.intents.borrow().clone()
    }

    fn effects(&self) -> Vec<Effect> {
        self.effects.borrow().clone()
    }

    fn type_text(&self, text: &str) {
        for c in text.chars() {
            self.key(&c.to_string());
        }
    }

    fn key(&self, text: &str) {
        let window = self.ui.window();
        window.dispatch_event(WindowEvent::KeyPressed { text: text.into() });
        window.dispatch_event(WindowEvent::KeyReleased { text: text.into() });
    }

    fn arrow_down(&self) {
        self.key("\u{F701}");
    }

    fn click_at(&self, x: f32, y: f32) {
        let window = self.ui.window();
        let position = LogicalPosition::new(x, y);
        let button = PointerEventButton::Left;
        window.dispatch_event(WindowEvent::PointerMoved { position });
        window.dispatch_event(WindowEvent::PointerPressed { position, button });
        window.dispatch_event(WindowEvent::PointerReleased { position, button });
    }

    fn move_pointer_to(&self, element: &ElementHandle) {
        let position = element.absolute_position();
        let size = element.size();
        self.ui.window().dispatch_event(WindowEvent::PointerMoved {
            position: LogicalPosition::new(
                position.x + size.width / 2.0,
                position.y + size.height / 2.0,
            ),
        });
    }

    fn find(&self, label: &str) -> Option<ElementHandle> {
        ElementHandle::find_by_accessible_label(&self.ui, label).next()
    }

    fn row(&self, title: &str) -> ElementHandle {
        ElementHandle::find_by_accessible_label(&self.ui, title)
            .find(|element| {
                element.accessible_role() == Some(i_slint_backend_testing::AccessibleRole::ListItem)
            })
            .unwrap_or_else(|| panic!("row {title} is drawn"))
    }

    fn query(&self) -> String {
        self.query_of("Search apps and commands")
    }

    fn query_of(&self, placeholder: &str) -> String {
        self.find(placeholder)
            .and_then(|prompt| prompt.accessible_value())
            .unwrap_or_default()
            .to_string()
    }
}

fn result(n: usize) -> ResultItem {
    ResultItem {
        extension_id: "fixture".into(),
        context: Context {
            title: "Fixture".into(),
            icon: None,
        },
        id: format!("result-{n}"),
        title: format!("Result {n}"),
        subtitle: None,
        icon: None,
        actions: vec![Action {
            id: "open".into(),
            title: "Open".into(),
            shortcut: None,
            ..Default::default()
        }],
        match_positions: Vec::new(),
        suggested: false,
        pinned: false,
        answer: false,
        web_search: false,
        hotkey: None,
    }
}

fn results(count: usize) -> Vec<ResultItem> {
    (1..=count).map(result).collect()
}

fn detail_tree(markdown: &str) -> ViewTree {
    ViewTree::new(View::Detail(DetailView {
        markdown: markdown.into(),
        loading: false,
        actions: Vec::new(),
        ..Default::default()
    }))
}

mod theme {
    use slint::language::ColorScheme;
    use slint::Color;

    use super::super::generated::{StdPalette, Theme};
    use super::*;

    #[test]
    fn the_scheme_picks_the_theme() {
        let h = Harness::new();
        let palette = h.ui.global::<StdPalette>();
        let theme = h.ui.global::<Theme>();

        palette.set_color_scheme(ColorScheme::Dark);
        assert!(theme.get_dark());
        assert_eq!(theme.get_popover(), Color::from_rgb_u8(0x1f, 0x20, 0x24));
        assert_eq!(theme.get_foreground(), Color::from_rgb_u8(0xec, 0xec, 0xee));
        assert_eq!(theme.get_accent(), Color::from_rgb_u8(0xa7, 0xb4, 0xc6));

        palette.set_color_scheme(ColorScheme::Light);
        assert!(!theme.get_dark());
        assert_eq!(theme.get_popover(), Color::from_rgb_u8(0xff, 0xff, 0xff));
        assert_eq!(theme.get_foreground(), Color::from_rgb_u8(0x1c, 0x1c, 0x1f));
        assert_eq!(theme.get_accent(), Color::from_rgb_u8(0x5b, 0x6b, 0x82));
    }

    #[test]
    fn selected_rows_and_selected_text_differ() {
        let h = Harness::new();
        let theme = h.ui.global::<Theme>();
        for scheme in [ColorScheme::Dark, ColorScheme::Light] {
            h.ui.global::<StdPalette>().set_color_scheme(scheme);
            assert_ne!(theme.get_selected(), theme.get_selection());
        }
    }
}

mod atoms {
    use super::super::generated::AtomGallery;
    use super::super::keys;
    use super::*;

    #[test]
    fn every_atom_is_found_by_its_label() {
        i_slint_backend_testing::init_no_event_loop();
        let gallery = AtomGallery::new().unwrap();
        gallery.set_chord(keys::panel_label().into());
        gallery.show().unwrap();
        for label in [
            "Icon",
            "Link",
            "2m",
            "Working",
            keys::panel_label().as_str(),
        ] {
            assert!(
                ElementHandle::find_by_accessible_label(&gallery, label)
                    .next()
                    .is_some(),
                "{label} is drawn"
            );
        }
    }

    fn gallery() -> AtomGallery {
        i_slint_backend_testing::init_no_event_loop();
        let gallery = AtomGallery::new().unwrap();
        gallery.set_chord(keys::panel_label().into());
        gallery.show().unwrap();
        gallery
    }

    fn described(gallery: &AtomGallery, description: &str) -> ElementHandle {
        let wanted = description.to_string();
        i_slint_backend_testing::ElementQuery::from_root(gallery)
            .match_descendants()
            .match_predicate(move |element| {
                element.accessible_description().as_deref() == Some(wanted.as_str())
            })
            .find_first()
            .unwrap_or_else(|| panic!("{description} is drawn"))
    }

    #[test]
    fn a_determinate_ring_reports_its_value() {
        let gallery = gallery();
        assert_eq!(
            described(&gallery, "Part done")
                .accessible_value()
                .as_deref(),
            Some("42")
        );
        assert_eq!(
            described(&gallery, "All done")
                .accessible_value()
                .as_deref(),
            Some("100")
        );
        assert_eq!(
            described(&gallery, "Bar").accessible_value().as_deref(),
            Some("62")
        );
    }

    #[test]
    fn an_indeterminate_ring_says_it_is_working() {
        let gallery = gallery();
        let ring = ElementHandle::find_by_accessible_label(&gallery, "Working")
            .find(|element| {
                element.accessible_role()
                    == Some(i_slint_backend_testing::AccessibleRole::ProgressIndicator)
            })
            .expect("the indeterminate ring");
        assert_eq!(ring.accessible_value().as_deref(), Some(""));
    }

    #[test]
    fn an_announced_dot_names_its_state() {
        let gallery = gallery();
        for label in ["Working", "Needs input", "Done", "Failed", "Queued"] {
            assert!(
                ElementHandle::find_by_accessible_label(&gallery, label)
                    .any(|element| element.accessible_role()
                        == Some(i_slint_backend_testing::AccessibleRole::Image)),
                "a dot is labelled {label}"
            );
        }
    }

    #[test]
    fn a_kbd_and_a_tag_are_announced_once() {
        i_slint_backend_testing::init_no_event_loop();
        let gallery = AtomGallery::new().unwrap();
        gallery.set_chord(keys::panel_label().into());
        gallery.show().unwrap();
        for label in ["Link", keys::panel_label().as_str()] {
            assert_eq!(
                ElementHandle::find_by_accessible_label(&gallery, label).count(),
                1,
                "{label} is announced once"
            );
        }
    }

    /// Only the bundled Geist Mono makes these two the same width; a system
    /// fallback would be proportional.
    #[test]
    fn key_chords_are_set_in_the_bundled_mono_face() {
        i_slint_backend_testing::init_no_event_loop();
        let width = |keys: &str| {
            let gallery = AtomGallery::new().unwrap();
            gallery.set_chord(keys.into());
            gallery.show().unwrap();
            let kbd = ElementHandle::find_by_accessible_label(&gallery, keys)
                .next()
                .expect("the kbd");
            kbd.size().width
        };
        assert_eq!(width("iiii"), width("WWWW"));
    }

    #[test]
    fn a_kbd_shows_the_platform_chord() {
        i_slint_backend_testing::init_no_event_loop();
        let gallery = AtomGallery::new().unwrap();
        gallery.set_chord(keys::panel_label().into());
        gallery.show().unwrap();
        let expected = if cfg!(target_os = "macos") {
            "⌘K"
        } else {
            "Ctrl K"
        };
        assert!(ElementHandle::find_by_accessible_label(&gallery, expected)
            .next()
            .is_some());
    }
}

mod protocol_error {
    use super::*;

    #[test]
    fn a_version_1_tree_shows_an_empty_view() {
        let h = Harness::new();
        let mut tree = detail_tree("Working");
        tree.protocol_version = 1;
        h.view(tree);
        assert_eq!(h.shell().get_screen(), ScreenKind::Empty);
        assert!(h.find("This view needs a newer version of Dango").is_some());
        assert!(h.find("Press Escape to go back").is_some());
    }
}

mod search_bar {
    use super::views::{form_tree, list_tree};
    use super::*;

    #[test]
    fn root_shows_no_back_control() {
        let h = Harness::root(results(3));
        assert!(h.find("Go back").is_none());
    }

    #[test]
    fn the_back_control_pops_a_view() {
        let h = Harness::root(results(3));
        h.view(list_tree());
        h.find("Go back")
            .expect("the back control")
            .mock_single_click(PointerEventButton::Left);
        assert_eq!(h.shell().get_screen(), ScreenKind::Root);
        assert!(h.find("Result 1").is_some());
    }

    #[test]
    fn going_back_leaves_the_text_cursor_after_the_query() {
        let h = Harness::root(results(3));
        h.type_text("clip");
        h.view(list_tree());
        h.key("\u{1b}");
        assert_eq!(h.shell().get_screen(), ScreenKind::Root);
        h.type_text("x");
        assert_eq!(h.query(), "clipx");
    }

    #[test]
    fn a_loading_view_shows_the_loading_bar() {
        let h = Harness::root(results(3));
        h.view(detail_tree("Working"));
        assert!(h.find("Loading…").is_none());
        let mut tree = detail_tree("Working");
        if let View::Detail(detail) = &mut tree.view {
            detail.loading = true;
        }
        h.view(tree);
        h.wake_busy();
        let bar = ElementHandle::find_by_accessible_label(&h.ui, "Loading…").find(|element| {
            element.accessible_role()
                == Some(i_slint_backend_testing::AccessibleRole::ProgressIndicator)
        });
        assert!(bar.is_some());
    }

    #[test]
    fn the_loading_bar_sweeps_while_loading() {
        let h = Harness::root(results(3));
        let mut tree = detail_tree("Working");
        if let View::Detail(detail) = &mut tree.view {
            detail.loading = true;
        }
        h.view(tree);
        h.wake_busy();
        let sweep_x = || {
            ElementHandle::find_by_element_id(&h.ui, "Progress::sweep")
                .next()
                .map(|sweep| sweep.absolute_position().x)
        };
        let before = sweep_x().expect("the sweep is shown while loading");
        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(300));
        assert_ne!(sweep_x(), Some(before));

        h.view(detail_tree("Done"));
        assert!(sweep_x().is_some(), "the bar stays its minimum time");
        h.wake_busy();
        assert!(sweep_x().is_none());
    }

    #[test]
    fn a_form_shows_its_title_instead_of_the_query() {
        let h = Harness::root(results(3));
        h.view(form_tree());
        assert!(!h.shell().get_typing());
        assert!(h.find("Fixture Command").is_some());
    }

    #[test]
    fn the_title_starts_right_after_the_back_control() {
        let h = Harness::root(results(3));
        h.view(form_tree());
        let back = h.find("Go back").expect("the back control");
        let title = ElementHandle::find_by_accessible_label(&h.ui, "Fixture Command")
            .min_by(|a, b| a.absolute_position().y.total_cmp(&b.absolute_position().y))
            .expect("the title");
        let gap = title.absolute_position().x - (back.absolute_position().x + back.size().width);
        assert_eq!(gap, 12.0);
    }
}

mod action_bar {
    use super::*;

    fn application() -> ResultItem {
        ResultItem {
            context: Context {
                title: "Applications".into(),
                icon: None,
            },
            ..result(1)
        }
    }

    #[test]
    fn an_application_names_its_extension_and_open() {
        let h = Harness::root(vec![application()]);
        assert!(h.find("Applications").is_some());
        assert!(h.find("Open").is_some());
    }

    #[test]
    fn a_pushed_view_names_its_command() {
        let h = Harness::root(results(3));
        h.event(Event::Rendered {
            owner: "dango.clipboard".into(),
            context: Context {
                title: "Clipboard History".into(),
                icon: None,
            },
            invocation: 1,
            tree: Box::new(views::list_tree()),
        });
        assert!(h.find("Clipboard History").is_some());
    }

    #[test]
    fn a_single_action_shows_no_actions_hint() {
        let h = Harness::root(vec![application()]);
        assert!(h.find("Actions").is_none());
        assert!(!h.shell().get_panel_hint());
    }
}

mod ticker {
    use super::*;
    use crate::extension::ActionOutcome;
    use crate::ui::{ActionRequest, Origin};

    fn ticker(h: &Harness) -> ElementHandle {
        let text = h.shell().get_ticker_text();
        h.find(&text).expect("the ticker")
    }

    fn request() -> ActionRequest {
        ActionRequest {
            origin: Origin::View,
            extension_id: "fixture".into(),
            item_id: "item-1".into(),
            action_id: "pin".into(),
            values: None,
            query: None,
        }
    }

    #[test]
    fn a_busy_ticker_says_what_is_running() {
        let mut command = result(1);
        command.title = "Clipboard History".into();
        command.actions.clear();
        let h = Harness::root(vec![command]);
        h.key(
            "
",
        );
        h.wake_busy();
        assert_eq!(
            h.shell().get_ticker_style(),
            super::super::generated::TickerStyle::Busy
        );
        assert!(h.find("Running Clipboard History…").is_some());
        let spinner = h.find("Working").expect("the busy ticker's spinner");
        assert_eq!(
            spinner.accessible_role(),
            Some(i_slint_backend_testing::AccessibleRole::ProgressIndicator)
        );
    }

    #[test]
    fn a_success_ticker_replaces_the_context() {
        let h = Harness::root(results(3));
        h.view(views::list_tree());
        h.event(Event::ActionFinished {
            request: request(),
            outcome: ActionOutcome::Ticker("Entry pinned".into()),
        });
        assert!(h.shell().get_ticker_shown());
        assert_eq!(
            h.shell().get_ticker_style(),
            super::super::generated::TickerStyle::Success
        );
        assert!(h.find("Entry pinned").is_some());
        assert!(h.find("Fixture Command").is_none());
    }

    #[test]
    fn a_failure_ticker_says_it_failed() {
        let h = Harness::root(results(3));
        h.event(Event::Failed("That command is no longer available".into()));
        assert_eq!(
            h.shell().get_ticker_style(),
            super::super::generated::TickerStyle::Failure
        );
        assert_eq!(
            ticker(&h).accessible_description().as_deref(),
            Some("Failed")
        );
    }

    #[test]
    fn the_longest_failures_fit_at_full_width() {
        let mut item = result(1);
        item.actions.push(Action {
            id: "copy".into(),
            title: "Copy".into(),
            ..Default::default()
        });
        let h = Harness::root(vec![item]);
        for message in [
            "Set up this command's provider in your config file",
            crate::extensions::display::NO_MONITOR,
            "Couldn't verify the update, so it wasn't installed",
            "One keyword can't be the start of another, so choose a different keyword",
        ] {
            h.event(Event::Failed(message.into()));
            assert!(h.shell().get_panel_hint());
            assert!(!h.ui.get_ticker_cut(), "{message} is cut off");
        }
        h.event(Event::Failed("Too long ".repeat(20)));
        assert!(h.ui.get_ticker_cut(), "the measure sees a cut");
    }

    #[test]
    fn typing_reaches_the_search_bar_while_a_ticker_shows() {
        let h = Harness::root(results(3));
        h.event(Event::Failed("That command is no longer available".into()));
        h.type_text("res");
        assert_eq!(h.query(), "res");
    }
}

mod root_search {
    use super::*;

    #[test]
    fn pointer_hover_never_selects_and_keyboard_suppresses_resting_hover() {
        let h = Harness::root(results(5));
        assert_eq!(h.row("Result 1").accessible_item_selected(), Some(true));

        h.move_pointer_to(&h.row("Result 3"));
        assert!(!h.shell().get_pointer_resting());
        assert_eq!(h.row("Result 1").accessible_item_selected(), Some(true));
        assert_eq!(h.row("Result 3").accessible_item_selected(), Some(false));
        assert!(h.intents().is_empty(), "hover sent {:?}", h.intents());

        h.arrow_down();
        assert!(h.shell().get_pointer_resting());
        assert_eq!(h.row("Result 2").accessible_item_selected(), Some(true));

        h.move_pointer_to(&h.row("Result 4"));
        assert!(!h.shell().get_pointer_resting());
    }

    #[test]
    fn clicking_keeps_typed_query_focus() {
        let h = Harness::root(results(5));
        h.type_text("re");
        assert_eq!(h.query(), "re");
        h.event(Event::Results {
            query: "re".into(),
            items: results(5),
            complete: true,
        });

        h.row("Result 2")
            .mock_single_click(PointerEventButton::Left);
        assert!(h.intents().contains(&Intent::Activate {
            id: "result-2".into()
        }));

        h.type_text("s");
        assert_eq!(h.query(), "res");
        assert_eq!(
            h.intents().last(),
            Some(&Intent::QueryChanged { text: "res".into() })
        );
    }

    #[test]
    fn the_first_row_of_a_later_group_is_scrolled_fully_into_view() {
        let mut items = results(12);
        for item in &mut items[..2] {
            item.pinned = true;
        }
        for item in &mut items[2..5] {
            item.suggested = true;
        }
        // Opening the list from the pinned card already stepped to row 2.
        let h = Harness::root(items);
        for _ in 0..4 {
            h.arrow_down();
        }
        let row = h.row("Result 6");
        assert_eq!(row.accessible_item_selected(), Some(true));
        let top = row.absolute_position().y;
        assert!(top >= LIST_TOP, "row starts at {top}");
        assert!(
            top + row.size().height <= LIST_BOTTOM,
            "row ends below the list"
        );

        let heading = h
            .find("Everything else")
            .expect("the group heading is drawn");
        assert!(heading.absolute_position().y >= LIST_TOP);
    }

    #[test]
    fn tab_does_not_take_typing_from_the_search_bar() {
        let h = Harness::root(results(3));
        h.type_text("re");
        h.key("	");
        h.type_text("s");
        assert_eq!(h.query(), "res");
    }

    #[test]
    fn an_answer_to_an_older_keystroke_does_not_overwrite_typing() {
        let h = Harness::root(results(3));
        h.type_text("ab");
        h.event(Event::Results {
            query: "a".into(),
            items: results(2),
            complete: true,
        });
        assert_eq!(h.query(), "ab");
    }
}

mod views {
    use super::*;

    pub(super) fn list_tree() -> ViewTree {
        ViewTree::new(View::List(ListView {
            loading: false,
            empty_state: None,
            items: (1..=3)
                .map(|n| ListItem {
                    id: format!("item-{n}"),
                    title: format!("Item {n}"),
                    subtitle: Some("Subtitle".into()),
                    icon: None,
                    actions: Vec::new(),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        }))
    }

    pub(super) fn form_tree() -> ViewTree {
        let field = |id: &str, kind| FormField {
            id: id.into(),
            label: id.into(),
            kind,
            value: Some(String::new()),
            ..Default::default()
        };
        ViewTree::new(View::Form(FormView {
            item_id: None,
            fields: vec![
                field("name", FieldKind::Text),
                field("enabled", FieldKind::Toggle),
                field("template", FieldKind::Template),
            ],
            actions: vec![Action {
                id: "save".into(),
                title: "Save".into(),
                shortcut: None,
                ..Default::default()
            }],
        }))
    }

    #[test]
    fn a_pushed_list_renders_its_items() {
        let h = Harness::root(results(3));
        h.view(list_tree());
        assert_eq!(h.shell().get_screen(), ScreenKind::List);
        assert!(h.find("Search").is_some());
        assert_eq!(h.row("Item 1").accessible_item_selected(), Some(true));
        assert!(h.find("Item 3").is_some());
    }

    #[test]
    fn a_detail_renders_its_text() {
        let h = Harness::root(results(3));
        h.view(detail_tree("Plain words"));
        assert_eq!(h.shell().get_screen(), ScreenKind::Detail);
        assert!(h.find("Plain words").is_some());
    }

    #[test]
    fn a_form_renders_every_field() {
        let h = Harness::root(results(3));
        h.view(form_tree());
        assert_eq!(h.shell().get_screen(), ScreenKind::Form);
        for label in ["name", "enabled", "template"] {
            assert!(h.find(label).is_some(), "{label} is drawn");
        }
    }

    #[test]
    fn a_template_error_is_the_fields_accessible_description() {
        let h = Harness::root(results(3));
        h.view(form_tree());
        h.ui.global::<Intents>()
            .invoke_field_edited("template".into(), "{{ broken".into());
        h.event(Event::TemplateInspected {
            field: "template".into(),
            source: "{{ broken".into(),
            inspection: TemplateInspection {
                arguments: Vec::new(),
                error: Some("The template doesn't close".into()),
            },
        });
        let described = ElementHandle::find_by_accessible_label(&h.ui, "template")
            .filter_map(|field| field.accessible_description())
            .any(|description| description == "The template doesn't close");
        assert!(described);
    }
}

mod list_view {
    use slint::language::ColorScheme;

    use super::super::generated::{StdPalette, Theme};
    use super::*;
    use crate::protocol::{Accessory, ItemDetail};

    fn tree(items: Vec<ListItem>, detail_pane: bool) -> ViewTree {
        ViewTree::new(View::List(ListView {
            detail_pane,
            items,
            ..Default::default()
        }))
    }

    fn item(n: usize) -> ListItem {
        ListItem {
            id: format!("item-{n}"),
            title: format!("Item {n}"),
            ..Default::default()
        }
    }

    struct NoPlatform;

    impl crate::platform::LauncherWindow for NoPlatform {
        fn will_show(&mut self) {}
        fn did_show(&mut self, _window: crate::platform::RawWindowHandle) {}
        fn position_on_active_display(
            &self,
            _window: crate::platform::RawWindowHandle,
            _display: crate::config::LauncherDisplay,
        ) {
        }
        fn restore_previous_focus(&mut self) {}
    }

    fn hide(h: &Harness) {
        use crate::shell::Shell as _;

        super::super::RENDERER.with(|slot| *slot.borrow_mut() = Some(h.renderer.clone()));
        let shell = super::super::SlintShell {
            window: h.ui.as_weak(),
            tray: slint::Weak::default(),
            platform: std::sync::Arc::new(std::sync::Mutex::new(Box::new(NoPlatform))),
            attached: Default::default(),
            notice: super::super::Notice {
                window: slint::Weak::default(),
                platform: std::sync::Arc::new(std::sync::Mutex::new(Box::new(
                    super::notice::NoPlatform,
                ))),
                attached: Default::default(),
                display: Default::default(),
            },
        };
        shell.hide(false);
        super::super::RENDERER.with(|slot| *slot.borrow_mut() = None);
    }

    #[test]
    fn hiding_lets_go_of_file_images() {
        let h = Harness::root(results(3));
        let icon = concat!(env!("CARGO_MANIFEST_DIR"), "/icons/32x32.png");
        h.view(tree(
            vec![ListItem {
                icon: Some(icon.into()),
                ..item(1)
            }],
            false,
        ));
        let rows = || h.ui.global::<ListData>().get_rows().row_count();
        assert_eq!(rows(), 1);
        assert!(h.renderer.borrow().holds_images());

        hide(&h);

        assert_eq!(rows(), 0);
        assert!(!h.renderer.borrow().holds_images());
    }

    #[test]
    fn the_launcher_is_revealed_without_a_show() {
        let h = Harness::root(results(3));
        assert!(h.ui.global::<Shell>().get_revealed());
    }

    #[test]
    fn a_hide_unreveals_until_the_next_activation_is_drawn() {
        let h = Harness::root(results(3));
        hide(&h);
        assert!(!h.ui.global::<Shell>().get_revealed());
        assert!(!h.renderer.borrow_mut().take_reveal());

        h.event(Event::Activated {
            held: true,
            probe: None,
            asked: true,
        });
        assert!(h.renderer.borrow_mut().take_reveal());
        assert!(!h.renderer.borrow_mut().take_reveal(), "taken once");
        assert!(!h.ui.global::<Shell>().get_revealed());

        h.renderer.borrow_mut().reveal(&h.ui);
        assert!(h.ui.global::<Shell>().get_revealed());
    }

    #[test]
    fn the_launcher_slides_in_only_on_the_way_in() {
        let h = Harness::root(results(3));
        let pill_top = || h.find("Search bar").unwrap().absolute_position().y;
        let offset = h.ui.global::<Theme>().get_reveal_offset();
        assert_eq!(pill_top(), MARGIN);

        hide(&h);
        assert_eq!(pill_top(), MARGIN - offset, "hiding moves it at once");

        h.renderer.borrow_mut().reveal(&h.ui);
        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(500));
        assert_eq!(pill_top(), MARGIN);
    }

    #[test]
    fn typing_works_before_the_reveal() {
        let h = Harness::new();
        hide(&h);
        h.event(Event::Activated {
            held: false,
            probe: None,
            asked: true,
        });
        h.type_text("fi");
        assert_eq!(h.query(), "fi");
    }

    #[test]
    fn a_row_is_forty_four_pixels_high() {
        let h = Harness::root(results(3));
        assert_eq!(h.row("Result 1").size().height, ROW);
    }

    #[test]
    fn a_fourth_accessory_is_dropped() {
        let accessories = (1..=4)
            .map(|n| Accessory::Text {
                text: format!("Accessory {n}"),
            })
            .collect();
        let h = Harness::root(results(1));
        h.view(tree(
            vec![ListItem {
                accessories,
                ..item(1)
            }],
            false,
        ));
        assert!(h.find("Accessory 3").is_some());
        assert!(h.find("Accessory 4").is_none());
    }

    fn progress_values(h: &Harness) -> Vec<String> {
        i_slint_backend_testing::ElementQuery::from_root(&h.ui)
            .match_descendants()
            .match_accessible_role(i_slint_backend_testing::AccessibleRole::ProgressIndicator)
            .find_all()
            .iter()
            .filter_map(|ring| ring.accessible_value().map(|value| value.to_string()))
            .collect()
    }

    #[test]
    fn progress_and_status_accessories_are_shown() {
        let h = Harness::root(results(1));
        h.view(tree(
            vec![ListItem {
                accessories: vec![
                    Accessory::Progress { value: Some(0.42) },
                    Accessory::Status {
                        status: crate::protocol::Status::Working,
                    },
                ],
                ..item(1)
            }],
            false,
        ));
        assert!(progress_values(&h).contains(&"42".to_string()));
        assert!(h.find("Working").is_some());
    }

    #[test]
    fn a_progress_value_out_of_range_is_clamped() {
        let h = Harness::root(results(1));
        h.view(tree(
            vec![ListItem {
                accessories: vec![Accessory::Progress { value: Some(1.5) }],
                ..item(1)
            }],
            false,
        ));
        assert!(progress_values(&h).contains(&"100".to_string()));
        assert!(h.find("Item 1").is_some());
    }

    fn loading(items: Vec<ListItem>) -> ViewTree {
        ViewTree::new(View::List(ListView {
            loading: true,
            items,
            ..Default::default()
        }))
    }

    fn skeleton_shown(h: &Harness) -> bool {
        h.ui.global::<ListData>().get_skeleton()
            && ElementHandle::find_by_element_type_name(&h.ui, "Skeleton")
                .next()
                .is_some()
    }

    #[test]
    fn a_cold_loading_list_shows_skeleton_rows_at_full_height() {
        let h = Harness::root(results(1));
        h.view(loading(Vec::new()));
        assert!(skeleton_shown(&h));
        assert_eq!(h.shell().get_screen(), ScreenKind::List);
        assert_eq!(h.height(), 480.0);
    }

    #[test]
    fn the_skeleton_keeps_to_the_list_beside_a_detail_pane() {
        let h = Harness::root(results(1));
        h.view(ViewTree::new(View::List(ListView {
            loading: true,
            detail_pane: true,
            ..Default::default()
        })));
        let skeleton = ElementHandle::find_by_element_type_name(&h.ui, "Skeleton")
            .next()
            .expect("the skeleton");
        assert!(skeleton.size().width <= 720.0 * 2.0 / 5.0 + 0.5);
    }

    #[test]
    fn arriving_items_replace_the_skeleton() {
        let h = Harness::root(results(1));
        h.view(loading(Vec::new()));
        h.view(loading(vec![item(1)]));
        assert!(!skeleton_shown(&h));
        assert!(h.find("Item 1").is_some());
    }

    #[test]
    fn a_refreshing_list_filtered_to_nothing_shows_no_skeleton() {
        let h = Harness::root(results(1));
        h.view(loading(vec![item(1)]));
        h.type_text("zzz");
        assert!(h.find("Item 1").is_none());
        assert!(!skeleton_shown(&h));
    }

    #[test]
    fn a_pushed_list_of_five_hundred_renders_fifty_rows() {
        let h = Harness::root(results(1));
        h.view(tree((1..=500).map(item).collect(), false));
        let rows = h.ui.global::<ListData>().get_rows();
        assert_eq!(rows.row_count(), 50);
        assert_eq!(rows.row_data(49).unwrap().id, "item-50");
    }

    #[test]
    fn hover_is_weaker_than_selected() {
        let h = Harness::new();
        let theme = h.ui.global::<Theme>();
        for scheme in [ColorScheme::Dark, ColorScheme::Light] {
            h.ui.global::<StdPalette>().set_color_scheme(scheme);
            assert!(theme.get_hover().alpha() < theme.get_selected().alpha());
        }
    }

    #[test]
    fn no_results_shows_the_empty_view_with_the_query() {
        let h = Harness::root(results(3));
        h.type_text("zzz");
        h.event(Event::Results {
            query: "zzz".into(),
            items: Vec::new(),
            complete: true,
        });
        assert_eq!(h.shell().get_screen(), ScreenKind::Empty);
        assert!(h.find("No results for “zzz”").is_some());
    }

    fn web_search(query: &str) -> ResultItem {
        ResultItem {
            id: format!("web:duckduckgo:{query}"),
            title: format!("Search DuckDuckGo for “{query}”"),
            web_search: true,
            ..result(0)
        }
    }

    #[test]
    fn a_web_search_sits_under_its_heading_below_the_results() {
        let h = Harness::root(results(3));
        h.type_text("fire");
        h.event(Event::Results {
            query: "fire".into(),
            items: vec![result(1), web_search("fire")],
            complete: true,
        });

        let ranked = h.row("Result 1").absolute_position().y;
        let heading = h
            .find("Search the web")
            .expect("the web search heading is drawn")
            .absolute_position()
            .y;
        let web = h.row("Search DuckDuckGo for “fire”").absolute_position().y;
        assert!(
            ranked < heading && heading < web,
            "{ranked} {heading} {web}"
        );
    }

    #[test]
    fn a_web_search_alone_is_selected_and_opens() {
        let h = Harness::root(results(3));
        h.type_text("zzqx");
        h.event(Event::Results {
            query: "zzqx".into(),
            items: vec![web_search("zzqx")],
            complete: true,
        });

        assert!(h.find("No results for “zzqx”").is_none());
        let row = h.row("Search DuckDuckGo for “zzqx”");
        assert_eq!(row.accessible_item_selected(), Some(true));
        assert!(h.find("Open").is_some());
    }

    #[test]
    fn arrow_down_changes_the_detail_pane() {
        let detailed = |n: usize, body: &str| ListItem {
            detail: Some(ItemDetail {
                markdown: body.into(),
                ..Default::default()
            }),
            ..item(n)
        };
        let h = Harness::root(results(1));
        h.view(tree(
            vec![detailed(1, "First body"), detailed(2, "Second body")],
            true,
        ));
        let first = h.find("First body").expect("the first detail");
        assert!(first.absolute_position().x >= MARGIN + 720.0 * 2.0 / 5.0);
        assert!(h.row("Item 1").size().width <= 720.0 * 2.0 / 5.0);
        h.arrow_down();
        assert!(h.find("Second body").is_some());
        assert!(h.find("First body").is_none());
    }
}

mod detail {
    use super::super::generated::DetailData;
    use super::*;
    use crate::protocol::{ItemDetail, Metadata};

    const IMAGE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/icons/32x32.png");

    fn source() -> Vec<Metadata> {
        vec![Metadata {
            label: "Source".into(),
            value: "Terminal".into(),
        }]
    }

    fn metadata_bottom(h: &Harness, text: &str) -> (f32, f32) {
        h.view(ViewTree::new(View::Detail(DetailView {
            text: Some(text.into()),
            metadata: source(),
            ..Default::default()
        })));
        let value = h.find("Terminal").expect("the metadata is drawn");
        (
            value.absolute_position().y + value.size().height,
            MARGIN + 480.0 - ACTION_BAR,
        )
    }

    #[test]
    fn a_short_body_leaves_the_metadata_at_the_bottom() {
        let h = Harness::root(results(1));
        let (bottom, detail_bottom) = metadata_bottom(&h, "One line");
        let body = h.find("One line").unwrap();
        assert!(
            bottom > detail_bottom - ACTION_BAR,
            "{bottom} near {detail_bottom}"
        );
        assert!(bottom - body.absolute_position().y > 200.0);
    }

    #[test]
    fn a_long_body_scrolls_above_the_metadata() {
        let h = Harness::root(results(1));
        let long = (1..=200).map(|n| format!("line {n}\n")).collect::<String>();
        let (bottom, detail_bottom) = metadata_bottom(&h, &long);
        assert!(bottom <= detail_bottom, "{bottom} within {detail_bottom}");
        assert!(bottom > detail_bottom - ACTION_BAR);
    }

    #[test]
    fn plain_text_is_shown_as_written_in_the_monospace_face() {
        let h = Harness::root(results(1));
        h.view(ViewTree::new(View::Detail(DetailView {
            markdown: "**ignored**".into(),
            text: Some("# not a heading".into()),
            ..Default::default()
        })));
        assert!(h.find("# not a heading").is_some());
        assert!(h.find("**ignored**").is_none());
        assert!(h.ui.global::<DetailData>().get_plain());
    }

    #[test]
    fn the_image_is_shown_above_the_body() {
        let h = Harness::root(results(1));
        h.view(ViewTree::new(View::Detail(DetailView {
            image: Some(IMAGE.into()),
            text: Some("Body".into()),
            metadata: source(),
            ..Default::default()
        })));
        let image = h.find("Image").expect("the image is drawn");
        let body = h.find("Body").expect("the body is drawn");
        assert!(image.absolute_position().y < body.absolute_position().y);
        assert_eq!(image.size().width, 32.0);
    }

    #[test]
    fn a_missing_image_still_shows_body_and_metadata() {
        let h = Harness::root(results(1));
        let missing = std::env::temp_dir().join("dango-no-such-detail-image.png");
        h.view(ViewTree::new(View::Detail(DetailView {
            image: Some(missing.to_string_lossy().into_owned()),
            text: Some("Body".into()),
            metadata: source(),
            ..Default::default()
        })));
        assert!(h.find("Image").is_none());
        assert!(h.find("Body").is_some());
        assert!(h.find("Terminal").is_some());
    }

    #[test]
    fn arrow_down_changes_the_clipboard_pane() {
        use crate::extensions::clipboard::{history_view, Entry, Kind, PrimaryAction};

        let text = |id: &str, text: &str, source: &str| Entry {
            id: id.into(),
            kind: Kind::Text,
            text: Some(text.into()),
            path: None,
            thumbnail: None,
            source: Some(source.into()),
            size: None,
            characters: Some(text.chars().count() as u32),
            lines: Some(1),
            copied_at: 0,
        };
        let h = Harness::root(results(1));
        h.view(history_view(
            &[
                text("a", "# first entry", "Notepad"),
                text("b", "second entry", "Code"),
            ],
            PrimaryAction::Paste,
            time::OffsetDateTime::now_utc(),
            &crate::icon_cache::IconCache::in_temp(),
        ));
        assert!(h.find("Notepad").is_some());
        assert!(h.ui.global::<DetailData>().get_plain());
        h.arrow_down();
        assert!(h.find("Code").is_some());
        assert!(h.find("Notepad").is_none());
    }

    #[test]
    fn a_keystroke_keeping_the_selection_loads_no_new_image() {
        let h = Harness::root(results(1));
        let item = |n: usize| ListItem {
            id: format!("item-{n}"),
            title: format!("Item {n}"),
            detail: Some(ItemDetail {
                image: Some(IMAGE.into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        h.view(ViewTree::new(View::List(ListView {
            detail_pane: true,
            items: vec![item(1), item(2)],
            ..Default::default()
        })));
        let decoded = h.renderer.borrow().decoded_images();
        h.type_text("it");
        assert_eq!(h.renderer.borrow().decoded_images(), decoded);
        assert!(h.find("Image").is_some());
    }

    #[test]
    fn metadata_follows_the_body_in_order() {
        let h = Harness::root(results(1));
        let mut tree = detail_tree("Some **copied** text");
        if let View::Detail(detail) = &mut tree.view {
            detail.metadata = vec![
                Metadata {
                    label: "Source".into(),
                    value: "Terminal".into(),
                },
                Metadata {
                    label: "Copied".into(),
                    value: "Today, 14:31".into(),
                },
            ];
        }
        h.view(tree);
        let y = |label: &str| {
            h.find(label)
                .unwrap_or_else(|| panic!("{label} is drawn"))
                .absolute_position()
                .y
        };
        assert!(y("Some **copied** text") < y("Source"));
        assert!(y("Source") < y("Copied"));
        assert!(h.find("Today, 14:31").is_some());
    }
}

mod form {
    use super::*;
    use crate::protocol::FieldOption;
    use i_slint_backend_testing::AccessibleRole;

    fn field(id: &str, label: &str, kind: FieldKind) -> FormField {
        FormField {
            id: id.into(),
            label: label.into(),
            kind,
            value: None,
            ..Default::default()
        }
    }

    fn every_kind() -> ViewTree {
        ViewTree::new(View::Form(FormView {
            item_id: None,
            fields: vec![
                field("name", "Name", FieldKind::Text),
                field("secret", "Secret", FieldKind::Password),
                field("notes", "Notes", FieldKind::Textarea),
                field("break", "", FieldKind::Separator),
                field("enabled", "Enabled", FieldKind::Toggle),
                FormField {
                    options: vec![
                        FieldOption {
                            id: "work".into(),
                            title: "Work".into(),
                        },
                        FieldOption {
                            id: "personal".into(),
                            title: "Personal".into(),
                        },
                    ],
                    ..field("folder", "Folder", FieldKind::Dropdown)
                },
            ],
            actions: vec![Action {
                id: "save".into(),
                title: "Save".into(),
                ..Default::default()
            }],
        }))
    }

    fn role(h: &Harness, label: &str) -> Option<AccessibleRole> {
        ElementHandle::find_by_accessible_label(&h.ui, label)
            .find_map(|element| element.accessible_role())
    }

    #[test]
    fn every_control_is_found_by_its_label() {
        let h = Harness::root(results(1));
        h.view(every_kind());
        assert_eq!(role(&h, "Name"), Some(AccessibleRole::TextInput));
        assert_eq!(role(&h, "Secret"), Some(AccessibleRole::TextInput));
        assert_eq!(role(&h, "Notes"), Some(AccessibleRole::TextInput));
        assert_eq!(role(&h, "Enabled"), Some(AccessibleRole::Checkbox));
        assert_eq!(role(&h, "Folder"), Some(AccessibleRole::Combobox));
        let folder = h.find("Folder").unwrap();
        assert_eq!(folder.accessible_value().as_deref(), Some("Work"));
    }

    #[test]
    fn tab_moves_to_the_next_control() {
        let h = Harness::root(results(1));
        h.view(every_kind());
        h.type_text("a");
        h.key("\t");
        h.type_text("b");
        let edits: Vec<Intent> = h
            .intents()
            .into_iter()
            .filter(|intent| matches!(intent, Intent::FieldChanged { .. }))
            .collect();
        assert_eq!(
            edits,
            [
                Intent::FieldChanged {
                    id: "name".into(),
                    value: "a".into()
                },
                Intent::FieldChanged {
                    id: "secret".into(),
                    value: "b".into()
                },
            ]
        );
    }

    #[test]
    fn leaving_a_form_from_a_textarea_gives_enter_back() {
        let h = Harness::root(results(1));
        h.view(every_kind());
        h.key("\t");
        h.key("\t");
        assert!(h.shell().get_multiline_focused());
        h.key("\u{1b}");
        assert_eq!(h.shell().get_screen(), ScreenKind::Root);
        h.intents.borrow_mut().clear();
        h.key("\n");
        assert_eq!(
            h.intents(),
            [Intent::Key {
                chord: crate::ui::Chord::plain("Enter")
            }]
        );
    }

    #[test]
    fn choosing_a_dropdown_option_edits_the_field() {
        let h = Harness::root(results(1));
        h.view(every_kind());
        h.find("Folder").unwrap().invoke_accessible_default_action();
        h.find("Personal")
            .expect("the option")
            .invoke_accessible_default_action();
        assert!(h.intents().contains(&Intent::FieldChanged {
            id: "folder".into(),
            value: "personal".into()
        }));
        assert_eq!(
            h.find("Folder").unwrap().accessible_value().as_deref(),
            Some("Personal")
        );
    }
}

mod grid {
    use super::*;
    use crate::protocol::{GridItem, GridView, TileContent};

    fn grid_tree(count: usize) -> ViewTree {
        ViewTree::new(View::Grid(GridView {
            columns: 5,
            aspect: 1.0,
            items: (1..=count)
                .map(|n| GridItem {
                    id: format!("tile-{n}"),
                    title: format!("Tile {n}"),
                    content: TileContent::Glyph {
                        glyph: "★".into()
                    },
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        }))
    }

    #[test]
    fn twelve_tiles_fill_three_rows_of_five() {
        let h = Harness::root(results(1));
        h.view(grid_tree(12));
        assert_eq!(h.shell().get_screen(), ScreenKind::Grid);
        let place = |n: usize| h.row(&format!("Tile {n}")).absolute_position();
        for n in 2..=5 {
            assert_eq!(place(n).y, place(1).y, "tile {n} is in the first row");
        }
        assert!(place(6).y > place(1).y);
        assert_eq!(place(6).x, place(1).x);
        assert!(place(11).y > place(6).y);
        assert_eq!(place(12).y, place(11).y);
        assert_eq!(h.row("Tile 1").accessible_item_selected(), Some(true));
    }

    #[test]
    fn arrow_left_moves_the_selection_not_the_text_cursor() {
        let h = Harness::root(results(1));
        h.view(grid_tree(12));
        h.type_text("ti");
        h.key("\u{F703}");
        assert_eq!(h.row("Tile 2").accessible_item_selected(), Some(true));
        h.key("\u{F702}");
        assert_eq!(h.row("Tile 1").accessible_item_selected(), Some(true));
        h.type_text("l");
        assert_eq!(h.query_of("Search"), "til");
    }

    #[test]
    fn two_rows_of_tiles_set_the_window_height() {
        let h = Harness::root(results(1));
        h.view(grid_tree(7));
        let content = h.ui.global::<GridData>().get_content_height();
        assert!((h.height() - (PILL + content + ACTION_BAR)).abs() < 0.01);
        let last = h.row("Tile 7");
        let bottom = last.absolute_position().y + last.size().height;
        assert!(
            bottom <= MARGIN + h.height() - ACTION_BAR,
            "the last row is not cut off"
        );
    }
}

mod games {
    use super::*;
    use crate::extensions::games::fake::FakeSource;
    use crate::extensions::games::{self, GameSource};

    #[test]
    fn typing_filters_the_covers_and_enter_asks_games_to_launch() {
        let source = FakeSource::with(&[(291550, "Brawlhalla"), (1091500, "Cyberpunk 2077")]);
        let h = Harness::root(results(1));
        h.view_of(games::EXTENSION_ID, games::grid(&source.games()));
        assert_eq!(h.shell().get_screen(), ScreenKind::Grid);

        h.type_text("cyber");
        h.row("Cyberpunk 2077");
        assert!(h.find("Brawlhalla").is_none());

        h.key("\n");
        let Some(request) = h.effects().into_iter().find_map(|effect| match effect {
            Effect::RunAction(request) => Some(request),
            _ => None,
        }) else {
            panic!("Enter runs the tile's action");
        };
        assert_eq!(
            (
                request.extension_id.as_str(),
                request.item_id.as_str(),
                request.action_id.as_str()
            ),
            (games::EXTENSION_ID, "1091500", games::ACTION_LAUNCH)
        );
    }
}

mod quicklink_browser {
    use std::sync::Arc;

    use super::*;
    use crate::extension::{ActionOutcome, Extension, FormValues};
    use crate::extensions::snippets::{self, Kind, Records, SnippetsExtension};
    use crate::ui::Chord;

    #[test]
    fn picking_firefox_in_the_form_stores_it() {
        let dir = std::env::temp_dir().join(format!("dango-ui-ql-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let records = Records::open(&dir, Kind::Quicklink).0;
        let id = records
            .create("Docs", "https://example.com/docs", None)
            .unwrap();
        let extension = SnippetsExtension::new(records.clone(), None, None, Arc::new(Vec::new));
        let ActionOutcome::Replaced(form) =
            extension.perform_action(&id, snippets::ACTION_EDIT, &FormValues::new())
        else {
            panic!("expected the edit form");
        };
        let h = Harness::root(results(1));
        h.view_of(snippets::QUICKLINKS_ID, *form);

        h.find("Browser")
            .unwrap()
            .invoke_accessible_default_action();
        h.find("Firefox")
            .expect("the option")
            .invoke_accessible_default_action();
        let effects = h.launcher.borrow_mut().apply(Input::Intent(Intent::Key {
            chord: Chord::command("Enter"),
        }));

        let Some(request) = effects.into_iter().find_map(|effect| match effect {
            Effect::RunAction(request) => Some(request),
            _ => None,
        }) else {
            panic!("the form is submitted");
        };
        extension.perform_action(
            &request.item_id,
            &request.action_id,
            &request.values.unwrap(),
        );
        assert_eq!(
            records.get(&id).unwrap().browser.as_deref(),
            Some("firefox")
        );
    }
}

mod filter_dropdown {
    use super::*;
    use crate::protocol::{FieldOption, Filter};

    #[test]
    fn choosing_text_only_keeps_the_query_and_shows_it() {
        let option = |id: &str, title: &str| FieldOption {
            id: id.into(),
            title: title.into(),
        };
        let mut tree = views::list_tree();
        if let View::List(list) = &mut tree.view {
            list.filter = Some(Filter {
                options: vec![option("all", "All types"), option("text", "Text only")],
                selected: "all".into(),
            });
        }
        let h = Harness::root(results(1));
        h.view(tree);
        h.type_text("it");
        let dropdown = h.find("Filter results").expect("the filter dropdown");
        assert_eq!(dropdown.accessible_value().as_deref(), Some("All types"));
        dropdown.invoke_accessible_default_action();
        h.find("Text only")
            .expect("the option")
            .invoke_accessible_default_action();
        assert!(h
            .intents()
            .contains(&Intent::FilterChosen { id: "text".into() }));
        assert_eq!(
            h.find("Filter results")
                .unwrap()
                .accessible_value()
                .as_deref(),
            Some("Text only")
        );
        assert_eq!(h.query_of("Search"), "it");
    }
}

mod bare_pill {
    use super::*;

    fn bare() -> Harness {
        let h = Harness::new();
        h.event(Event::Results {
            query: String::new(),
            items: results(5),
            complete: true,
        });
        h
    }

    #[test]
    fn the_launcher_opens_as_the_pill_alone() {
        let h = bare();
        assert!(!h.shell().get_card_shown());
        assert_eq!(h.height(), 52.0);
        assert!(h.find("Result 1").is_none());
        assert!(h.find("Actions").is_none());
    }

    #[test]
    fn the_first_keystroke_keeps_the_pill_until_results_land() {
        let h = bare();
        h.arrow_down();
        h.type_text("r");
        assert_eq!(h.height(), 52.0);
        assert!(!h.shell().get_card_shown());
    }

    #[test]
    fn typing_on_after_no_results_keeps_the_full_height() {
        let h = bare();
        h.type_text("x");
        h.event(Event::Results {
            query: "x".into(),
            items: Vec::new(),
            complete: true,
        });
        assert_eq!(h.height(), 480.0);
        h.type_text("y");
        assert_eq!(h.height(), 480.0);
        assert!(h.shell().get_card_shown());
    }

    #[test]
    fn the_first_keystroke_opens_the_card() {
        let h = bare();
        h.type_text("r");
        h.event(Event::Results {
            query: "r".into(),
            items: results(3),
            complete: true,
        });
        assert!(h.shell().get_card_shown());
        assert_eq!(
            h.height(),
            PILL + LIST_PADDING + 3.0 * ROW + LIST_PADDING + ACTION_BAR
        );
        assert!(h.find("Result 1").is_some());
    }

    #[test]
    fn a_ticker_shows_at_the_pills_end_without_resizing() {
        let h = bare();
        h.event(Event::Failed("Couldn't paste".into()));
        let ticker = h.find("Couldn't paste").expect("the ticker on the pill");
        let pill = h.find("Search bar").expect("the pill");
        assert!(ticker.absolute_position().y < MARGIN + pill.size().height);
        assert_eq!(ticker.accessible_description().as_deref(), Some("Failed"));
        assert_eq!(h.height(), 52.0);
    }

    #[test]
    fn opening_the_card_moves_the_ticker_into_the_action_bar() {
        let h = bare();
        h.event(Event::Task(Box::new(crate::tasks::TaskState {
            id: 1,
            owner: "dango.updates".into(),
            title: "Dango 0.5.0".into(),
            line: "Downloading Dango 0.5.1".into(),
            status: crate::tasks::TaskStatus::Running,
            progress: None,
            steps: Vec::new(),
            subtitle: String::new(),
            detail: String::new(),
            cancel: None,
            retry: None,
            show: None,
            item_id: "update".into(),
        })));
        h.wake_busy();
        let pill = h.find("Search bar").expect("the pill").size().height;
        let on_pill = h.find("Downloading Dango 0.5.1").expect("the ticker");
        assert!(on_pill.absolute_position().y < MARGIN + pill);
        h.arrow_down();
        let in_bar = h.find("Downloading Dango 0.5.1").expect("the ticker");
        assert!(in_bar.absolute_position().y > MARGIN + pill);
    }
}

mod window_height {
    use super::*;

    #[test]
    fn the_card_sits_below_the_pill_across_a_gap() {
        let h = Harness::root(results(3));
        let pill = h.find("Search bar").expect("the pill");
        let card = h.find("Results").expect("the card");
        assert_eq!(pill.size().height, 52.0);
        assert_eq!(
            card.absolute_position().y,
            pill.absolute_position().y + pill.size().height + 8.0
        );
    }

    #[test]
    fn a_click_in_the_gap_is_a_click_outside() {
        let h = Harness::root(results(3));
        h.click_at(MARGIN + 360.0, MARGIN + 52.0 + 4.0);
        assert_eq!(h.intents(), [Intent::ClickedOutside]);
    }

    #[test]
    fn a_click_in_the_margin_beside_the_card_is_a_click_outside() {
        let h = Harness::root(results(3));
        let card = h.find("Results").expect("the card");
        h.click_at(MARGIN / 2.0, card.absolute_position().y + 20.0);
        assert_eq!(h.intents(), [Intent::ClickedOutside]);
    }

    #[test]
    fn the_pill_and_the_card_sit_inside_the_margin() {
        let h = Harness::root(results(3));
        for label in ["Search bar", "Results"] {
            let part = h.find(label).expect(label);
            assert_eq!(part.absolute_position().x, MARGIN);
            assert_eq!(part.size().width, 720.0);
        }
        assert_eq!(h.find("Search bar").unwrap().absolute_position().y, MARGIN);
    }

    #[test]
    fn a_click_on_the_pill_or_an_empty_part_of_the_card_stays_inside() {
        let h = Harness::root(results(3));
        h.click_at(MARGIN + 360.0, MARGIN + 26.0);
        let card = h.find("Results").expect("the card");
        h.click_at(
            MARGIN + 360.0,
            card.absolute_position().y + LIST_PADDING / 2.0,
        );
        assert!(!h.intents().contains(&Intent::ClickedOutside));
    }

    #[test]
    fn the_card_fits_its_rows_when_the_window_keeps_its_size() {
        let h = Harness::root(results(3));
        let card = h.find("Results").expect("the card");
        assert_eq!(
            card.size().height,
            LIST_PADDING + 3.0 * ROW + LIST_PADDING + ACTION_BAR
        );
        assert!(PILL + card.size().height < 480.0);
    }

    #[test]
    fn three_results_make_a_short_window() {
        let h = Harness::root(results(3));
        assert_eq!(
            h.height(),
            PILL + LIST_PADDING + 3.0 * ROW + LIST_PADDING + ACTION_BAR
        );
    }

    #[test]
    fn forty_results_stop_at_the_maximum() {
        let h = Harness::root(results(40));
        assert_eq!(h.height(), 480.0);
    }

    #[test]
    fn a_partial_answer_does_not_shrink_the_window() {
        let h = Harness::root(results(5));
        h.type_text("r");
        h.event(Event::Results {
            query: "r".into(),
            items: results(5),
            complete: true,
        });
        let full = h.height();
        h.type_text("e");
        h.event(Event::Results {
            query: "re".into(),
            items: results(2),
            complete: false,
        });
        assert_eq!(h.height(), full);
        h.event(Event::Results {
            query: "re".into(),
            items: results(8),
            complete: false,
        });
        assert_eq!(
            h.height(),
            PILL + LIST_PADDING + 8.0 * ROW + LIST_PADDING + ACTION_BAR
        );
        h.event(Event::Results {
            query: "re".into(),
            items: results(2),
            complete: true,
        });
        assert_eq!(
            h.height(),
            PILL + LIST_PADDING + 2.0 * ROW + LIST_PADDING + ACTION_BAR
        );
    }

    #[test]
    fn a_detail_takes_the_full_height() {
        let h = Harness::root(results(3));
        h.view(detail_tree("Body"));
        assert_eq!(h.height(), 480.0);
    }
}

mod action_panel {
    use super::*;

    fn actions_button(h: &Harness) -> ElementHandle {
        ElementHandle::find_by_accessible_label(&h.ui, "Actions")
            .find(|element| {
                element.accessible_role() == Some(i_slint_backend_testing::AccessibleRole::Button)
            })
            .expect("the Actions button")
    }

    fn two_actions(count: usize) -> Vec<ResultItem> {
        let mut items = results(count);
        for item in &mut items {
            item.actions.push(Action {
                id: "copy".into(),
                title: "Copy".into(),
                ..Default::default()
            });
        }
        items
    }

    #[test]
    fn the_panel_opens_and_closes_only_from_the_snapshot() {
        let h = Harness::root(two_actions(3));
        h.freeze();
        actions_button(&h).mock_single_click(PointerEventButton::Left);
        assert_eq!(h.intents(), [Intent::TogglePanel]);
        assert!(!h.panel().get_open());

        h.launcher
            .borrow_mut()
            .apply(Input::Intent(Intent::TogglePanel));
        let snapshot = h.launcher.borrow().snapshot();
        h.renderer.borrow_mut().render(&h.ui, snapshot);
        assert!(h.panel().get_open());
        assert_eq!(h.panel().get_rows().row_count(), 2);
    }

    #[test]
    fn a_click_outside_does_not_reach_the_parent() {
        let h = Harness::root(two_actions(3));
        actions_button(&h).mock_single_click(PointerEventButton::Left);
        assert!(h.panel().get_open());
        h.freeze();
        h.intents.borrow_mut().clear();

        h.row("Result 2")
            .mock_single_click(PointerEventButton::Left);
        assert_eq!(h.intents(), [Intent::ClosePanel]);
        assert!(h.panel().get_open(), "only a snapshot closes the panel");
    }

    fn managed() -> ResultItem {
        let manage = |id: &str, title: &str, style| Action {
            id: id.into(),
            title: title.into(),
            section: Some("Manage".into()),
            style,
            ..Default::default()
        };
        let mut item = result(1);
        item.actions.push(manage(
            "delete",
            "Delete entry",
            crate::protocol::ActionStyle::Danger,
        ));
        item.actions.push(manage(
            "pin",
            "Pin entry",
            crate::protocol::ActionStyle::Normal,
        ));
        item
    }

    #[test]
    fn sections_head_their_actions_and_danger_comes_last() {
        let h = Harness::root(vec![managed()]);
        actions_button(&h).mock_single_click(PointerEventButton::Left);
        assert!(h.find("Manage").is_some());
        let rows = h.panel().get_rows();
        let last = rows.row_data(rows.row_count() - 1).unwrap();
        assert_eq!(last.title, "Delete entry");
        assert!(last.danger);
        let pin = h.find("Pin entry").unwrap().absolute_position().y;
        let delete = h.find("Delete entry").unwrap().absolute_position().y;
        assert!(delete > pin);
    }

    #[test]
    fn the_panel_over_a_short_list_sets_the_window_height() {
        let h = Harness::root(vec![managed()]);
        actions_button(&h).mock_single_click(PointerEventButton::Left);
        let panel = ElementHandle::find_by_accessible_label(&h.ui, "Actions")
            .find(|element| {
                element.accessible_role() == Some(i_slint_backend_testing::AccessibleRole::List)
            })
            .expect("the action panel");
        assert_eq!(
            h.height(),
            PILL + panel.size().height + ACTION_BAR + 2.0 * 8.0
        );
    }

    #[test]
    fn typing_in_the_panel_filters_it_and_escape_returns_to_the_query() {
        let h = Harness::root(two_actions(3));
        actions_button(&h).mock_single_click(PointerEventButton::Left);
        h.type_text("co");
        assert_eq!(h.query(), "");
        let rows = h.panel().get_rows();
        assert_eq!(rows.row_count(), 1);
        assert_eq!(rows.row_data(0).unwrap().title, "Copy");

        h.type_text("zz");
        assert!(h.find("No matching actions").is_some());

        h.key("\u{1b}");
        assert!(!h.panel().get_open());
        h.type_text("r");
        assert_eq!(h.query(), "r");
    }

    #[test]
    fn confirming_in_the_panel_runs_one_action() {
        let h = Harness::root(two_actions(3));
        actions_button(&h).mock_single_click(PointerEventButton::Left);
        h.arrow_down();
        h.intents.borrow_mut().clear();
        h.key("\n");
        assert_eq!(
            h.intents(),
            [Intent::Key {
                chord: crate::ui::Chord::plain("Enter")
            }]
        );
        assert!(!h.panel().get_open());
        assert_eq!(h.row("Result 1").accessible_item_selected(), Some(true));
    }

    #[test]
    fn closing_over_a_detail_gives_it_its_shortcuts_back() {
        let h = Harness::root(results(1));
        let mut tree = detail_tree("Body");
        if let View::Detail(detail) = &mut tree.view {
            detail.actions = vec![
                Action {
                    id: "open".into(),
                    title: "Open".into(),
                    ..Default::default()
                },
                Action {
                    id: "copy".into(),
                    title: "Copy".into(),
                    shortcut: Some(crate::protocol::Shortcut {
                        key: "c".into(),
                        modifiers: vec![crate::protocol::Modifier::Ctrl],
                    }),
                    ..Default::default()
                },
            ];
        }
        h.view(tree);
        let copy = crate::ui::Chord::command("c");
        assert!(h.renderer.borrow().binds(&copy, false));
        actions_button(&h).mock_single_click(PointerEventButton::Left);
        assert!(!h.renderer.borrow().binds(&copy, false));
        h.key("\u{1b}");
        assert!(h.renderer.borrow().binds(&copy, false));
    }

    #[test]
    fn the_shortcut_is_shown_beside_its_action() {
        let mut item = result(1);
        item.actions.push(Action {
            id: "copy".into(),
            title: "Copy".into(),
            shortcut: Some(crate::protocol::Shortcut {
                key: "c".into(),
                modifiers: vec![crate::protocol::Modifier::Ctrl],
            }),
            ..Default::default()
        });
        let h = Harness::root(vec![item]);
        actions_button(&h).mock_single_click(PointerEventButton::Left);
        let shortcut = h.panel().get_rows().row_data(1).unwrap().shortcut;
        assert!(shortcut.ends_with('C'), "{shortcut}");
    }
}

mod tray {
    use super::*;

    #[test]
    fn the_tray_lists_the_problems() {
        i_slint_backend_testing::init_no_event_loop();
        let tray = Tray::new().unwrap();
        super::super::show_tray(
            &tray,
            &TrayStatus {
                problems: vec!["Pasting is off, see dango.log".into()],
                config_error: false,
                missing: Vec::new(),
            },
        );
        let lines: Vec<String> = tray.get_lines().iter().map(String::from).collect();
        assert_eq!(
            lines,
            ["Pasting is off, see dango.log", "Config file loaded"]
        );
        assert!(!tray.get_grant_accessibility());
    }
}

mod notice {
    use super::super::generated::{NoticeWindow, TickerStyle as SlintTickerStyle};
    use super::super::render::notice;
    use super::*;
    use crate::ui::snapshot::Notice;
    use crate::ui::TickerStyle;

    fn shown(style: TickerStyle, text: &str) -> NoticeWindow {
        i_slint_backend_testing::init_no_event_loop();
        let ui = NoticeWindow::new().unwrap();
        ui.show().unwrap();
        notice::write(
            &ui,
            &Notice {
                id: 1,
                style,
                text: text.into(),
            },
        );
        ui
    }

    /// The testing backend has no native window, so the platform is never
    /// reached.
    pub(super) struct NoPlatform;

    impl crate::platform::NoticeWindow for NoPlatform {
        fn place(
            &self,
            _window: crate::platform::RawWindowHandle,
            _display: crate::config::LauncherDisplay,
        ) {
        }
    }

    #[test]
    fn the_shell_shows_a_new_notice_and_hides_a_gone_one() {
        i_slint_backend_testing::init_no_event_loop();
        let ui = NoticeWindow::new().unwrap();
        let shell_notice = super::super::Notice {
            window: ui.as_weak(),
            platform: std::sync::Arc::new(std::sync::Mutex::new(Box::new(NoPlatform))),
            attached: Default::default(),
            display: Default::default(),
        };
        let copied = Notice {
            id: 7,
            style: TickerStyle::Success,
            text: "Copied to clipboard".into(),
        };
        shell_notice.update(Some(&copied));
        assert!(ui.window().is_visible());
        assert_eq!(ui.get_text(), "Copied to clipboard");
        shell_notice.update(Some(&Notice {
            id: 8,
            style: TickerStyle::Failure,
            text: "Nope".into(),
        }));
        assert_eq!(ui.get_text(), "Nope");
        assert_eq!(ui.get_style(), SlintTickerStyle::Failure);
        shell_notice.update(None);
        assert!(!ui.window().is_visible());
        super::super::NOTICE_SHOWN.set(None);
    }

    fn label(ui: &NoticeWindow, text: &str) -> ElementHandle {
        ElementHandle::find_by_accessible_label(ui, text)
            .next()
            .unwrap_or_else(|| panic!("no element labelled {text}"))
    }

    #[test]
    fn a_failure_notice_says_it_failed() {
        let ui = shown(
            TickerStyle::Failure,
            "Dango can't move administrator windows",
        );
        assert_eq!(ui.get_style(), SlintTickerStyle::Failure);
        let text = label(&ui, "Dango can't move administrator windows");
        assert_eq!(text.accessible_description().as_deref(), Some("Failed"));
    }

    #[test]
    fn a_success_notice_shows_its_text() {
        let ui = shown(TickerStyle::Success, "Copied to clipboard");
        assert_eq!(ui.get_style(), SlintTickerStyle::Success);
        let text = label(&ui, "Copied to clipboard");
        assert_eq!(text.accessible_description().as_deref(), Some(""));
    }

    #[test]
    fn the_longest_failures_fit_the_notice() {
        let ui = shown(TickerStyle::Failure, "");
        let write = |ui: &NoticeWindow, text: &str| {
            notice::write(
                ui,
                &Notice {
                    id: 1,
                    style: TickerStyle::Failure,
                    text: text.into(),
                },
            )
        };
        for message in [
            "Set up this command's provider in your config file",
            "Couldn't verify the update, so it wasn't installed",
            "One keyword can't be the start of another, so choose a different keyword",
            "Can't open Visual Studio Code because it isn't installed",
        ] {
            write(&ui, message);
            assert!(!ui.get_cut(), "{message} is cut off");
        }
        write(&ui, &"Too long ".repeat(20));
        assert!(ui.get_cut(), "the measure sees a cut");
    }
}

mod progress_view {
    use super::*;
    use crate::protocol::{ProgressView, Status, Step};

    fn tree(value: Option<f32>) -> ViewTree {
        ViewTree::new(View::Progress(ProgressView {
            title: "Dango 0.5.0".into(),
            subtitle: "You have 0.4.2".into(),
            value,
            detail: "18.4 MB of 29.6 MB".into(),
            steps: [
                ("Download", Status::Working),
                ("Verify signature", Status::Queued),
                ("Restart", Status::Queued),
            ]
            .into_iter()
            .map(|(title, status)| Step {
                title: title.into(),
                status,
            })
            .collect(),
            actions: Vec::new(),
        }))
    }

    fn bar(h: &Harness) -> ElementHandle {
        i_slint_backend_testing::ElementQuery::from_root(&h.ui)
            .match_descendants()
            .match_predicate(|element| {
                element.accessible_description().as_deref() == Some("Dango 0.5.0")
            })
            .find_first()
            .expect("the progress bar")
    }

    #[test]
    fn a_progress_view_shows_its_bar_title_and_steps() {
        let h = Harness::root(results(1));
        h.view(tree(Some(0.62)));
        assert_eq!(bar(&h).accessible_value().as_deref(), Some("62"));
        assert!(h.find("Download, working").is_some());
        assert!(h.find("Verify signature, queued").is_some());
        assert!(h.find("Restart, queued").is_some());
        assert!(h.find("You have 0.4.2").is_some());
        assert!(h.find("18.4 MB of 29.6 MB").is_some());
    }

    #[test]
    fn a_progress_view_without_a_value_is_indeterminate() {
        let h = Harness::root(results(1));
        h.view(tree(None));
        let bar = bar(&h);
        assert_eq!(bar.accessible_value().as_deref(), Some(""));
        assert_eq!(bar.accessible_label().as_deref(), Some("Working"));
    }
}

mod ticker_action {
    use super::super::keys;
    use super::*;
    use crate::protocol::{Modifier, Shortcut};
    use crate::ui::{ActionRequest, Origin, TickerAction};

    #[test]
    fn a_ticker_chord_is_as_wide_as_its_keys() {
        let h = Harness::root(results(3));
        h.event(Event::Failed("Couldn't download the update.".into()));
        let mut snapshot = h.launcher.borrow().snapshot();
        let retry = Shortcut {
            key: "r".into(),
            modifiers: vec![Modifier::Cmd],
        };
        if let Some(ticker) = snapshot.ticker.as_mut() {
            ticker.action = Some(TickerAction {
                title: "Retry".into(),
                shortcut: retry.clone(),
                request: ActionRequest {
                    origin: Origin::Root,
                    extension_id: "dango.updates".into(),
                    item_id: "update".into(),
                    action_id: "install".into(),
                    values: None,
                    query: None,
                },
            });
        }
        h.publish(snapshot);
        let keys = keys::shortcut_label(&retry);
        let in_ticker = ElementHandle::find_by_element_type_name(&h.ui, "Kbd")
            .find(|kbd| kbd.accessible_label().as_deref() == Some(keys.as_str()))
            .expect("the ticker's chord")
            .size()
            .width;
        let gallery = super::super::generated::AtomGallery::new().unwrap();
        gallery.set_chord(keys.as_str().into());
        gallery.show().unwrap();
        let alone = ElementHandle::find_by_accessible_label(&gallery, &keys)
            .next()
            .expect("the chord on its own")
            .size()
            .width;
        assert_eq!(in_ticker, alone);
    }

    #[test]
    fn a_failure_ticker_shows_its_action_and_chord() {
        let h = Harness::root(results(3));
        h.event(Event::Failed("Couldn't download the update.".into()));
        let mut snapshot = h.launcher.borrow().snapshot();
        let shortcut = Shortcut {
            key: "r".into(),
            modifiers: vec![Modifier::Cmd],
        };
        if let Some(ticker) = snapshot.ticker.as_mut() {
            ticker.action = Some(TickerAction {
                title: "Retry".into(),
                shortcut: shortcut.clone(),
                request: ActionRequest {
                    origin: Origin::Root,
                    extension_id: "dango.updates".into(),
                    item_id: "update".into(),
                    action_id: "install".into(),
                    values: None,
                    query: None,
                },
            });
        }
        h.publish(snapshot);
        let chord = keys::shortcut_label(&shortcut);
        assert_eq!(h.shell().get_ticker_action_label(), "Retry");
        assert_eq!(h.shell().get_ticker_action_keys(), chord.as_str());
        let ticker = h.find("Couldn't download the update.").expect("the ticker");
        assert_eq!(
            ticker.accessible_description().as_deref(),
            Some(format!("Failed. Retry, {chord}").as_str())
        );
    }
}

mod inline_answer {
    use super::*;

    fn answer() -> ResultItem {
        ResultItem {
            extension_id: "dango.calculator".into(),
            id: "4".into(),
            title: "4".into(),
            answer: true,
            web_search: false,
            ..result(0)
        }
    }

    fn answered(rows: Vec<ResultItem>) -> Harness {
        let h = Harness::new();
        h.type_text("2+2");
        let mut items = vec![answer()];
        items.extend(rows);
        h.event(Event::Results {
            query: "2+2".into(),
            items,
            complete: true,
        });
        h
    }

    #[test]
    fn the_card_shows_and_is_selected() {
        let h = answered(results(2));
        let card = h.find("2+2 = 4").expect("the answer card");
        assert_eq!(card.accessible_item_selected(), Some(true));
        assert_eq!(h.row("Result 1").accessible_item_selected(), Some(false));
    }

    #[test]
    fn the_card_adds_its_height() {
        let h = Harness::new();
        h.type_text("2+2");
        h.event(Event::Results {
            query: "2+2".into(),
            items: results(2),
            complete: true,
        });
        let without = h.height();
        let mut items = vec![answer()];
        items.extend(results(2));
        h.event(Event::Results {
            query: "2+2".into(),
            items,
            complete: true,
        });
        assert_eq!(h.height(), without + 64.0);
    }

    #[test]
    fn an_answer_alone_shows_no_empty_view() {
        let h = answered(Vec::new());
        assert!(h.find("2+2 = 4").is_some());
        assert!(h.find("No results for “2+2”").is_none());
    }
}

mod root_hotkey {
    use super::*;
    use global_hotkey::hotkey::{Code, HotKey, Modifiers};

    #[test]
    fn a_root_row_shows_its_hotkey() {
        let chord = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyV);
        let label = super::super::keys::hotkey_label(&chord);
        let h = Harness::root(vec![
            ResultItem {
                hotkey: Some(chord),
                ..result(1)
            },
            result(2),
        ]);
        let shown: Vec<_> = ElementHandle::find_by_accessible_label(&h.ui, &label).collect();
        assert_eq!(shown.len(), 1, "{label} is drawn once");
    }
}
