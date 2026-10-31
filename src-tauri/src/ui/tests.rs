use crate::extension::ActionOutcome;
use crate::protocol::{
    Action, DetailView, FieldKind, Filtering, FormField, FormView, ListItem, ListView, Modifier,
    Shortcut, View, ViewTree,
};

use super::snapshot::{DetailBody, Focus, ListBody, RootScreen, Screen, ViewBody};
use super::*;

fn actions() -> Vec<Action> {
    vec![
        Action {
            id: "open".into(),
            title: "Open".into(),
            shortcut: None,
        },
        Action {
            id: "copy".into(),
            title: "Copy".into(),
            shortcut: Some(Shortcut {
                key: "c".into(),
                modifiers: vec![Modifier::Ctrl],
            }),
        },
    ]
}

fn result(n: usize) -> ResultItem {
    ResultItem {
        extension_id: "fixture".into(),
        id: format!("result-{n}"),
        title: format!("Result {n}"),
        subtitle: None,
        icon: None,
        tint: None,
        actions: actions(),
        match_positions: Vec::new(),
        suggested: false,
        pinned: false,
    }
}

fn results(count: usize) -> Vec<ResultItem> {
    (1..=count).map(result).collect()
}

fn list_item(n: usize) -> ListItem {
    ListItem {
        id: format!("item-{n}"),
        title: format!("Item {n}"),
        subtitle: None,
        icon: None,
        actions: actions(),
    }
}

fn list_tree(items: Vec<ListItem>) -> ViewTree {
    ViewTree::new(View::List(ListView {
        filtering: Filtering::Launcher,
        loading: false,
        empty_state: None,
        items,
    }))
}

fn detail_tree(markdown: &str) -> ViewTree {
    ViewTree::new(View::Detail(DetailView {
        markdown: markdown.into(),
        loading: true,
        actions: actions(),
    }))
}

fn field(id: &str, kind: FieldKind, value: &str) -> FormField {
    FormField {
        id: id.into(),
        label: id.into(),
        kind,
        value: Some(value.into()),
    }
}

fn form_tree(fields: Vec<FormField>) -> ViewTree {
    let mut form_actions = vec![Action {
        id: "save".into(),
        title: "Save".into(),
        shortcut: None,
    }];
    form_actions.extend(actions());
    ViewTree::new(View::Form(FormView {
        item_id: Some("fixture-form".into()),
        fields,
        actions: form_actions,
    }))
}

fn mixed_form() -> ViewTree {
    form_tree(vec![
        field("name", FieldKind::Text, "Morning note"),
        field("password", FieldKind::Password, "fixture-only"),
        field("enabled", FieldKind::Toggle, "true"),
        field("template", FieldKind::Template, "Hello {{name}}"),
    ])
}

struct Harness {
    launcher: Launcher,
    effects: Vec<Effect>,
}

impl Harness {
    fn new() -> Self {
        Self {
            launcher: Launcher::new(false),
            effects: Vec::new(),
        }
    }

    fn root(count: usize) -> Self {
        let mut harness = Self::new();
        harness.results("", results(count));
        harness
    }

    fn view(tree: ViewTree) -> Self {
        let mut harness = Self::root(18);
        harness.render(1, tree);
        harness
    }

    fn intent(&mut self, intent: Intent) -> Vec<Effect> {
        let effects = self.launcher.apply(Input::Intent(intent));
        self.effects.extend(effects.clone());
        effects
    }

    fn event(&mut self, event: Event) -> Vec<Effect> {
        let effects = self.launcher.apply(Input::Event(event));
        self.effects.extend(effects.clone());
        effects
    }

    fn key(&mut self, chord: Chord) -> Vec<Effect> {
        self.intent(Intent::Key { chord })
    }

    fn press(&mut self, key: &str) -> Vec<Effect> {
        self.key(Chord::plain(key))
    }

    fn command(&mut self, key: &str) -> Vec<Effect> {
        self.key(Chord::command(key))
    }

    fn typed(&mut self, text: &str) -> Vec<Effect> {
        self.intent(Intent::QueryChanged { text: text.into() })
    }

    fn results(&mut self, query: &str, items: Vec<ResultItem>) {
        self.event(Event::Results {
            query: query.into(),
            items,
        });
    }

    fn render(&mut self, invocation: u64, tree: ViewTree) {
        self.event(Event::Rendered {
            owner: "fixture".into(),
            invocation,
            tree,
        });
    }

    fn finish(&mut self, request: ActionRequest, outcome: ActionOutcome) -> Vec<Effect> {
        self.event(Event::ActionFinished { request, outcome })
    }

    fn snapshot(&self) -> Snapshot {
        self.launcher.snapshot()
    }

    fn root_screen(&self) -> RootScreen {
        match self.snapshot().screen {
            Screen::Root(root) => root,
            other => panic!("expected root, got {other:?}"),
        }
    }

    fn view_body(&self) -> ViewBody {
        match self.snapshot().screen {
            Screen::View(view) => view.body,
            other => panic!("expected a view, got {other:?}"),
        }
    }

    fn list(&self) -> ListBody {
        match self.view_body() {
            ViewBody::List(list) => list,
            other => panic!("expected a list, got {other:?}"),
        }
    }

    fn selected(&self) -> Option<String> {
        match self.snapshot().screen {
            Screen::Root(root) => root.selected,
            Screen::View(view) => match view.body {
                ViewBody::List(list) => list.selected,
                _ => None,
            },
            Screen::Error => None,
        }
    }

    fn panel_selected(&self) -> Option<String> {
        self.snapshot().panel.selected
    }

    fn runs(&self) -> Vec<ActionRequest> {
        self.effects
            .iter()
            .filter_map(|effect| match effect {
                Effect::RunAction(request) => Some(request.clone()),
                _ => None,
            })
            .collect()
    }

    fn count(&self, wanted: &Effect) -> usize {
        self.effects
            .iter()
            .filter(|effect| *effect == wanted)
            .count()
    }

    fn searches(&self, query: &str) -> usize {
        self.count(&Effect::Search(query.into()))
    }
}

fn id(n: usize) -> Option<String> {
    Some(format!("result-{n}"))
}

fn item_id(n: usize) -> Option<String> {
    Some(format!("item-{n}"))
}

// Root search

#[test]
fn root_selection_starts_on_the_first_row_and_does_not_wrap() {
    let mut h = Harness::root(18);
    assert_eq!(h.selected(), id(1));
    h.press("ArrowUp");
    assert_eq!(h.selected(), id(1));
    for n in 2..=18 {
        h.press("ArrowDown");
        assert_eq!(h.selected(), id(n));
    }
    h.press("ArrowDown");
    assert_eq!(h.selected(), id(18));
    assert_eq!(h.snapshot().focus, Focus::RootInput);
}

#[test]
fn root_replacement_keeps_the_selected_identity_and_falls_back_when_it_goes() {
    let mut h = Harness::root(18);
    h.press("ArrowDown");
    let mut reordered = results(18);
    reordered.swap(0, 2);
    h.results("", reordered.clone());
    assert_eq!(h.root_screen().sections[0].items[0].id, "result-3");
    assert_eq!(h.selected(), id(2));
    reordered.retain(|item| item.id != "result-2");
    h.results("", reordered);
    assert_eq!(h.selected(), id(3));
}

#[test]
fn root_results_for_an_older_query_are_dropped() {
    let mut h = Harness::root(18);
    h.typed("Result 12");
    h.results("Result 12", vec![result(12)]);
    h.results("", results(18));
    let root = h.root_screen();
    assert_eq!(
        root.sections.iter().map(|s| s.items.len()).sum::<usize>(),
        1
    );
    assert_eq!(h.selected(), id(12));
}

#[test]
fn typing_searches_and_resets_the_pick_without_echoing_the_query() {
    let mut h = Harness::root(18);
    h.press("ArrowDown");
    let revision = h.root_screen().query.revision;
    let effects = h.typed("Res");
    assert_eq!(effects, vec![Effect::Search("Res".into())]);
    h.results("Res", results(18));
    assert_eq!(h.selected(), id(1));
    assert_eq!(h.root_screen().query.text, "Res");
    assert_eq!(h.root_screen().query.revision, revision);
}

#[test]
fn root_sections_group_pinned_and_suggested_results() {
    let mut h = Harness::new();
    let mut items = results(5);
    items[3].pinned = true;
    items[0].suggested = true;
    items[1].suggested = true;
    h.results("", items);
    let sections = h.root_screen().sections;
    let headings: Vec<_> = sections.iter().map(|s| s.heading.as_deref()).collect();
    assert_eq!(
        headings,
        [
            Some("Needs attention"),
            Some("Suggestions"),
            Some("Everything else")
        ]
    );
    let ids: Vec<_> = sections
        .iter()
        .map(|s| s.items.iter().map(|i| i.id.as_str()).collect::<Vec<_>>())
        .collect();
    assert_eq!(
        ids,
        [
            vec!["result-4"],
            vec!["result-1", "result-2"],
            vec!["result-3", "result-5"]
        ]
    );
    assert_eq!(h.selected(), id(4));
    h.press("ArrowDown");
    assert_eq!(h.selected(), id(1));
}

#[test]
fn root_without_groups_is_one_untitled_section() {
    let h = Harness::root(3);
    let sections = h.root_screen().sections;
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].heading, None);
}

#[test]
fn an_empty_search_says_what_found_nothing() {
    let mut h = Harness::root(3);
    assert_eq!(h.root_screen().empty, None);
    h.typed("zzz");
    h.results("zzz", Vec::new());
    assert_eq!(
        h.root_screen().empty.as_deref(),
        Some("No results for “zzz”")
    );
}

// View stack

#[test]
fn a_view_pushes_and_takes_the_keyboard() {
    let h = Harness::view(list_tree((1..=18).map(list_item).collect()));
    assert_eq!(h.snapshot().focus, Focus::ViewInput);
    assert_eq!(h.selected(), item_id(1));
}

#[test]
fn thirty_replacements_of_the_same_invocation_keep_selection() {
    let mut h = Harness::view(list_tree((1..=18).map(list_item).collect()));
    h.press("ArrowDown");
    for i in 0..30 {
        let mut tree = list_tree((1..=18).map(list_item).collect());
        if let View::List(list) = &mut tree.view {
            list.loading = i < 29;
        }
        h.render(1, tree);
    }
    assert_eq!(h.launcher.stack.len(), 1);
    assert_eq!(h.selected(), item_id(2));
    assert_eq!(h.snapshot().focus, Focus::ViewInput);
}

#[test]
fn list_replacement_keeps_identity_and_falls_back_to_the_first_row() {
    let mut h = Harness::view(list_tree((1..=18).map(list_item).collect()));
    h.press("ArrowDown");
    let mut items: Vec<_> = (1..=18).map(list_item).collect();
    items.swap(0, 2);
    h.render(1, list_tree(items.clone()));
    assert_eq!(h.list().items[0].id, "item-3");
    assert_eq!(h.selected(), item_id(2));
    let items: Vec<_> = (1..=18).filter(|n| *n != 2).map(list_item).collect();
    h.render(1, list_tree(items));
    assert_eq!(h.selected(), item_id(1));
}

#[test]
fn a_later_invocation_pushes_on_top() {
    let mut h = Harness::view(detail_tree("first"));
    h.render(2, detail_tree("second"));
    assert_eq!(h.launcher.stack.len(), 2);
    h.press("Escape");
    assert_eq!(
        h.view_body(),
        ViewBody::Detail(DetailBody {
            markdown: "first".into()
        })
    );
}

#[test]
fn a_late_tree_after_escape_is_ignored() {
    let mut h = Harness::view(detail_tree("Chunk 1"));
    let effects = h.press("Escape");
    assert!(effects.contains(&Effect::CancelInvocation));
    assert!(matches!(h.snapshot().screen, Screen::Root(_)));
    h.render(1, detail_tree("Late chunk"));
    assert!(matches!(h.snapshot().screen, Screen::Root(_)));
    h.render(2, detail_tree("New command"));
    assert!(matches!(h.snapshot().screen, Screen::View(_)));
}

#[test]
fn an_unsupported_protocol_version_shows_the_error_until_escape() {
    let mut h = Harness::view(detail_tree("Working…"));
    let mut tree = detail_tree("Working…");
    tree.protocol_version = 99;
    h.render(1, tree);
    let snapshot = h.snapshot();
    assert_eq!(snapshot.screen, Screen::Error);
    assert_eq!(snapshot.focus, Focus::Nothing);
    assert_eq!(snapshot.bindings.len(), 1);
    assert!(h.press("Escape").is_empty());
    assert!(matches!(h.snapshot().screen, Screen::View(_)));
}

#[test]
fn an_action_in_a_view_goes_to_the_owner_of_the_view() {
    let mut h = Harness::new();
    h.event(Event::Rendered {
        owner: "clipboard".into(),
        invocation: 4,
        tree: list_tree(vec![list_item(1)]),
    });
    h.press("Enter");
    let runs = h.runs();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].extension_id, "clipboard");
    assert_eq!(runs[0].origin, Origin::View);
    assert_eq!(runs[0].query, None);
}

#[test]
fn popping_the_last_view_returns_to_the_root_query() {
    let mut h = Harness::root(18);
    h.typed("Res");
    h.results("Res", results(3));
    let before = h.root_screen().query.revision;
    h.render(1, detail_tree("x"));
    h.press("Escape");
    let root = h.root_screen();
    assert_eq!(root.query.text, "Res");
    assert!(root.query.revision > before);
}

// Pushed views

#[test]
fn a_launcher_filtered_list_narrows_by_title_and_keeps_its_own_query() {
    let mut h = Harness::view(list_tree((1..=18).map(list_item).collect()));
    let revision = h.list().query.revision;
    assert!(h.typed("item 1").is_empty());
    let list = h.list();
    assert_eq!(list.items.len(), 10);
    assert_eq!(list.query.text, "item 1");
    assert_eq!(list.query.revision, revision);
    h.typed("item 12");
    assert_eq!(h.selected(), item_id(12));
    assert_eq!(h.root_screen_query(), "");
    h.typed("nothing");
    let list = h.list();
    assert_eq!(list.empty.unwrap().title, "No results for “nothing”");
    assert!(h.snapshot().footer.hide_status);
}

impl Harness {
    fn root_screen_query(&self) -> String {
        self.launcher.query.clone()
    }
}

#[test]
fn a_list_view_moves_its_own_selection_without_wrapping() {
    let mut h = Harness::view(list_tree((1..=3).map(list_item).collect()));
    h.press("ArrowUp");
    assert_eq!(h.selected(), item_id(1));
    h.press("ArrowDown");
    h.press("ArrowDown");
    h.press("ArrowDown");
    assert_eq!(h.selected(), item_id(3));
}

#[test]
fn an_item_s_actions_win_over_the_view_s() {
    let mut items: Vec<_> = (1..=2).map(list_item).collect();
    items[1].actions = vec![Action {
        id: "only".into(),
        title: "Only".into(),
        shortcut: None,
    }];
    let mut h = Harness::view(list_tree(items));
    h.press("ArrowDown");
    let snapshot = h.snapshot();
    assert_eq!(snapshot.panel.actions.len(), 1);
    assert_eq!(snapshot.footer.primary_label, "Only");
}

#[test]
fn enter_in_a_list_runs_the_selected_item_s_primary_action() {
    let mut h = Harness::view(list_tree((1..=3).map(list_item).collect()));
    h.press("ArrowDown");
    h.press("Enter");
    let runs = h.runs();
    assert_eq!(runs.len(), 1);
    assert_eq!(
        (runs[0].item_id.as_str(), runs[0].action_id.as_str()),
        ("item-2", "open")
    );
}

#[test]
fn enter_on_a_list_item_without_actions_does_nothing() {
    let mut item = list_item(1);
    item.actions.clear();
    let mut h = Harness::view(list_tree(vec![item]));
    h.press("Enter");
    assert!(h.runs().is_empty());
    assert_eq!(h.snapshot().footer.primary_label, "Select");
}

#[test]
fn clicking_a_list_row_picks_and_runs_it() {
    let mut h = Harness::view(list_tree((1..=3).map(list_item).collect()));
    h.intent(Intent::Activate {
        id: "item-3".into(),
    });
    assert_eq!(h.selected(), item_id(3));
    assert_eq!(h.runs()[0].item_id, "item-3");
}

#[test]
fn enter_in_a_detail_view_runs_the_view_s_primary_action_on_no_item() {
    let mut h = Harness::view(detail_tree("Working…"));
    assert_eq!(h.snapshot().focus, Focus::Detail);
    assert_eq!(h.snapshot().footer.status, "Loading…");
    h.press("Enter");
    let runs = h.runs();
    assert_eq!(runs.len(), 1);
    assert_eq!(
        (runs[0].item_id.as_str(), runs[0].action_id.as_str()),
        ("", "open")
    );
    assert_eq!(runs[0].values, None);
}

#[test]
fn enter_in_a_form_submits_its_primary_action_with_the_values() {
    let mut h = Harness::view(mixed_form());
    assert_eq!(h.snapshot().focus, Focus::Form);
    h.intent(Intent::FieldChanged {
        id: "name".into(),
        value: "Morning note revised".into(),
    });
    h.press("Enter");
    let runs = h.runs();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].item_id, "fixture-form");
    assert_eq!(runs[0].action_id, "save");
    let values = runs[0].values.clone().unwrap();
    assert_eq!(values.len(), 4);
    assert_eq!(values["name"], "Morning note revised");
    assert_eq!(values["password"], "fixture-only");
    assert_eq!(values["enabled"], "true");
    assert_eq!(values["template"], "Hello {{name}}");
}

// Forms

#[test]
fn a_form_seeds_values_from_its_tree_and_reseeds_on_replacement() {
    let mut h = Harness::view(mixed_form());
    let ViewBody::Form(form) = h.view_body() else {
        panic!()
    };
    assert_eq!(form.fields[0].value, "Morning note");
    h.intent(Intent::FieldChanged {
        id: "name".into(),
        value: "typed".into(),
    });
    let ViewBody::Form(typed) = h.view_body() else {
        panic!()
    };
    assert_eq!(typed.revision, form.revision);
    h.render(
        1,
        form_tree(vec![field("first", FieldKind::Text, "Initial value")]),
    );
    let ViewBody::Form(replaced) = h.view_body() else {
        panic!()
    };
    assert!(replaced.revision > form.revision);
    assert_eq!(replaced.fields.len(), 1);
    assert_eq!(replaced.fields[0].value, "Initial value");
}

#[test]
fn each_first_field_kind_submits_string_values() {
    for (kind, initial, edited) in [
        (FieldKind::Text, "Initial value", "Initial value edited"),
        (FieldKind::Password, "Initial value", "Initial value edited"),
        (FieldKind::Template, "Initial value", "Initial value edited"),
        (FieldKind::Toggle, "true", "false"),
    ] {
        let template = kind == FieldKind::Template;
        let mut h = Harness::view(mixed_form());
        h.render(1, form_tree(vec![field("first", kind, initial)]));
        h.intent(Intent::FieldChanged {
            id: "first".into(),
            value: edited.into(),
        });
        if template {
            h.command("Enter");
        } else {
            h.press("Enter");
        }
        let runs = h.runs();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].values.clone().unwrap()["first"], edited);
    }
}

#[test]
fn plain_enter_belongs_to_a_multiline_field_and_command_enter_submits_once() {
    let h = Harness::view(form_tree(vec![field(
        "template",
        FieldKind::Template,
        "Hello {{name}}",
    )]));
    let snapshot = h.snapshot();
    assert!(snapshot.footer.confirm_with_command);
    let enter = snapshot
        .bindings
        .iter()
        .find(|b| b.chord == Chord::plain("Enter"))
        .unwrap();
    assert!(!enter.in_multiline);
    let command_enter = snapshot
        .bindings
        .iter()
        .find(|b| b.chord == Chord::command("Enter"))
        .unwrap();
    assert!(command_enter.in_multiline);

    let mut h = h;
    h.intent(Intent::FieldChanged {
        id: "template".into(),
        value: "Hello {{name}}\nSecond line".into(),
    });
    h.command("Enter");
    let runs = h.runs();
    assert_eq!(runs.len(), 1);
    assert_eq!(
        runs[0].values.clone().unwrap()["template"],
        "Hello {{name}}\nSecond line"
    );
}

#[test]
fn a_form_without_a_template_confirms_with_plain_enter() {
    let h = Harness::view(form_tree(vec![field("name", FieldKind::Text, "x")]));
    assert!(!h.snapshot().footer.confirm_with_command);
}

#[test]
fn template_fields_are_inspected_on_arrival_and_on_every_edit() {
    let mut h = Harness::root(1);
    let effects = h.event(Event::Rendered {
        owner: "fixture".into(),
        invocation: 1,
        tree: mixed_form(),
    });
    assert_eq!(
        effects,
        vec![Effect::InspectTemplate {
            field: "template".into(),
            source: "Hello {{name}}".into(),
        }]
    );
    let ViewBody::Form(form) = h.view_body() else {
        panic!()
    };
    assert_eq!(form.fields[3].help.as_deref(), Some("Nothing to fill in"));
    assert_eq!(form.fields[0].help, None);

    h.event(Event::TemplateInspected {
        field: "template".into(),
        source: "Hello {{name}}".into(),
        inspection: TemplateInspection {
            arguments: vec!["name".into()],
            error: None,
        },
    });
    let ViewBody::Form(form) = h.view_body() else {
        panic!()
    };
    assert_eq!(form.fields[3].help.as_deref(), Some("Will ask for: name"));
    assert!(form.fields[3].help_emphasized);

    let effects = h.intent(Intent::FieldChanged {
        id: "template".into(),
        value: "Hello {{".into(),
    });
    assert_eq!(
        effects,
        vec![Effect::InspectTemplate {
            field: "template".into(),
            source: "Hello {{".into(),
        }]
    );
    assert!(h
        .intent(Intent::FieldChanged {
            id: "name".into(),
            value: "y".into(),
        })
        .is_empty());
}

#[test]
fn an_inspection_of_text_no_longer_in_the_field_is_dropped() {
    let mut h = Harness::view(form_tree(vec![field("template", FieldKind::Template, "a")]));
    h.intent(Intent::FieldChanged {
        id: "template".into(),
        value: "Hello {{".into(),
    });
    h.event(Event::TemplateInspected {
        field: "template".into(),
        source: "a".into(),
        inspection: TemplateInspection {
            arguments: vec!["stale".into()],
            error: None,
        },
    });
    h.event(Event::TemplateInspected {
        field: "template".into(),
        source: "Hello {{".into(),
        inspection: TemplateInspection {
            arguments: Vec::new(),
            error: Some("Couldn't read template. Check the placeholders.".into()),
        },
    });
    let ViewBody::Form(form) = h.view_body() else {
        panic!()
    };
    assert_eq!(
        form.fields[0].error.as_deref(),
        Some("Couldn't read template. Check the placeholders.")
    );
}

// Action panel

fn panel_flows() -> Vec<(&'static str, Harness)> {
    vec![
        ("root", Harness::root(18)),
        (
            "list",
            Harness::view(list_tree((1..=18).map(list_item).collect())),
        ),
        ("form", Harness::view(mixed_form())),
        ("detail", Harness::view(detail_tree("Working…"))),
    ]
}

#[test]
fn the_panel_owns_selection_and_confirms_once() {
    for (scene, mut h) in panel_flows() {
        let lists = matches!(scene, "root" | "list");
        if lists {
            h.press("ArrowDown");
        }
        let parent = h.selected();
        h.command("k");
        let snapshot = h.snapshot();
        assert!(snapshot.panel.open, "{scene}");
        assert_eq!(snapshot.focus, Focus::Panel);
        let first = snapshot.panel.actions[0].id.clone();
        assert_eq!(h.panel_selected(), Some(first), "{scene}");
        h.press("ArrowUp");
        assert_eq!(
            h.panel_selected(),
            Some(snapshot.panel.actions[0].id.clone())
        );
        h.press("ArrowDown");
        let second = snapshot.panel.actions[1].id.clone();
        assert_eq!(h.panel_selected(), Some(second.clone()));
        h.press("Enter");
        assert!(!h.snapshot().panel.open);
        let runs = h.runs();
        assert_eq!(runs.len(), 1, "{scene}");
        assert_eq!(runs[0].action_id, second);
        let expected_item = match scene {
            "root" => "result-2",
            "list" => "item-2",
            "form" => "fixture-form",
            _ => "",
        };
        assert_eq!(runs[0].item_id, expected_item, "{scene}");
        assert_eq!(h.selected(), parent);

        h.intent(Intent::TogglePanel);
        assert_eq!(
            h.panel_selected(),
            Some(snapshot.panel.actions[0].id.clone())
        );
        h.press("Escape");
        assert!(!h.snapshot().panel.open);
        assert_eq!(h.count(&Effect::Dismiss), 0, "{scene}");
        assert_eq!(h.count(&Effect::CancelInvocation), 0, "{scene}");
    }
}

#[test]
fn the_panel_binds_only_its_own_keys() {
    let mut h = Harness::root(3);
    h.command("k");
    let chords: Vec<_> = h.snapshot().bindings.into_iter().map(|b| b.chord).collect();
    assert_eq!(
        chords,
        ["ArrowUp", "ArrowDown", "Enter", "Escape"].map(Chord::plain)
    );
}

#[test]
fn a_click_on_a_panel_action_runs_it() {
    let mut h = Harness::root(3);
    h.intent(Intent::TogglePanel);
    h.intent(Intent::Activate { id: "copy".into() });
    assert_eq!(h.runs()[0].action_id, "copy");
    assert!(!h.snapshot().panel.open);
}

#[test]
fn a_click_on_a_row_while_the_panel_is_open_does_nothing() {
    let mut h = Harness::root(3);
    h.command("k");
    h.intent(Intent::Activate {
        id: "result-2".into(),
    });
    assert!(h.runs().is_empty());
    assert!(h.snapshot().panel.open);
    h.intent(Intent::ClosePanel);
    assert!(!h.snapshot().panel.open);
    assert_eq!(h.selected(), id(1));
}

#[test]
fn no_actions_means_no_panel() {
    for (scene, mut h) in panel_flows() {
        match scene {
            "root" => {
                let mut item = result(1);
                item.actions.clear();
                h.results("", vec![item]);
            }
            "list" => {
                let mut item = list_item(1);
                item.actions.clear();
                h.render(1, list_tree(vec![item]));
            }
            "form" => {
                let mut tree = mixed_form();
                if let View::Form(form) = &mut tree.view {
                    form.actions.clear();
                }
                h.render(1, tree);
            }
            _ => {
                let mut tree = detail_tree("x");
                if let View::Detail(detail) = &mut tree.view {
                    detail.actions.clear();
                }
                h.render(1, tree);
            }
        }
        assert!(h.snapshot().panel.actions.is_empty(), "{scene}");
        h.command("k");
        h.intent(Intent::TogglePanel);
        assert!(!h.snapshot().panel.open, "{scene}");
        assert!(h.runs().is_empty());
    }
}

#[test]
fn removing_every_action_closes_the_open_panel() {
    let mut h = Harness::root(18);
    h.command("k");
    let mut item = result(1);
    item.actions.clear();
    h.results("", vec![item]);
    let snapshot = h.snapshot();
    assert!(!snapshot.panel.open);
    assert_eq!(snapshot.focus, Focus::RootInput);
    assert!(h.runs().is_empty());
}

#[test]
fn panel_selection_survives_replacement_of_unchanged_actions() {
    let long: Vec<Action> = (1..=24)
        .map(|n| Action {
            id: format!("action-{n}"),
            title: format!("Open action {n}"),
            shortcut: None,
        })
        .collect();
    let items: Vec<_> = results(18)
        .into_iter()
        .map(|item| ResultItem {
            actions: long.clone(),
            ..item
        })
        .collect();
    let mut h = Harness::new();
    h.results("", items.clone());
    h.command("k");
    h.press("ArrowDown");
    h.results("", items);
    assert_eq!(h.panel_selected().as_deref(), Some("action-2"));
    for _ in 0..30 {
        h.press("ArrowDown");
    }
    assert_eq!(h.panel_selected().as_deref(), Some("action-24"));
}

#[test]
fn a_reset_while_the_panel_is_open_closes_it_and_the_next_open_starts_fresh() {
    for scene in ["root", "list"] {
        let mut h = if scene == "root" {
            Harness::root(18)
        } else {
            Harness::view(list_tree((1..=18).map(list_item).collect()))
        };
        h.command("k");
        h.press("ArrowDown");
        h.event(Event::Reset);
        h.event(Event::Failed("Couldn't paste. Try again.".into()));
        h.results("", results(18));
        h.event(Event::Activated {
            held: false,
            probe: None,
        });
        let snapshot = h.snapshot();
        assert!(!snapshot.panel.open);
        assert_eq!(
            snapshot.failure.as_deref(),
            Some("Couldn't paste. Try again.")
        );
        assert_eq!(snapshot.focus, Focus::RootInput);
        h.command("k");
        assert_eq!(h.panel_selected().as_deref(), Some("open"));
        h.press("Escape");
        assert!(!h.snapshot().panel.open);
    }
}

#[test]
fn the_panel_keeps_a_carried_failure() {
    let mut h = Harness::root(18);
    h.event(Event::Failed("Couldn't paste. Try again.".into()));
    h.command("k");
    h.press("Escape");
    assert!(h.snapshot().failure.is_some());
    h.typed("Result");
    assert!(h.snapshot().failure.is_none());
}

// Escape, shortcuts, failures, status, bindings

#[test]
fn escape_clears_the_query_then_dismisses() {
    let mut h = Harness::root(18);
    h.typed("Res");
    let revision = h.root_screen().query.revision;
    let effects = h.press("Escape");
    assert_eq!(effects, vec![Effect::Search(String::new())]);
    let root = h.root_screen();
    assert_eq!(root.query.text, "");
    assert!(root.query.revision > revision);
    assert_eq!(h.press("Escape"), vec![Effect::Dismiss]);
}

#[test]
fn escape_in_a_view_pops_it_before_touching_the_query() {
    let mut h = Harness::root(18);
    h.typed("Res");
    h.render(1, detail_tree("x"));
    h.render(2, list_tree(vec![list_item(1)]));
    h.press("Escape");
    assert!(matches!(h.view_body(), ViewBody::Detail(_)));
    h.press("Escape");
    assert_eq!(h.root_screen().query.text, "Res");
    assert_eq!(h.count(&Effect::CancelInvocation), 2);
}

#[test]
fn an_action_shortcut_runs_on_the_selection() {
    let mut h = Harness::root(18);
    h.press("ArrowDown");
    let chord = Chord {
        key: "c".into(),
        command: true,
        alt: false,
        shift: false,
    };
    h.key(chord.clone());
    let runs = h.runs();
    assert_eq!(runs.len(), 1);
    assert_eq!(
        (runs[0].item_id.as_str(), runs[0].action_id.as_str()),
        ("result-2", "copy")
    );
    assert_eq!(runs[0].query.as_deref(), Some(""));

    let mut h = Harness::view(detail_tree("x"));
    h.key(chord);
    assert_eq!(h.runs()[0].action_id, "copy");
}

#[test]
fn bindings_hold_the_shortcuts_of_the_current_selection() {
    let mut items = results(2);
    items[1].actions = vec![Action {
        id: "pin".into(),
        title: "Pin".into(),
        shortcut: Some(Shortcut {
            key: "P".into(),
            modifiers: vec![Modifier::Cmd, Modifier::Shift],
        }),
    }];
    let mut h = Harness::new();
    h.results("", items);
    let copy = Chord::command("c");
    let pin = Chord {
        key: "p".into(),
        command: true,
        alt: false,
        shift: true,
    };
    let chords = |h: &Harness| -> Vec<Chord> {
        h.snapshot().bindings.into_iter().map(|b| b.chord).collect()
    };
    assert!(chords(&h).contains(&copy));
    assert!(!chords(&h).contains(&pin));
    h.press("ArrowDown");
    assert!(!chords(&h).contains(&copy));
    assert!(chords(&h).contains(&pin));
    for key in ["Escape", "ArrowUp", "ArrowDown", "Enter"] {
        assert!(chords(&h).contains(&Chord::plain(key)));
    }
    assert!(chords(&h).contains(&Chord::command("k")));
}

#[test]
fn a_failure_survives_reset_either_way_round_and_clears_on_typing_acting_or_escape() {
    for before_reset in [true, false] {
        for clear in ["typing", "action", "Escape"] {
            let mut h = Harness::root(18);
            if before_reset {
                h.event(Event::Failed("Couldn't paste. Try again.".into()));
            }
            h.event(Event::Reset);
            if !before_reset {
                h.event(Event::Failed("Couldn't paste. Try again.".into()));
            }
            h.results("", results(18));
            h.event(Event::Activated {
                held: false,
                probe: None,
            });
            assert!(h.snapshot().failure.is_some());
            assert_eq!(h.snapshot().focus, Focus::RootInput);
            match clear {
                "typing" => {
                    h.typed("Result");
                }
                "action" => {
                    h.press("Enter");
                }
                _ => {
                    h.press("Escape");
                }
            }
            assert!(h.snapshot().failure.is_none(), "{before_reset} {clear}");
        }
    }
}

#[test]
fn a_running_command_shows_in_the_footer_until_it_fails_or_resets() {
    let mut h = Harness::root(18);
    let mut command = result(1);
    command.title = "Clipboard History".into();
    command.actions.clear();
    h.results("", vec![command]);
    let effects = h.press("Enter");
    assert_eq!(
        effects,
        vec![Effect::InvokeCommand {
            command_id: "result-1".into(),
            query: String::new(),
        }]
    );
    let snapshot = h.snapshot();
    assert_eq!(snapshot.footer.status, "Running Clipboard History…");
    assert!(h.root_screen().busy);
    h.event(Event::Failed("Couldn't paste. Try again.".into()));
    assert_eq!(h.snapshot().footer.status, "");
    assert!(h.snapshot().failure.is_some());
    h.press("Enter");
    assert_eq!(h.snapshot().footer.status, "Running Clipboard History…");
    h.event(Event::Reset);
    assert_eq!(h.snapshot().footer.status, "");
    h.typed("Result");
    assert_eq!(h.snapshot().footer.status, "");
}

#[test]
fn a_view_arriving_ends_the_running_status() {
    let mut h = Harness::new();
    let mut command = result(1);
    command.actions.clear();
    h.results("", vec![command]);
    h.press("Enter");
    h.render(3, detail_tree("x"));
    assert!(h.launcher.working.is_none());
}

#[test]
fn a_copy_puts_the_text_on_the_clipboard_and_dismisses() {
    let mut h = Harness::root(3);
    h.press("Enter");
    let request = h.runs()[0].clone();
    let effects = h.finish(request, ActionOutcome::CopyToClipboard("text".into()));
    assert_eq!(
        effects,
        vec![Effect::CopyText("text".into()), Effect::Dismiss]
    );
}

#[test]
fn a_root_action_answering_with_a_view_opens_it_for_that_extension() {
    let mut h = Harness::root(3);
    h.press("Enter");
    let request = h.runs()[0].clone();
    h.finish(request, ActionOutcome::Replaced(Box::new(mixed_form())));
    assert!(matches!(h.view_body(), ViewBody::Form(_)));
    h.press("Enter");
    assert_eq!(h.runs()[1].extension_id, "fixture");
}

#[test]
fn a_root_removal_searches_again() {
    let mut h = Harness::root(3);
    h.typed("Res");
    h.press("Enter");
    let request = h.runs()[0].clone();
    let effects = h.finish(
        request,
        ActionOutcome::Removed(Box::new(list_tree(Vec::new()))),
    );
    assert_eq!(effects, vec![Effect::Search("Res".into())]);
}

#[test]
fn a_view_action_answering_with_a_view_replaces_the_top() {
    let mut h = Harness::view(list_tree((1..=3).map(list_item).collect()));
    h.press("Enter");
    let request = h.runs()[0].clone();
    h.finish(
        request,
        ActionOutcome::Removed(Box::new(list_tree(vec![list_item(2)]))),
    );
    assert_eq!(h.launcher.stack.len(), 1);
    assert_eq!(h.selected(), item_id(2));
}

#[test]
fn a_failed_action_shows_its_message() {
    let mut h = Harness::root(3);
    h.press("Enter");
    let request = h.runs()[0].clone();
    h.finish(request, ActionOutcome::Failed("Nope".into()));
    assert_eq!(h.snapshot().failure.as_deref(), Some("Nope"));
}

#[test]
fn blur_dismisses_and_ready_searches_and_finishes_warmup() {
    let mut h = Harness::new();
    assert_eq!(
        h.intent(Intent::Ready),
        vec![Effect::Search(String::new()), Effect::FinishWarmup]
    );
    assert_eq!(h.intent(Intent::Blur), vec![Effect::Dismiss]);
    assert_eq!(
        h.intent(Intent::Painted { probe: 7 }),
        vec![Effect::ReportPaint(7)]
    );
}

#[test]
fn the_snapshot_carries_the_build_kind() {
    assert!(Launcher::new(true).snapshot().development);
    assert!(!Launcher::new(false).snapshot().development);
}

// Hold

#[test]
fn held_activation_keeps_query_and_selection_and_refreshes_results() {
    let mut h = Harness::root(18);
    h.typed("Result 1");
    h.results("Result 1", results(18));
    h.press("ArrowDown");
    let before = h.searches("Result 1");
    let count = h.snapshot().activation.count;
    let revision = h.root_screen().query.revision;
    h.event(Event::Activated {
        held: true,
        probe: None,
    });
    assert_eq!(h.searches("Result 1"), before + 1);
    let snapshot = h.snapshot();
    assert_eq!(snapshot.activation.count, count + 1);
    assert_eq!(snapshot.focus, Focus::RootInput);
    let root = h.root_screen();
    assert_eq!(
        (root.query.text.as_str(), root.query.revision),
        ("Result 1", revision)
    );
    assert_eq!(h.selected(), id(2));
}

#[test]
fn held_activation_leaves_a_pushed_view_in_place() {
    let mut h = Harness::view(mixed_form());
    h.event(Event::Activated {
        held: true,
        probe: Some(3),
    });
    let snapshot = h.snapshot();
    assert_eq!(snapshot.focus, Focus::Form);
    assert_eq!(snapshot.activation.probe, Some(3));
    assert!(matches!(snapshot.screen, Screen::View(_)));
}

#[test]
fn a_reset_starts_at_an_empty_prompt() {
    let mut h = Harness::root(18);
    h.typed("Result 1");
    h.render(1, detail_tree("x"));
    let effects = h.event(Event::Reset);
    assert_eq!(effects, vec![Effect::Search(String::new())]);
    h.results("", results(18));
    h.event(Event::Activated {
        held: false,
        probe: None,
    });
    let snapshot = h.snapshot();
    assert_eq!(snapshot.focus, Focus::RootInput);
    let root = h.root_screen();
    assert_eq!(root.query.text, "");
    assert_eq!(root.sections[0].items.len(), 18);
}

#[test]
fn a_reset_does_not_abandon_the_running_command() {
    let mut h = Harness::view(detail_tree("x"));
    h.event(Event::Reset);
    h.render(1, detail_tree("after work"));
    assert!(matches!(h.view_body(), ViewBody::Detail(_)));
}
