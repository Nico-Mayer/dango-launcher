use crate::extension::ActionOutcome;
use crate::protocol::{
    Action, DetailView, FieldKind, FormField, FormView, ItemDetail, ListItem, ListView, Modifier,
    Shortcut, View, ViewTree,
};

use super::snapshot::{DetailBody, Focus, ListBody, RootScreen, Screen, ViewBody};
use super::state::BusyMarks;
use super::*;

fn actions() -> Vec<Action> {
    vec![
        Action {
            id: "open".into(),
            title: "Open".into(),
            shortcut: None,
            ..Default::default()
        },
        Action {
            id: "copy".into(),
            title: "Copy".into(),
            shortcut: Some(Shortcut {
                key: "c".into(),
                modifiers: vec![Modifier::Ctrl],
            }),
            ..Default::default()
        },
    ]
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
        ..Default::default()
    }
}

fn list_tree(items: Vec<ListItem>) -> ViewTree {
    ViewTree::new(View::List(ListView {
        loading: false,
        empty_state: None,
        items,
        ..Default::default()
    }))
}

fn detail_tree(markdown: &str) -> ViewTree {
    ViewTree::new(View::Detail(DetailView {
        markdown: markdown.into(),
        loading: true,
        actions: actions(),
        ..Default::default()
    }))
}

fn field(id: &str, kind: FieldKind, value: &str) -> FormField {
    FormField {
        id: id.into(),
        label: id.into(),
        kind,
        value: Some(value.into()),
        ..Default::default()
    }
}

fn form_tree(fields: Vec<FormField>) -> ViewTree {
    let mut form_actions = vec![Action {
        id: "save".into(),
        title: "Save".into(),
        shortcut: None,
        ..Default::default()
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
        let mut launcher = Launcher::new();
        launcher.apply(Input::Event(Event::Activated {
            held: false,
            probe: None,
        }));
        Self {
            launcher,
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
            complete: true,
        });
    }

    fn render(&mut self, invocation: u64, tree: ViewTree) {
        self.event(Event::Rendered {
            owner: "fixture".into(),
            context: Context {
                title: "Fixture Command".into(),
                icon: None,
            },
            invocation,
            tree: Box::new(tree),
        });
    }

    fn finish(&mut self, request: ActionRequest, outcome: ActionOutcome) -> Vec<Effect> {
        self.event(Event::ActionFinished { request, outcome })
    }

    /// Lets the busy marks' next wait pass, as the shell's timer would.
    fn wake_busy(&mut self) {
        match self.launcher.busy_marks {
            BusyMarks::Pending { id }
            | BusyMarks::Shown {
                id, held: false, ..
            } => {
                self.event(Event::BusyMarksWoke { id });
            }
            _ => {}
        }
    }

    fn snapshot(&self) -> Snapshot {
        self.launcher.snapshot()
    }

    fn toast(&self, style: ToastStyle) -> Option<String> {
        self.snapshot()
            .toast
            .filter(|toast| toast.style == style)
            .map(|toast| toast.text)
    }

    fn failure(&self) -> Option<String> {
        self.toast(ToastStyle::Failure)
    }

    fn status(&self) -> String {
        self.toast(ToastStyle::Busy).unwrap_or_default()
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
                ViewBody::Grid(grid) => grid.selected,
                _ => None,
            },
            Screen::Error(_) => None,
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
fn nothing_found_waits_for_every_provider() {
    let mut h = Harness::root(3);
    h.typed("zzz");
    h.event(Event::Results {
        query: "zzz".into(),
        items: Vec::new(),
        complete: false,
    });
    let root = h.root_screen();
    assert_eq!(root.empty, None);
    assert!(root.settling);
    h.results("zzz", Vec::new());
    let root = h.root_screen();
    assert!(root.empty.is_some());
    assert!(!root.settling);
}

#[test]
fn an_empty_search_says_what_found_nothing() {
    let mut h = Harness::root(3);
    assert_eq!(h.root_screen().empty, None);
    h.typed("zzz");
    h.results("zzz", Vec::new());
    assert_eq!(
        h.root_screen().empty.map(|empty| empty.title).as_deref(),
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
    assert_eq!(h.list().sections[0].items[0].id, "item-3");
    assert_eq!(h.selected(), item_id(2));
    let items: Vec<_> = (1..=18).filter(|n| *n != 2).map(list_item).collect();
    h.render(1, list_tree(items));
    assert_eq!(h.selected(), item_id(1));
}

#[test]
fn the_pane_carries_the_image_and_plain_text() {
    let item = ListItem {
        detail: Some(ItemDetail {
            image: Some("C:/img.png".into()),
            text: Some("# raw".into()),
            ..Default::default()
        }),
        ..list_item(1)
    };
    let mut tree = list_tree(vec![item]);
    if let View::List(list) = &mut tree.view {
        list.detail_pane = true;
    }
    let h = Harness::view(tree);
    let pane = h.list().pane.expect("a pane");
    assert_eq!(pane.image.as_deref(), Some("C:/img.png"));
    assert_eq!(pane.text.as_deref(), Some("# raw"));
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
            markdown: "first".into(),
            ..Default::default()
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
    assert!(matches!(snapshot.screen, Screen::Error(_)));
    assert_eq!(snapshot.focus, Focus::Nothing);
    // Escape, and Tab held so focus stays put.
    assert_eq!(snapshot.bindings.len(), 3);
    assert!(h.press("Escape").is_empty());
    assert!(matches!(h.snapshot().screen, Screen::View(_)));
}

#[test]
fn a_version_1_tree_is_unsupported() {
    let mut h = Harness::root(3);
    let mut tree = detail_tree("Old");
    tree.protocol_version = 1;
    h.render(1, tree);
    let Screen::Error(text) = h.snapshot().screen else {
        panic!("expected the unsupported view");
    };
    assert_eq!(text.title, "This view needs a newer version of Dango");
    assert_eq!(text.description.as_deref(), Some("Press Escape to go back"));
}

#[test]
fn an_action_in_a_view_goes_to_the_owner_of_the_view() {
    let mut h = Harness::new();
    h.event(Event::Rendered {
        owner: "clipboard".into(),
        context: Context::default(),
        invocation: 4,
        tree: Box::new(list_tree(vec![list_item(1)])),
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
    assert_eq!(list.sections[0].items.len(), 10);
    assert_eq!(list.query.text, "item 1");
    assert_eq!(list.query.revision, revision);
    h.typed("item 12");
    assert_eq!(h.selected(), item_id(12));
    assert_eq!(h.root_screen_query(), "");
    h.typed("nothing");
    let list = h.list();
    assert_eq!(list.empty.unwrap().title, "No results for “nothing”");
}

fn shown_ids(list: &ListBody) -> Vec<String> {
    list.sections
        .iter()
        .flat_map(|section| section.items.iter().map(|item| item.id.clone()))
        .collect()
}

fn item_ids(range: std::ops::RangeInclusive<usize>) -> Vec<String> {
    range.map(|n| format!("item-{n}")).collect()
}

#[test]
fn a_long_list_shows_the_first_fifty_and_filtering_reaches_the_rest() {
    let mut h = Harness::view(list_tree((1..=500).map(list_item).collect()));
    assert_eq!(shown_ids(&h.list()), item_ids(1..=50));
    h.typed("item 300");
    assert_eq!(shown_ids(&h.list()), item_ids(300..=300));
    assert_eq!(h.selected(), item_id(300));
}

#[test]
fn the_limit_counts_across_sections_and_drops_a_section_left_empty() {
    let item = |n: usize, section: &str| ListItem {
        section: Some(section.into()),
        ..list_item(n)
    };
    let section = |id: &str| crate::protocol::Section {
        id: id.into(),
        title: id.into(),
        subtitle: None,
    };
    let items = (1..=40)
        .map(|n| item(n, "First"))
        .chain((41..=80).map(|n| item(n, "Second")))
        .chain([item(81, "Third")])
        .collect();
    let h = Harness::view(ViewTree::new(View::List(ListView {
        sections: vec![section("First"), section("Second"), section("Third")],
        items,
        ..Default::default()
    })));
    let list = h.list();
    assert_eq!(section_titles(&list), ["First", "Second"]);
    assert_eq!(list.sections[0].items.len(), 40);
    assert_eq!(shown_ids(&list), item_ids(1..=50));
}

#[test]
fn a_grid_shows_fifty_tiles() {
    let h = Harness::view(grid_tree(5, 80));
    let ViewBody::Grid(grid) = h.view_body() else {
        panic!("expected a grid");
    };
    let ids: Vec<&str> = grid
        .sections
        .iter()
        .flat_map(|section| section.items.iter().map(|tile| tile.id.as_str()))
        .collect();
    assert_eq!(ids.len(), 50);
    assert_eq!(ids.last(), Some(&"tile-50"));
}

#[test]
fn arrow_down_stays_on_the_last_shown_item_and_its_panel_opens() {
    let mut items: Vec<_> = (1..=500).map(list_item).collect();
    items[49].actions = vec![Action {
        id: "fiftieth".into(),
        title: "Fiftieth".into(),
        ..Default::default()
    }];
    let mut h = Harness::view(list_tree(items));
    for _ in 0..60 {
        h.press("ArrowDown");
    }
    assert_eq!(h.selected(), item_id(50));
    h.command("k");
    let panel = h.snapshot().panel;
    assert!(panel.open);
    assert_eq!(panel.actions.len(), 1);
    assert_eq!(panel.actions[0].id, "fiftieth");
}

impl Harness {
    fn root_screen_query(&self) -> String {
        self.launcher.query.clone()
    }
}

fn sectioned_tree() -> ViewTree {
    let item = |n: usize, section: &str| ListItem {
        section: Some(section.into()),
        ..list_item(n)
    };
    let section = |id: &str| crate::protocol::Section {
        id: id.into(),
        title: id.into(),
        subtitle: None,
    };
    ViewTree::new(View::List(ListView {
        // Declared out of item order: sections decide what comes first.
        sections: vec![section("Today"), section("Yesterday")],
        items: vec![
            item(4, "Yesterday"),
            item(1, "Today"),
            item(2, "Today"),
            item(3, "Today"),
        ],
        ..Default::default()
    }))
}

fn section_titles(list: &ListBody) -> Vec<String> {
    list.sections
        .iter()
        .filter_map(|section| section.title.clone())
        .collect()
}

#[test]
fn sections_are_shown_in_their_order() {
    let h = Harness::view(sectioned_tree());
    let list = h.list();
    assert_eq!(section_titles(&list), ["Today", "Yesterday"]);
    let ids: Vec<&str> = list.sections[0]
        .items
        .iter()
        .map(|i| i.id.as_str())
        .collect();
    assert_eq!(ids, ["item-1", "item-2", "item-3"]);
    assert_eq!(h.selected(), item_id(1));
}

#[test]
fn filtering_hides_a_section_with_no_match() {
    let mut h = Harness::view(sectioned_tree());
    h.typed("item 2");
    assert_eq!(section_titles(&h.list()), ["Today"]);
}

#[test]
fn arrow_down_crosses_into_the_next_section() {
    let mut h = Harness::view(sectioned_tree());
    for _ in 0..3 {
        h.press("ArrowDown");
    }
    assert_eq!(h.selected(), item_id(4));
}

#[test]
fn a_keyword_matches_without_being_the_title() {
    let terminal = ListItem {
        title: "Terminal".into(),
        keywords: vec!["shell".into()],
        ..list_item(1)
    };
    let mut h = Harness::view(list_tree(vec![terminal, list_item(2)]));
    h.typed("shell");
    let list = h.list();
    assert_eq!(list.sections.len(), 1);
    assert_eq!(list.sections[0].items[0].title, "Terminal");
    assert_eq!(list.sections[0].items.len(), 1);
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
        ..Default::default()
    }];
    let mut h = Harness::view(list_tree(items));
    h.press("ArrowDown");
    let snapshot = h.snapshot();
    assert_eq!(snapshot.panel.actions.len(), 1);
    assert_eq!(snapshot.footer.primary_label.as_deref(), Some("Only"));
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
    assert_eq!(h.snapshot().footer.primary_label, None);
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
fn enter_on_an_empty_view_runs_its_first_action() {
    let mut tree = list_tree(Vec::new());
    if let View::List(list) = &mut tree.view {
        list.empty_state = Some(crate::protocol::EmptyState {
            title: "No matches".into(),
            icon: Some("icon:sparkles".into()),
            actions: vec![Action {
                id: "web".into(),
                title: "Search the web".into(),
                ..Default::default()
            }],
            ..Default::default()
        });
    }
    let mut h = Harness::view(tree);
    let list = h.list();
    assert_eq!(list.empty.unwrap().icon.as_deref(), Some("icon:sparkles"));
    assert_eq!(
        h.snapshot().footer.primary_label.as_deref(),
        Some("Search the web")
    );
    h.press("Enter");
    let runs = h.runs();
    assert_eq!(runs.len(), 1);
    assert_eq!(
        (runs[0].item_id.as_str(), runs[0].action_id.as_str()),
        ("", "web")
    );
}

#[test]
fn enter_in_a_detail_view_runs_the_view_s_primary_action_on_no_item() {
    let mut h = Harness::view(detail_tree("Working…"));
    assert_eq!(h.snapshot().focus, Focus::Detail);
    h.wake_busy();
    assert_eq!(h.status(), "Loading…");
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

// Grid

fn grid_tree(columns: u8, count: usize) -> ViewTree {
    ViewTree::new(View::Grid(crate::protocol::GridView {
        columns,
        aspect: 1.0,
        items: (1..=count)
            .map(|n| crate::protocol::GridItem {
                id: format!("tile-{n}"),
                title: format!("Tile {n}"),
                content: crate::protocol::TileContent::Glyph {
                    glyph: "★".into()
                },
                actions: actions(),
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    }))
}

fn tile(n: usize) -> Option<String> {
    Some(format!("tile-{n}"))
}

fn grid_from(columns: u8, count: usize, selected: usize) -> Harness {
    let mut h = Harness::view(grid_tree(columns, count));
    for _ in 1..selected {
        h.press("ArrowRight");
    }
    assert_eq!(h.selected(), tile(selected));
    h
}

#[test]
fn a_grid_outside_four_to_eight_columns_is_refused() {
    let h = Harness::view(grid_tree(2, 4));
    let Screen::Error(text) = h.snapshot().screen else {
        panic!("expected the invalid view");
    };
    assert_eq!(text.title, "This view can't be shown");
}

#[test]
fn an_action_answering_with_an_invalid_grid_is_refused() {
    let mut h = Harness::view(list_tree((1..=3).map(list_item).collect()));
    h.press("Enter");
    let request = h.runs()[0].clone();
    h.finish(request, ActionOutcome::Replaced(Box::new(grid_tree(2, 4))));
    assert_refused(&h);
}

#[test]
fn a_command_pushing_again_after_an_action_replaced_its_view_replaces_it_again() {
    let mut h = Harness::view(list_tree((1..=3).map(list_item).collect()));
    h.press("Enter");
    let request = h.runs()[0].clone();
    h.finish(
        request,
        ActionOutcome::Replaced(Box::new(list_tree((1..=2).map(list_item).collect()))),
    );
    h.render(1, list_tree((1..=2).map(list_item).collect()));
    assert_eq!(h.launcher.stack.len(), 1);
}

fn shortcut_action(key: &str, modifiers: Vec<Modifier>) -> Action {
    Action {
        id: "shortcut".into(),
        title: "Shortcut".into(),
        shortcut: Some(Shortcut {
            key: key.into(),
            modifiers,
        }),
        ..Default::default()
    }
}

fn assert_refused(h: &Harness) {
    let Screen::Error(text) = h.snapshot().screen else {
        panic!("expected the invalid view");
    };
    assert_eq!(text.title, "This view can't be shown");
}

#[test]
fn a_shift_shortcut_on_a_digit_is_refused() {
    let mut item = list_item(1);
    item.actions = vec![shortcut_action("1", vec![Modifier::Shift])];
    assert_refused(&Harness::view(list_tree(vec![item])));
}

#[test]
fn an_alt_shortcut_on_a_symbol_is_refused() {
    let tree = ViewTree::new(View::List(ListView {
        empty_state: Some(crate::protocol::EmptyState {
            title: "Nothing".into(),
            description: None,
            icon: None,
            actions: vec![shortcut_action("/", vec![Modifier::Alt])],
        }),
        ..Default::default()
    }));
    assert_refused(&Harness::view(tree));
}

#[test]
fn a_ctrl_shift_shortcut_on_a_letter_runs() {
    let mut h = Harness::view(ViewTree::new(View::Detail(DetailView {
        markdown: "x".into(),
        actions: vec![shortcut_action("P", vec![Modifier::Ctrl, Modifier::Shift])],
        ..Default::default()
    })));
    h.key(Chord {
        key: "p".into(),
        command: true,
        alt: false,
        shift: true,
    });
    assert_eq!(h.runs()[0].action_id, "shortcut");
}

#[test]
fn shift_or_alt_shortcuts_on_named_keys_run() {
    let mut h = Harness::view(ViewTree::new(View::Detail(DetailView {
        markdown: "x".into(),
        actions: vec![
            shortcut_action("Enter", vec![Modifier::Shift]),
            Action {
                id: "up".into(),
                ..shortcut_action("ArrowUp", vec![Modifier::Alt])
            },
        ],
        ..Default::default()
    })));
    h.key(Chord {
        shift: true,
        ..Chord::plain("Enter")
    });
    h.key(Chord {
        alt: true,
        ..Chord::plain("ArrowUp")
    });
    let runs = h.runs();
    assert_eq!(runs[0].action_id, "shortcut");
    assert_eq!(runs[1].action_id, "up");
}

#[test]
fn a_root_action_answering_with_a_refused_shortcut_is_refused() {
    let mut h = Harness::root(3);
    h.press("Enter");
    let request = h.runs()[0].clone();
    let mut item = list_item(1);
    item.actions = vec![shortcut_action("1", vec![Modifier::Shift])];
    h.finish(
        request,
        ActionOutcome::Replaced(Box::new(list_tree(vec![item]))),
    );
    assert_refused(&h);
}

#[test]
fn a_five_column_grid_shows_its_tiles() {
    let h = Harness::view(grid_tree(5, 12));
    let ViewBody::Grid(grid) = h.view_body() else {
        panic!("expected a grid");
    };
    assert_eq!(grid.columns, 5);
    assert_eq!(grid.sections[0].items.len(), 12);
    assert_eq!(grid.selected, tile(1));
    assert_eq!(h.snapshot().focus, Focus::ViewInput);
}

#[test]
fn a_grid_moves_right_and_wraps_into_the_next_row() {
    let mut h = grid_from(5, 12, 2);
    h.press("ArrowRight");
    assert_eq!(h.selected(), tile(3));
    let mut h = grid_from(5, 12, 5);
    h.press("ArrowRight");
    assert_eq!(h.selected(), tile(6));
}

#[test]
fn a_grid_moves_down_by_a_row() {
    let mut h = grid_from(5, 12, 2);
    h.press("ArrowDown");
    assert_eq!(h.selected(), tile(7));
    h.press("ArrowUp");
    assert_eq!(h.selected(), tile(2));
}

#[test]
fn moving_down_into_a_short_row_takes_its_last_tile() {
    let mut h = grid_from(5, 7, 5);
    h.press("ArrowDown");
    assert_eq!(h.selected(), tile(7));
}

#[test]
fn the_last_tile_stays() {
    let mut h = grid_from(5, 7, 7);
    h.press("ArrowRight");
    assert_eq!(h.selected(), tile(7));
    h.press("ArrowDown");
    assert_eq!(h.selected(), tile(7));
}

#[test]
fn a_grid_binds_left_and_right() {
    let h = Harness::view(grid_tree(5, 3));
    let bound = |key| {
        h.snapshot()
            .bindings
            .iter()
            .any(|binding| binding.chord == Chord::plain(key))
    };
    assert!(bound("ArrowLeft"));
    assert!(bound("ArrowRight"));
}

// Filter dropdown

fn filtered_tree() -> ViewTree {
    let option = |id: &str, title: &str| crate::protocol::FieldOption {
        id: id.into(),
        title: title.into(),
    };
    let mut item = list_item(1);
    item.actions.push(Action {
        id: "delete".into(),
        title: "Delete entry".into(),
        style: crate::protocol::ActionStyle::Danger,
        ..Default::default()
    });
    ViewTree::new(View::List(ListView {
        filter: Some(crate::protocol::Filter {
            options: vec![option("all", "All types"), option("text", "Text only")],
            selected: "all".into(),
        }),
        items: vec![item, list_item(2)],
        ..Default::default()
    }))
}

#[test]
fn the_panel_offers_the_filter_before_danger_actions() {
    let mut h = Harness::view(filtered_tree());
    h.command("k");
    let panel = h.snapshot().panel;
    let titles: Vec<&str> = panel.actions.iter().map(|a| a.title.as_str()).collect();
    assert_eq!(
        titles,
        ["Open", "Copy", "All types", "Text only", "Delete entry"]
    );
    assert_eq!(panel.actions[3].section.as_deref(), Some("Filter"));
    assert_eq!(panel.checked.as_deref(), Some(panel.actions[2].id.as_str()));
}

#[test]
fn confirming_a_filter_option_runs_the_filter_and_closes_the_panel() {
    let mut h = Harness::view(filtered_tree());
    h.typed("item");
    h.command("k");
    for _ in 0..3 {
        h.press("ArrowDown");
    }
    let effects = h.press("Enter");
    let [Effect::RunAction(request)] = effects.as_slice() else {
        panic!("expected one action, got {effects:?}");
    };
    assert_eq!(request.action_id, crate::protocol::FILTER_ACTION);
    assert_eq!(request.item_id, "text");
    assert!(!h.snapshot().panel.open);
    let list = h.list();
    assert_eq!(list.query.text, "item");
    assert_eq!(list.filter.unwrap().selected, "text");
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
fn a_dropdown_submits_the_chosen_identifier() {
    let folder = FormField {
        options: ["work", "personal"]
            .map(|id| crate::protocol::FieldOption {
                id: id.into(),
                title: id.into(),
            })
            .to_vec(),
        value: None,
        ..field("folder", FieldKind::Dropdown, "")
    };
    let mut h = Harness::view(form_tree(vec![
        folder,
        field("break", FieldKind::Separator, ""),
    ]));
    h.press("Enter");
    assert_eq!(
        h.runs()[0]
            .values
            .as_ref()
            .unwrap()
            .get("folder")
            .map(String::as_str),
        Some("work")
    );
    assert!(!h.runs()[0].values.as_ref().unwrap().contains_key("break"));
    h.intent(Intent::FieldChanged {
        id: "folder".into(),
        value: "personal".into(),
    });
    h.press("Enter");
    assert_eq!(
        h.runs()[1]
            .values
            .as_ref()
            .unwrap()
            .get("folder")
            .map(String::as_str),
        Some("personal")
    );
}

#[test]
fn a_text_area_keeps_its_line_break() {
    let mut h = Harness::view(form_tree(vec![field("notes", FieldKind::Textarea, "")]));
    assert!(h.snapshot().footer.confirm_with_command);
    h.intent(Intent::FieldChanged {
        id: "notes".into(),
        value: "first
second"
            .into(),
    });
    h.command("Enter");
    assert_eq!(
        h.runs()[0]
            .values
            .as_ref()
            .unwrap()
            .get("notes")
            .map(String::as_str),
        Some(
            "first
second"
        )
    );
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
        context: Context::default(),
        invocation: 1,
        tree: Box::new(mixed_form()),
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
    let shift_tab = Chord {
        shift: true,
        ..Chord::plain("Tab")
    };
    let mut expected = ["ArrowUp", "ArrowDown", "Enter", "Escape", "Tab"]
        .map(Chord::plain)
        .to_vec();
    expected.push(shift_tab);
    assert_eq!(chords, expected);
}

#[test]
fn danger_actions_come_last_under_their_section() {
    let manage = |id: &str, title: &str, style| Action {
        id: id.into(),
        title: title.into(),
        section: Some("Manage".into()),
        style,
        ..Default::default()
    };
    let mut item = result(1);
    item.actions = vec![
        actions().remove(0),
        manage(
            "delete",
            "Delete entry",
            crate::protocol::ActionStyle::Danger,
        ),
        manage("pin", "Pin entry", crate::protocol::ActionStyle::Normal),
    ];
    let mut h = Harness::new();
    h.results("", vec![item]);
    h.command("k");
    let panel = h.snapshot().panel;
    let titles: Vec<&str> = panel.actions.iter().map(|a| a.title.as_str()).collect();
    assert_eq!(titles, ["Open", "Pin entry", "Delete entry"]);
    assert_eq!(panel.actions[2].section.as_deref(), Some("Manage"));
    h.press("ArrowDown");
    h.press("ArrowDown");
    assert_eq!(h.snapshot().panel.selected.as_deref(), Some("delete"));
}

#[test]
fn the_panel_filter_narrows_actions_and_leaves_the_query() {
    let mut h = Harness::root(3);
    h.typed("res");
    h.results("res", results(3));
    h.command("k");
    h.intent(Intent::PanelFilterChanged { text: "co".into() });
    let panel = h.snapshot().panel;
    let titles: Vec<&str> = panel.actions.iter().map(|a| a.title.as_str()).collect();
    assert_eq!(titles, ["Copy"]);
    assert_eq!(panel.selected.as_deref(), Some("copy"));
    assert_eq!(h.launcher.query, "res");

    h.intent(Intent::PanelFilterChanged { text: "zz".into() });
    assert!(h.snapshot().panel.actions.is_empty());
    assert!(h.snapshot().panel.open);
    assert!(h.press("Enter").is_empty());

    h.press("Escape");
    assert_eq!(h.snapshot().focus, Focus::RootInput);
    h.command("k");
    assert_eq!(h.snapshot().panel.filter, "");
    assert_eq!(h.snapshot().panel.actions.len(), 2);
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
            ..Default::default()
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
        h.event(Event::Hidden);
        h.event(Event::Reset);
        h.results("", results(18));
        h.event(Event::Activated {
            held: false,
            probe: None,
        });
        let snapshot = h.snapshot();
        assert!(!snapshot.panel.open);
        assert_eq!(snapshot.focus, Focus::RootInput);
        h.command("k");
        assert_eq!(h.panel_selected().as_deref(), Some("open"));
        h.press("Escape");
        assert!(!h.snapshot().panel.open);
    }
}

#[test]
fn opening_the_panel_clears_a_failure() {
    let mut h = Harness::root(18);
    h.event(Event::Failed("Couldn't paste".into()));
    h.command("k");
    assert!(h.failure().is_none());
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
        ..Default::default()
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
fn a_failure_around_a_hide_goes_to_the_hud_either_way_round() {
    for before_hide in [true, false] {
        let mut h = Harness::root(18);
        if before_hide {
            h.event(Event::Failed("Couldn't paste".into()));
        }
        let mut effects = h.event(Event::Hidden);
        h.event(Event::Reset);
        if !before_hide {
            effects = h.event(Event::Failed("Couldn't paste".into()));
        }
        assert_eq!(
            hud_text(&h).as_deref(),
            Some("Couldn't paste"),
            "{before_hide}"
        );
        assert!(matches!(
            effects.as_slice(),
            [Effect::ExpireToast {
                style: ToastStyle::Failure,
                ..
            }]
        ));
        h.results("", results(18));
        h.event(Event::Activated {
            held: false,
            probe: None,
        });
        assert!(h.failure().is_none(), "{before_hide}");
        assert_eq!(hud_text(&h), None);
    }
}

#[test]
fn a_failure_while_held_goes_to_the_hud_and_not_into_the_held_view() {
    let mut h = Harness::view(detail_tree("x"));
    h.event(Event::Hidden);
    h.event(Event::Failed("That command is no longer available".into()));
    assert_eq!(
        hud_text(&h).as_deref(),
        Some("That command is no longer available")
    );
    h.event(Event::Activated {
        held: true,
        probe: None,
    });
    assert!(h.failure().is_none());
    assert!(matches!(h.view_body(), ViewBody::Detail(_)));
}

#[test]
fn a_running_command_shows_a_busy_toast_until_it_fails_or_resets() {
    let mut h = Harness::root(18);
    let mut command = result(1);
    command.title = "Clipboard History".into();
    command.actions.clear();
    h.results("", vec![command]);
    let effects = h.press("Enter");
    assert_eq!(
        effects[0],
        Effect::InvokeCommand {
            command_id: "result-1".into(),
            query: String::new(),
        }
    );
    h.wake_busy();
    assert_eq!(h.status(), "Running Clipboard History…");
    h.event(Event::Failed("Couldn't paste".into()));
    assert_eq!(h.status(), "");
    assert!(h.failure().is_some());
    h.press("Enter");
    h.wake_busy();
    assert_eq!(h.status(), "Running Clipboard History…");
    h.event(Event::Reset);
    assert_eq!(h.status(), "");
    h.typed("Result");
    assert_eq!(h.status(), "");
}

#[test]
fn a_hotkey_failure_while_held_keeps_the_running_status() {
    let mut h = Harness::root(18);
    let mut command = result(1);
    command.title = "Clipboard History".into();
    command.actions.clear();
    h.results("", vec![command]);
    h.press("Enter");
    h.event(Event::Hidden);
    h.event(Event::HotkeyFailed("Can't open Zed".into()));
    assert_eq!(hud_text(&h).as_deref(), Some("Can't open Zed"));
    h.event(Event::Activated {
        held: true,
        probe: None,
    });
    h.wake_busy();
    assert_eq!(h.status(), "Running Clipboard History…");
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
    assert_eq!(h.failure().as_deref(), Some("Nope"));
}

fn finish_with_toast(h: &mut Harness, text: &str) -> u64 {
    h.press("Enter");
    let request = h.runs().last().unwrap().clone();
    let effects = h.finish(request, ActionOutcome::Toast(text.into()));
    match effects.as_slice() {
        [Effect::ExpireToast { id, .. }] => *id,
        other => panic!("expected one expiry, got {other:?}"),
    }
}

#[test]
fn a_success_toast_shows_until_it_expires() {
    let mut h = Harness::view(list_tree((1..=3).map(list_item).collect()));
    let id = finish_with_toast(&mut h, "Entry pinned");
    assert_eq!(
        h.toast(ToastStyle::Success).as_deref(),
        Some("Entry pinned")
    );
    assert_eq!(h.launcher.stack.len(), 1);
    h.event(Event::ToastExpired { id });
    assert!(h.snapshot().toast.is_none());
}

#[test]
fn a_stale_expiry_leaves_a_newer_toast() {
    let mut h = Harness::view(list_tree((1..=3).map(list_item).collect()));
    let first = finish_with_toast(&mut h, "Entry pinned");
    finish_with_toast(&mut h, "Entry unpinned");
    h.event(Event::ToastExpired { id: first });
    assert_eq!(
        h.toast(ToastStyle::Success).as_deref(),
        Some("Entry unpinned")
    );
}

#[test]
fn a_failure_toast_stays_until_a_key() {
    let mut h = Harness::view(list_tree((1..=3).map(list_item).collect()));
    h.event(Event::Failed("Nope".into()));
    h.event(Event::ToastExpired { id: u64::MAX });
    assert_eq!(h.failure().as_deref(), Some("Nope"));
    h.press("ArrowDown");
    assert!(h.failure().is_none());
}

#[test]
fn the_footer_names_the_extension_at_root_and_the_command_in_a_view() {
    let mut h = Harness::root(3);
    assert_eq!(h.snapshot().footer.context.title, "Fixture");
    h.render(1, list_tree(vec![list_item(1)]));
    assert_eq!(h.snapshot().footer.context.title, "Fixture Command");
}

#[test]
fn ready_searches_and_painted_reports() {
    let mut h = Harness::new();
    assert_eq!(h.intent(Intent::Ready), vec![Effect::Search(String::new())]);
    assert_eq!(
        h.intent(Intent::Painted { probe: 7 }),
        vec![Effect::ReportPaint(7)]
    );
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

// HUD

fn copy_from_root(h: &mut Harness) -> Vec<Effect> {
    h.press("Enter");
    let request = h.runs().last().unwrap().clone();
    h.finish(request, ActionOutcome::CopyToClipboard("x".into()))
}

fn hud_text(h: &Harness) -> Option<String> {
    h.snapshot().hud.map(|hud| hud.text)
}

fn hud_expiry(effects: &[Effect]) -> u64 {
    match effects {
        [Effect::ExpireToast { id, .. }] => *id,
        other => panic!("expected one expiry, got {other:?}"),
    }
}

#[test]
fn a_copy_dismisses_and_shows_its_hud_only_once_hidden() {
    let mut h = Harness::root(3);
    let effects = copy_from_root(&mut h);
    assert_eq!(effects, vec![Effect::CopyText("x".into()), Effect::Dismiss]);
    assert!(h.snapshot().toast.is_none());
    assert_eq!(hud_text(&h), None);
    let effects = h.event(Event::Hidden);
    let id = hud_expiry(&effects);
    assert_eq!(hud_text(&h).as_deref(), Some("Copied to clipboard"));
    assert_eq!(
        effects,
        vec![Effect::ExpireToast {
            id,
            style: ToastStyle::Success
        }]
    );
    h.event(Event::ToastExpired { id });
    assert_eq!(hud_text(&h), None);
}

#[test]
fn an_action_that_copied_itself_dismisses_and_shows_the_copy_hud() {
    let mut h = Harness::root(3);
    h.press("Enter");
    let request = h.runs().last().unwrap().clone();
    let effects = h.finish(request, ActionOutcome::Copied);
    assert_eq!(effects, vec![Effect::Dismiss]);
    h.event(Event::Hidden);
    assert_eq!(hud_text(&h).as_deref(), Some("Copied to clipboard"));
}

#[test]
fn a_stale_expiry_leaves_a_newer_hud() {
    let mut h = Harness::root(3);
    copy_from_root(&mut h);
    let first = hud_expiry(&h.event(Event::Hidden));
    h.event(Event::Activated {
        held: false,
        probe: None,
    });
    copy_from_root(&mut h);
    let second = hud_expiry(&h.event(Event::Hidden));
    assert_ne!(first, second);
    h.event(Event::ToastExpired { id: first });
    assert_eq!(hud_text(&h).as_deref(), Some("Copied to clipboard"));
    assert_eq!(h.snapshot().hud.unwrap().id, second);
}

#[test]
fn a_newer_hud_replaces_the_one_showing() {
    let mut h = Harness::root(3);
    copy_from_root(&mut h);
    let first = hud_expiry(&h.event(Event::Hidden));
    let effects = copy_from_root(&mut h);
    let second = hud_expiry(&effects[2..]);
    assert_ne!(first, second);
    assert_eq!(h.snapshot().hud.unwrap().id, second);
}

#[test]
fn showing_the_launcher_removes_the_hud() {
    let mut h = Harness::root(3);
    copy_from_root(&mut h);
    h.event(Event::Hidden);
    h.event(Event::Activated {
        held: false,
        probe: None,
    });
    assert_eq!(hud_text(&h), None);
    h.event(Event::Hidden);
    assert_eq!(hud_text(&h), None);
}

fn running(h: &mut Harness) {
    let mut command = result(1);
    command.title = "Clipboard History".into();
    command.actions.clear();
    h.results("", vec![command]);
    h.press("Enter");
}

fn done_tree() -> ViewTree {
    ViewTree::new(View::Detail(DetailView {
        markdown: "Done".into(),
        actions: actions(),
        ..Default::default()
    }))
}

fn marks_id(h: &Harness) -> Option<u64> {
    match h.launcher.busy_marks {
        BusyMarks::Pending { id } | BusyMarks::Shown { id, .. } => Some(id),
        BusyMarks::Hidden => None,
    }
}

#[test]
fn a_fast_command_shows_no_busy_toast() {
    let mut h = Harness::new();
    running(&mut h);
    assert_eq!(h.status(), "", "nothing before the delay");
    let late = marks_id(&h).unwrap();
    h.render(3, done_tree());
    assert_eq!(h.launcher.busy_marks, BusyMarks::Hidden);
    h.event(Event::BusyMarksWoke { id: late });
    assert_eq!(h.status(), "");
}

#[test]
fn running_passes_to_loading_without_a_gap() {
    let mut h = Harness::new();
    running(&mut h);
    h.wake_busy();
    assert_eq!(h.status(), "Running Clipboard History…");
    let id = marks_id(&h);
    h.render(3, detail_tree("Working"));
    assert_eq!(h.status(), "Loading…");
    assert_eq!(marks_id(&h), id, "the same marks carry on");
}

#[test]
fn busy_marks_stay_until_their_hold_passes() {
    let mut h = Harness::view(detail_tree("Working"));
    h.wake_busy();
    assert_eq!(h.status(), "Loading…");
    h.render(1, done_tree());
    assert_eq!(h.status(), "Loading…", "shown its minimum time");
    h.wake_busy();
    assert_eq!(h.status(), "");
    assert_eq!(h.launcher.busy_marks, BusyMarks::Hidden);
}

#[test]
fn a_hold_that_passes_while_busy_keeps_the_marks() {
    let mut h = Harness::view(detail_tree("Working"));
    h.wake_busy();
    h.wake_busy();
    assert_eq!(h.status(), "Loading…");
    h.render(1, done_tree());
    assert_eq!(h.status(), "");
}

#[test]
fn a_streamed_view_keeps_its_busy_marks() {
    let mut h = Harness::view(detail_tree("Work"));
    h.wake_busy();
    let shown = h.launcher.busy_marks.clone();
    for text in ["Worki", "Workin", "Working"] {
        let effects = h.event(Event::Rendered {
            owner: "fixture".into(),
            context: Context {
                title: "Fixture Command".into(),
                icon: None,
            },
            invocation: 1,
            tree: Box::new(detail_tree(text)),
        });
        assert!(!effects
            .iter()
            .any(|effect| matches!(effect, Effect::WakeBusyMarks { .. })));
    }
    assert_eq!(h.launcher.busy_marks, shown);
}

#[test]
fn leaving_a_view_clears_its_busy_marks_at_once() {
    let mut h = Harness::view(detail_tree("Working"));
    h.wake_busy();
    assert_eq!(h.status(), "Loading…");
    h.press("Escape");
    assert_eq!(h.status(), "");
    assert_eq!(h.launcher.busy_marks, BusyMarks::Hidden);
}

#[test]
fn a_stale_wake_is_ignored() {
    let mut h = Harness::view(detail_tree("Working"));
    let pending = h.launcher.busy_marks.clone();
    let id = marks_id(&h).unwrap();
    h.event(Event::BusyMarksWoke { id: id + 1 });
    assert_eq!(h.launcher.busy_marks, pending);
    assert_eq!(h.status(), "");
}

#[test]
fn every_wait_for_busy_marks_takes_a_new_id() {
    let mut h = Harness::view(detail_tree("Working"));
    let first = marks_id(&h).unwrap();
    h.render(1, done_tree());
    h.render(1, detail_tree("Again"));
    let second = marks_id(&h).unwrap();
    assert_ne!(first, second);
    h.event(Event::BusyMarksWoke { id: first });
    assert_eq!(h.status(), "", "the old wake does not show the new marks");
}

#[test]
fn a_reset_clears_the_busy_marks_at_once() {
    let mut h = Harness::view(detail_tree("Working"));
    h.wake_busy();
    assert_eq!(h.status(), "Loading…");
    h.event(Event::Reset);
    assert_eq!(h.launcher.busy_marks, BusyMarks::Hidden);
    assert_eq!(h.status(), "");
}

#[test]
fn a_failed_command_does_not_come_back_as_running() {
    let mut h = Harness::new();
    running(&mut h);
    h.wake_busy();
    assert_eq!(h.status(), "Running Clipboard History…");
    h.event(Event::Failed("Couldn't paste".into()));
    h.typed("R");
    assert_eq!(h.status(), "");
}

fn progress_tree() -> ViewTree {
    use crate::protocol::{ProgressView, Status, Step};
    ViewTree::new(View::Progress(ProgressView {
        title: "Dango 0.5.0".into(),
        subtitle: "You have 0.4.2".into(),
        value: Some(0.62),
        detail: "18.4 MB of 29.6 MB".into(),
        steps: vec![
            Step {
                title: "Download".into(),
                status: Status::Working,
            },
            Step {
                title: "Restart".into(),
                status: Status::Queued,
            },
        ],
        actions: actions(),
    }))
}

#[test]
fn enter_on_a_progress_view_runs_its_primary_action() {
    let mut h = Harness::view(progress_tree());
    assert_eq!(h.snapshot().focus, Focus::Detail);
    h.press("Enter");
    let runs = h.runs();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].action_id, "open");
}

#[test]
fn escape_leaves_a_progress_view() {
    let mut h = Harness::view(progress_tree());
    h.press("Escape");
    assert!(matches!(h.snapshot().screen, Screen::Root(_)));
}

#[test]
fn an_action_answering_back_leaves_the_view() {
    let mut h = Harness::new();
    h.results("", results(3));
    h.render(1, progress_tree());
    let request = ActionRequest {
        origin: Origin::View,
        extension_id: "fixture".into(),
        item_id: String::new(),
        action_id: "open".into(),
        values: None,
        query: None,
    };
    let effects = h.finish(request, ActionOutcome::Back);
    assert!(effects.contains(&Effect::CancelInvocation));
    assert!(matches!(h.snapshot().screen, Screen::Root(_)));
    h.render(1, progress_tree());
    assert!(
        matches!(h.snapshot().screen, Screen::Root(_)),
        "a later render from the left view is dropped"
    );
}

fn retry_request() -> ActionRequest {
    ActionRequest {
        origin: Origin::Root,
        extension_id: "dango.updates".into(),
        item_id: "update".into(),
        action_id: "install".into(),
        values: None,
        query: None,
    }
}

fn retry_toast(h: &mut Harness) {
    h.launcher.toast = Some(super::state::HeldToast {
        id: 9_000,
        style: ToastStyle::Failure,
        text: "Couldn't download the update. Check your connection and try again.".into(),
        action: Some(ToastAction {
            title: "Retry".into(),
            shortcut: Shortcut {
                key: "r".into(),
                modifiers: vec![Modifier::Cmd],
            },
            request: retry_request(),
        }),
    });
}

#[test]
fn a_toast_chord_runs_its_action_once_and_clears_it() {
    let mut h = Harness::root(3);
    retry_toast(&mut h);
    let effects = h.command("r");
    assert_eq!(effects, vec![Effect::RunAction(retry_request())]);
    assert!(h.failure().is_none());
    let again = h.command("r");
    assert!(!again.iter().any(
        |effect| matches!(effect, Effect::RunAction(request) if request.action_id == "install")
    ));
}

#[test]
fn another_key_clears_a_toast_without_running_its_action() {
    let mut h = Harness::root(3);
    retry_toast(&mut h);
    let effects = h.press("ArrowDown");
    assert!(h.failure().is_none());
    assert!(!effects
        .iter()
        .any(|effect| matches!(effect, Effect::RunAction(_))));
}

#[test]
fn a_toast_chord_beats_an_item_action_with_the_same_chord() {
    let mut h = Harness::new();
    let mut item = result(1);
    item.actions.push(Action {
        id: "reveal".into(),
        title: "Show in Explorer".into(),
        shortcut: Some(Shortcut {
            key: "r".into(),
            modifiers: vec![Modifier::Cmd],
        }),
        ..Default::default()
    });
    h.results("", vec![item]);
    retry_toast(&mut h);
    assert!(h
        .snapshot()
        .bindings
        .iter()
        .any(|binding| binding.chord == Chord::command("r")));
    let effects = h.command("r");
    assert_eq!(effects, vec![Effect::RunAction(retry_request())]);
}

#[test]
fn the_panel_keeps_a_toast_chord_from_running() {
    let mut h = Harness::root(3);
    h.command("k");
    retry_toast(&mut h);
    assert!(h.launcher.toast_action().is_none());
    let effects = h.command("r");
    assert!(!effects.contains(&Effect::RunAction(retry_request())));
}
