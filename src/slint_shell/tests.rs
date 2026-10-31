//! The renderer mounted headless on Slint's testing backend, fed by the real
//! state machine.

use std::cell::RefCell;
use std::rc::Rc;

use i_slint_backend_testing::ElementHandle;
use slint::platform::{PointerEventButton, WindowEvent};
use slint::{ComponentHandle, LogicalPosition, Model};

use super::generated::{LauncherWindow, ScreenKind, Tray};
use super::render::{self, Renderer};
use crate::protocol::{
    Action, DetailView, FieldKind, Filtering, FormField, FormView, ListItem, ListView, View,
    ViewTree,
};
use crate::shell::{self, TrayStatus};
use crate::ui::{Event, Input, Intent, Launcher, ResultItem, TemplateInspection};

const LIST_TOP: f32 = 64.0 + 8.0;
const LIST_BOTTOM: f32 = 400.0 - 40.0 - 8.0;

struct Harness {
    ui: LauncherWindow,
    launcher: Rc<RefCell<Launcher>>,
    renderer: Rc<RefCell<Renderer>>,
    intents: Rc<RefCell<Vec<Intent>>>,
    /// Records intents without feeding them to the state machine, so what is
    /// on screen can only change through a snapshot.
    frozen: Rc<RefCell<bool>>,
}

impl Harness {
    fn new(development: bool) -> Self {
        i_slint_backend_testing::init_no_event_loop();
        let ui = LauncherWindow::new().unwrap();
        ui.window().set_size(slint::LogicalSize::new(720.0, 400.0));
        ui.show().unwrap();
        let renderer = Rc::new(RefCell::new(Renderer::new(&ui)));
        let launcher = Rc::new(RefCell::new(Launcher::new(development)));
        let intents = Rc::new(RefCell::new(Vec::new()));
        let frozen = Rc::new(RefCell::new(false));
        let dispatch = {
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
                renderer.borrow_mut().render(&weak.unwrap(), snapshot);
            }) as Rc<dyn Fn(Intent)>
        };
        render::connect(&ui, renderer.clone(), dispatch);
        let harness = Self {
            ui,
            launcher,
            renderer,
            intents,
            frozen,
        };
        harness.event(Event::Activated {
            held: false,
            probe: None,
        });
        harness
    }

    fn root(items: Vec<ResultItem>) -> Self {
        let harness = Self::new(false);
        harness.event(Event::Results {
            query: String::new(),
            items,
        });
        harness
    }

    fn event(&self, event: Event) {
        self.launcher.borrow_mut().apply(Input::Event(event));
        let snapshot = self.launcher.borrow().snapshot();
        self.renderer.borrow_mut().render(&self.ui, snapshot);
    }

    fn view(&self, tree: ViewTree) {
        self.event(Event::Rendered {
            owner: "fixture".into(),
            invocation: 1,
            tree,
        });
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
        self.find("Search apps and commands")
            .and_then(|prompt| prompt.accessible_value())
            .unwrap_or_default()
            .to_string()
    }
}

fn result(n: usize) -> ResultItem {
    ResultItem {
        extension_id: "fixture".into(),
        id: format!("result-{n}"),
        title: format!("Result {n}"),
        subtitle: None,
        icon: None,
        tint: None,
        actions: vec![Action {
            id: "open".into(),
            title: "Open".into(),
            shortcut: None,
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
    }))
}

mod development_indicator {
    use super::*;

    #[test]
    fn root_search_marks_a_development_build() {
        let h = Harness::new(true);
        assert!(h.find("Dango (development build)").is_some());
        assert!(h.find("dev").is_some());
    }

    #[test]
    fn root_search_shows_nothing_in_a_release_build() {
        let h = Harness::new(false);
        assert!(h.find("Dango (development build)").is_none());
        assert!(h.find("Dango").is_some());
        assert!(h.find("dev").is_none());
    }

    #[test]
    fn a_pushed_view_keeps_the_marker() {
        let h = Harness::new(true);
        h.view(detail_tree("Working"));
        assert_eq!(h.ui.get_screen(), ScreenKind::Detail);
        assert!(h.find("Dango (development build)").is_some());
        assert!(h.find("dev").is_some());
    }

    #[test]
    fn protocol_error_screen_keeps_the_edge() {
        let h = Harness::new(true);
        let mut tree = detail_tree("Working");
        tree.protocol_version = 99;
        h.view(tree);
        assert_eq!(h.ui.get_screen(), ScreenKind::Error);
        assert!(h.find("Dango (development build)").is_some());
        assert!(h
            .find("This view needs a newer version of Dango.")
            .is_some());
    }
}

mod root_search {
    use super::*;

    #[test]
    fn pointer_hover_never_selects_and_keyboard_suppresses_resting_hover() {
        let h = Harness::root(results(5));
        assert_eq!(h.row("result-1").accessible_item_selected(), Some(true));

        h.move_pointer_to(&h.row("result-3"));
        assert!(!h.ui.get_pointer_resting());
        assert_eq!(h.row("result-1").accessible_item_selected(), Some(true));
        assert_eq!(h.row("result-3").accessible_item_selected(), Some(false));
        assert!(h.intents().is_empty(), "hover sent {:?}", h.intents());

        h.arrow_down();
        assert!(h.ui.get_pointer_resting());
        assert_eq!(h.row("result-2").accessible_item_selected(), Some(true));

        h.move_pointer_to(&h.row("result-4"));
        assert!(!h.ui.get_pointer_resting());
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

    fn list_tree() -> ViewTree {
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
                })
                .collect(),
        }))
    }

    fn form_tree() -> ViewTree {
        let field = |id: &str, kind| FormField {
            id: id.into(),
            label: id.into(),
            kind,
            value: Some(String::new()),
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
            }],
        }))
    }

    #[test]
    fn a_pushed_list_renders_its_items() {
        let h = Harness::root(results(3));
        h.view(list_tree());
        assert_eq!(h.ui.get_screen(), ScreenKind::List);
        assert!(h.find("Search").is_some());
        assert_eq!(h.row("item-1").accessible_item_selected(), Some(true));
        assert!(h.find("item-3").is_some());
    }

    #[test]
    fn a_detail_renders_its_text() {
        let h = Harness::root(results(3));
        h.view(detail_tree("Plain words"));
        assert_eq!(h.ui.get_screen(), ScreenKind::Detail);
        assert!(h.find("Plain words").is_some());
    }

    #[test]
    fn a_form_renders_every_field() {
        let h = Harness::root(results(3));
        h.view(form_tree());
        assert_eq!(h.ui.get_screen(), ScreenKind::Form);
        for label in ["name", "enabled", "template"] {
            assert!(h.find(label).is_some(), "{label} is drawn");
        }
    }

    #[test]
    fn a_template_error_is_the_fields_accessible_description() {
        let h = Harness::root(results(3));
        h.view(form_tree());
        h.ui.invoke_field_edited("template".into(), "{{ broken".into());
        h.event(Event::TemplateInspected {
            field: "template".into(),
            source: "{{ broken".into(),
            inspection: TemplateInspection {
                arguments: Vec::new(),
                error: Some("The template doesn't close.".into()),
            },
        });
        let described = ElementHandle::find_by_accessible_label(&h.ui, "template")
            .filter_map(|field| field.accessible_description())
            .any(|description| description == "The template doesn't close.");
        assert!(described);
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

    #[test]
    fn the_panel_opens_and_closes_only_from_the_snapshot() {
        let h = Harness::root(results(3));
        h.freeze();
        actions_button(&h).mock_single_click(PointerEventButton::Left);
        assert_eq!(h.intents(), [Intent::TogglePanel]);
        assert!(!h.ui.get_panel_open());

        h.launcher
            .borrow_mut()
            .apply(Input::Intent(Intent::TogglePanel));
        let snapshot = h.launcher.borrow().snapshot();
        h.renderer.borrow_mut().render(&h.ui, snapshot);
        assert!(h.ui.get_panel_open());
        assert_eq!(h.ui.get_actions().row_count(), 1);
    }

    #[test]
    fn a_click_outside_does_not_reach_the_parent() {
        let h = Harness::root(results(3));
        actions_button(&h).mock_single_click(PointerEventButton::Left);
        assert!(h.ui.get_panel_open());
        h.freeze();
        h.intents.borrow_mut().clear();

        h.row("result-2")
            .mock_single_click(PointerEventButton::Left);
        assert_eq!(h.intents(), [Intent::ClosePanel]);
        assert!(h.ui.get_panel_open(), "only a snapshot closes the panel");
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
        });
        let h = Harness::root(vec![item]);
        actions_button(&h).mock_single_click(PointerEventButton::Left);
        let shortcut = h.ui.get_actions().row_data(1).unwrap().shortcut;
        assert!(shortcut.ends_with('C'), "{shortcut}");
    }
}

mod tray {
    use super::*;

    #[test]
    fn the_tray_lists_the_notices_and_names_a_development_build() {
        i_slint_backend_testing::init_no_event_loop();
        let tray = Tray::new().unwrap();
        tray.set_hint(shell::tooltip().into());
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
        #[cfg(debug_assertions)]
        assert_eq!(tray.get_hint(), "Dango (development build)");
    }
}
