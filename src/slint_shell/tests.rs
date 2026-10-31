//! The renderer mounted headless on Slint's testing backend, fed by the real
//! state machine.

use std::cell::RefCell;
use std::rc::Rc;

use i_slint_backend_testing::ElementHandle;
use slint::platform::{PointerEventButton, WindowEvent};
use slint::{ComponentHandle, LogicalPosition, Model};

use super::generated::{Intents, LauncherWindow, Panel, ScreenKind, Shell, Tray};
use super::render::{self, Renderer};
use crate::protocol::{
    Action, DetailView, FieldKind, Filtering, FormField, FormView, ListItem, ListView, View,
    ViewTree,
};
use crate::shell::testing::{Call, RecordingShell};
use crate::shell::TrayStatus;
use crate::ui::{Context, Event, Input, Intent, Launcher, ResultItem, TemplateInspection};

const LIST_TOP: f32 = 56.0 + 6.0;
const LIST_BOTTOM: f32 = 480.0 - 40.0 - 6.0;

struct Harness {
    ui: LauncherWindow,
    launcher: Rc<RefCell<Launcher>>,
    renderer: Rc<RefCell<Renderer>>,
    intents: Rc<RefCell<Vec<Intent>>>,
    /// Records intents without feeding them to the state machine, so what is
    /// on screen can only change through a snapshot.
    frozen: Rc<RefCell<bool>>,
    recorder: Rc<RecordingShell>,
}

impl Harness {
    fn new() -> Self {
        i_slint_backend_testing::init_no_event_loop();
        let ui = LauncherWindow::new().unwrap();
        ui.window().set_size(slint::LogicalSize::new(720.0, 480.0));
        ui.show().unwrap();
        let renderer = Rc::new(RefCell::new(Renderer::new(&ui)));
        let launcher = Rc::new(RefCell::new(Launcher::new()));
        let intents = Rc::new(RefCell::new(Vec::new()));
        let frozen = Rc::new(RefCell::new(false));
        let recorder = Rc::new(RecordingShell::default());
        let dispatch = {
            let recorder = recorder.clone();
            let weak = ui.as_weak();
            let launcher = launcher.clone();
            let renderer = renderer.clone();
            let intents = intents.clone();
            let frozen = frozen.clone();
            Rc::new(move |intent: Intent| {
                intents.borrow_mut().push(intent.clone());
                if *frozen.borrow() {
                    return;
                }
                launcher.borrow_mut().apply(Input::Intent(intent));
                let snapshot = launcher.borrow().snapshot();
                render::publish(
                    &*recorder,
                    &mut renderer.borrow_mut(),
                    &weak.unwrap(),
                    snapshot,
                );
            }) as Rc<dyn Fn(Intent)>
        };
        render::connect(&ui, renderer.clone(), dispatch);
        let harness = Self {
            ui,
            launcher,
            renderer,
            intents,
            frozen,
            recorder,
        };
        harness.event(Event::Activated {
            held: false,
            probe: None,
        });
        harness
    }

    fn root(items: Vec<ResultItem>) -> Self {
        let harness = Self::new();
        harness.event(Event::Results {
            query: String::new(),
            items,
        });
        harness
    }

    fn event(&self, event: Event) {
        self.launcher.borrow_mut().apply(Input::Event(event));
        let snapshot = self.launcher.borrow().snapshot();
        render::publish(
            &*self.recorder,
            &mut self.renderer.borrow_mut(),
            &self.ui,
            snapshot,
        );
    }

    fn height(&self) -> f32 {
        self.recorder
            .calls()
            .into_iter()
            .rev()
            .find_map(|call| match call {
                Call::Height(height) => Some(height),
                _ => None,
            })
            .expect("a height was set")
    }

    fn view(&self, tree: ViewTree) {
        self.event(Event::Rendered {
            owner: "fixture".into(),
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

    fn row(&self, id: &str) -> ElementHandle {
        self.find(id).unwrap_or_else(|| panic!("row {id} is drawn"))
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
        assert_eq!(theme.get_surface(), Color::from_rgb_u8(0x1e, 0x1e, 0x21));
        assert_eq!(theme.get_muted(), Color::from_rgb_u8(0x9a, 0x9a, 0xa2));

        palette.set_color_scheme(ColorScheme::Light);
        assert!(!theme.get_dark());
        assert_eq!(theme.get_surface(), Color::from_rgb_u8(0xff, 0xff, 0xff));
        assert_eq!(theme.get_muted(), Color::from_rgb_u8(0x6b, 0x6b, 0x72));
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
        for label in ["Icon", "Link", "2m", "Busy", keys::panel_label().as_str()] {
            assert!(
                ElementHandle::find_by_accessible_label(&gallery, label)
                    .next()
                    .is_some(),
                "{label} is drawn"
            );
        }
    }

    #[test]
    fn a_kbd_shows_the_platform_chord() {
        i_slint_backend_testing::init_no_event_loop();
        let gallery = AtomGallery::new().unwrap();
        gallery.set_chord(keys::panel_label().into());
        gallery.show().unwrap();
        let expected = if cfg!(target_os = "macos") {
            "⌘ K"
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
        assert!(h.find("result-1").is_some());
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
        let bar = ElementHandle::find_by_accessible_label(&h.ui, "Loading…").find(|element| {
            element.accessible_role()
                == Some(i_slint_backend_testing::AccessibleRole::ProgressIndicator)
        });
        assert!(bar.is_some());
    }

    #[test]
    fn a_form_shows_its_title_instead_of_the_query() {
        let h = Harness::root(results(3));
        h.view(form_tree());
        assert!(!h.shell().get_typing());
        assert!(h.find("Fixture Command").is_some());
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

mod toast {
    use super::*;
    use crate::extension::ActionOutcome;
    use crate::ui::{ActionRequest, Origin};

    fn toast(h: &Harness) -> ElementHandle {
        let text = h.shell().get_toast_text();
        h.find(&text).expect("the toast")
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
    fn a_busy_toast_says_what_is_running() {
        let mut command = result(1);
        command.title = "Clipboard History".into();
        command.actions.clear();
        let h = Harness::root(vec![command]);
        h.key(
            "
",
        );
        assert_eq!(
            h.shell().get_toast_style(),
            super::super::generated::ToastStyle::Busy
        );
        assert!(h.find("Running Clipboard History…").is_some());
    }

    #[test]
    fn a_success_toast_replaces_the_context() {
        let h = Harness::root(results(3));
        h.view(views::list_tree());
        h.event(Event::ActionFinished {
            request: request(),
            outcome: ActionOutcome::Toast("Entry pinned".into()),
        });
        assert!(h.shell().get_toast_shown());
        assert_eq!(
            h.shell().get_toast_style(),
            super::super::generated::ToastStyle::Success
        );
        assert!(h.find("Entry pinned").is_some());
        assert!(h.find("Fixture Command").is_none());
    }

    #[test]
    fn a_failure_toast_says_it_failed() {
        let h = Harness::root(results(3));
        h.event(Event::Failed("That command is no longer available".into()));
        assert_eq!(
            h.shell().get_toast_style(),
            super::super::generated::ToastStyle::Failure
        );
        assert_eq!(
            toast(&h).accessible_description().as_deref(),
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
            "Couldn't verify the update, so it wasn't installed",
            "Copy the text first, as Visual Studio Code doesn't share its selection",
        ] {
            h.event(Event::Failed(message.into()));
            assert!(h.shell().get_panel_hint());
            assert!(!h.ui.get_toast_cut(), "{message} is cut off");
        }
        h.event(Event::Failed("Too long ".repeat(20)));
        assert!(h.ui.get_toast_cut(), "the measure sees a cut");
    }

    #[test]
    fn typing_reaches_the_search_bar_while_a_toast_shows() {
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
        assert_eq!(h.row("result-1").accessible_item_selected(), Some(true));

        h.move_pointer_to(&h.row("result-3"));
        assert!(!h.shell().get_pointer_resting());
        assert_eq!(h.row("result-1").accessible_item_selected(), Some(true));
        assert_eq!(h.row("result-3").accessible_item_selected(), Some(false));
        assert!(h.intents().is_empty(), "hover sent {:?}", h.intents());

        h.arrow_down();
        assert!(h.shell().get_pointer_resting());
        assert_eq!(h.row("result-2").accessible_item_selected(), Some(true));

        h.move_pointer_to(&h.row("result-4"));
        assert!(!h.shell().get_pointer_resting());
    }

    #[test]
    fn clicking_keeps_typed_query_focus() {
        let h = Harness::root(results(5));
        h.type_text("re");
        assert_eq!(h.query(), "re");

        h.row("result-2")
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
        let h = Harness::root(items);
        for _ in 0..5 {
            h.arrow_down();
        }
        let row = h.row("result-6");
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
    fn an_answer_to_an_older_keystroke_does_not_overwrite_typing() {
        let h = Harness::root(results(3));
        h.type_text("ab");
        h.event(Event::Results {
            query: "a".into(),
            items: results(2),
        });
        assert_eq!(h.query(), "ab");
    }
}

mod views {
    use super::*;

    pub(super) fn list_tree() -> ViewTree {
        ViewTree::new(View::List(ListView {
            filtering: Filtering::Launcher,
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
        assert_eq!(h.row("item-1").accessible_item_selected(), Some(true));
        assert!(h.find("item-3").is_some());
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

    #[test]
    fn a_row_is_forty_pixels_high() {
        let h = Harness::root(results(3));
        assert_eq!(h.row("result-1").size().height, 40.0);
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

    #[test]
    fn hover_is_weaker_than_selection() {
        let h = Harness::new();
        let theme = h.ui.global::<Theme>();
        for scheme in [ColorScheme::Dark, ColorScheme::Light] {
            h.ui.global::<StdPalette>().set_color_scheme(scheme);
            assert!(theme.get_hover().alpha() < theme.get_selection().alpha());
        }
    }

    #[test]
    fn no_results_shows_the_empty_view_with_the_query() {
        let h = Harness::root(results(3));
        h.type_text("zzz");
        h.event(Event::Results {
            query: "zzz".into(),
            items: Vec::new(),
        });
        assert_eq!(h.shell().get_screen(), ScreenKind::Empty);
        assert!(h.find("No results for “zzz”").is_some());
    }

    #[test]
    fn arrow_down_changes_the_detail_pane() {
        let detailed = |n: usize, body: &str| ListItem {
            detail: Some(ItemDetail {
                markdown: body.into(),
                metadata: Vec::new(),
            }),
            ..item(n)
        };
        let h = Harness::root(results(1));
        h.view(tree(
            vec![detailed(1, "First body"), detailed(2, "Second body")],
            true,
        ));
        let first = h.find("First body").expect("the first detail");
        assert!(first.absolute_position().x >= 720.0 * 2.0 / 5.0);
        assert!(h.row("item-1").size().width <= 720.0 * 2.0 / 5.0);
        h.arrow_down();
        assert!(h.find("Second body").is_some());
        assert!(h.find("First body").is_none());
    }
}

mod detail {
    use super::*;
    use crate::protocol::Metadata;

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
            filtering: Filtering::Command,
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
        let place = |n: usize| h.row(&format!("tile-{n}")).absolute_position();
        for n in 2..=5 {
            assert_eq!(place(n).y, place(1).y, "tile {n} is in the first row");
        }
        assert!(place(6).y > place(1).y);
        assert_eq!(place(6).x, place(1).x);
        assert!(place(11).y > place(6).y);
        assert_eq!(place(12).y, place(11).y);
        assert_eq!(h.row("tile-1").accessible_item_selected(), Some(true));
    }

    #[test]
    fn arrow_left_moves_the_selection_not_the_text_cursor() {
        let h = Harness::root(results(1));
        h.view(grid_tree(12));
        h.type_text("ab");
        h.key("\u{F703}");
        assert_eq!(h.row("tile-2").accessible_item_selected(), Some(true));
        h.key("\u{F702}");
        assert_eq!(h.row("tile-1").accessible_item_selected(), Some(true));
        h.type_text("c");
        assert_eq!(h.query_of("Search"), "abc");
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

mod window_height {
    use super::*;

    #[test]
    fn three_results_make_a_short_window() {
        let h = Harness::root(results(3));
        assert_eq!(h.height(), 56.0 + 6.0 + 3.0 * 40.0 + 6.0 + 40.0);
    }

    #[test]
    fn forty_results_stop_at_the_maximum() {
        let h = Harness::root(results(40));
        assert_eq!(h.height(), 480.0);
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

        h.row("result-2")
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
        assert_eq!(h.row("result-1").accessible_item_selected(), Some(true));
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
    fn the_tray_lists_the_notices() {
        i_slint_backend_testing::init_no_event_loop();
        let tray = Tray::new().unwrap();
        super::super::show_tray(
            &tray,
            &TrayStatus {
                notices: vec!["Pasting is off, see dango.log".into()],
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
