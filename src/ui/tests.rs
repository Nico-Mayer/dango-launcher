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
        answer: false,
        web_search: false,
        hotkey: None,
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
            asked: false,
        }));
        Self {
            launcher,
            effects: Vec::new(),
        }
    }

    /// Root with the empty-query list opened, as most tests want it.
    fn root(count: usize) -> Self {
        let mut harness = Self::new();
        harness.results("", results(count));
        harness.press("ArrowDown");
        harness.effects.clear();
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

    fn ticker(&self, style: TickerStyle) -> Option<String> {
        self.snapshot()
            .ticker
            .filter(|ticker| ticker.style == style)
            .map(|ticker| ticker.text)
    }

    fn failure(&self) -> Option<String> {
        self.ticker(TickerStyle::Failure)
    }

    fn status(&self) -> String {
        self.ticker(TickerStyle::Busy).unwrap_or_default()
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

/// Root with results for the empty query and the list still closed.
fn bare_pill(count: usize) -> Harness {
    let mut h = Harness::new();
    h.results("", results(count));
    h
}

#[test]
fn a_bare_pill_shows_and_selects_nothing() {
    let h = bare_pill(5);
    assert!(h.root_screen().sections.is_empty());
    assert_eq!(h.selected(), None);
}

#[test]
fn a_bare_pill_ignores_enter_and_the_panel_and_hidden_shortcuts() {
    let mut h = bare_pill(5);
    assert!(h.press("Enter").is_empty());
    h.command("k");
    assert!(!h.snapshot().panel.open);
    let copy = Chord::command("c");
    assert!(!h.snapshot().bindings.iter().any(|b| b.chord == copy));
    assert!(h.key(copy).is_empty());
}

#[test]
fn arrow_down_on_a_bare_pill_opens_the_list_at_the_first_row() {
    let mut h = bare_pill(5);
    h.press("ArrowDown");
    assert_eq!(h.root_screen().sections[0].items.len(), 5);
    assert_eq!(h.selected(), id(1));
}

#[test]
fn arrow_up_on_a_bare_pill_does_nothing() {
    let mut h = bare_pill(5);
    h.press("ArrowUp");
    assert!(h.root_screen().sections.is_empty());
    assert_eq!(h.selected(), None);
}

#[test]
fn typing_after_opening_shows_ranked_results_from_the_top() {
    let mut h = bare_pill(5);
    h.press("ArrowDown");
    h.press("ArrowDown");
    h.typed("R");
    h.results("R", results(3));
    assert_eq!(h.root_screen().sections.len(), 1);
    assert_eq!(h.selected(), id(1));
}

#[test]
fn the_first_keystroke_drops_the_empty_query_list() {
    let mut h = bare_pill(5);
    h.press("ArrowDown");
    h.typed("R");
    let root = h.root_screen();
    assert!(root.sections.is_empty());
    assert!(root.bare, "the pill stays alone until a result for R lands");
    h.results("R", results(2));
    assert!(!h.root_screen().bare);
}

#[test]
fn the_card_stays_while_a_query_without_results_grows() {
    let mut h = bare_pill(5);
    h.typed("x");
    h.results("x", Vec::new());
    assert!(h.root_screen().empty.is_some());
    h.typed("xy");
    assert!(
        !h.root_screen().bare,
        "the next key must not drop to the pill"
    );
}

#[test]
fn clearing_the_query_closes_the_list() {
    let mut h = bare_pill(5);
    h.press("ArrowDown");
    h.typed("R");
    h.typed("");
    h.results("", results(5));
    assert!(h.root_screen().sections.is_empty());
}

#[test]
fn a_reset_closes_the_list() {
    let mut h = bare_pill(5);
    h.press("ArrowDown");
    h.event(Event::Hidden);
    h.event(Event::Reset);
    h.results("", results(5));
    assert!(h.root_screen().sections.is_empty());
}

#[test]
fn escape_clears_the_query_then_closes_the_list_then_dismisses() {
    let mut h = bare_pill(5);
    h.press("ArrowDown");
    h.typed("R");
    assert!(h.press("Escape").contains(&Effect::Search(String::new())));
    h.results("", results(5));
    assert!(
        h.root_screen().sections.is_empty(),
        "clearing closes the list"
    );
    h.press("ArrowDown");
    assert!(!h.press("Escape").contains(&Effect::Dismiss));
    assert!(h.root_screen().sections.is_empty());
    assert_eq!(h.press("Escape"), vec![Effect::Dismiss]);
}

#[test]
fn escape_on_the_pinned_card_dismisses() {
    let mut h = Harness::new();
    let mut items = results(3);
    items[0].pinned = true;
    h.results("", items);
    assert_eq!(h.root_screen().sections.len(), 1);
    assert_eq!(h.press("Escape"), vec![Effect::Dismiss]);
}

#[test]
fn root_sections_group_pinned_and_suggested_results() {
    let mut h = Harness::new();
    let mut items = results(5);
    items[3].pinned = true;
    items[0].suggested = true;
    items[1].suggested = true;
    h.results("", items);
    let closed = h.root_screen().sections;
    assert_eq!(closed.len(), 1, "a closed root shows only what is pinned");
    assert_eq!(closed[0].heading.as_deref(), Some("Needs attention"));
    assert_eq!(h.selected(), id(4));
    h.press("ArrowDown");
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
    assert_eq!(h.selected(), id(1), "opening steps past the pinned row");
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
    h.press("ArrowDown");
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
    h.press("ArrowDown");
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
            asked: false,
        });
        let snapshot = h.snapshot();
        assert!(!snapshot.panel.open);
        assert_eq!(snapshot.focus, Focus::RootInput);
        h.press("ArrowDown");
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
    h.press("ArrowDown");
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
fn a_failure_around_a_hide_goes_to_the_notice_either_way_round() {
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
            notice_text(&h).as_deref(),
            Some("Couldn't paste"),
            "{before_hide}"
        );
        assert!(matches!(
            effects.as_slice(),
            [Effect::ExpireTicker {
                style: TickerStyle::Failure,
                ..
            }]
        ));
        h.results("", results(18));
        h.event(Event::Activated {
            held: false,
            probe: None,
            asked: false,
        });
        assert!(h.failure().is_none(), "{before_hide}");
        assert_eq!(notice_text(&h), None);
    }
}

#[test]
fn a_failure_while_held_goes_to_the_notice_and_not_into_the_held_view() {
    let mut h = Harness::view(detail_tree("x"));
    h.event(Event::Hidden);
    h.event(Event::Failed("That command is no longer available".into()));
    assert_eq!(
        notice_text(&h).as_deref(),
        Some("That command is no longer available")
    );
    h.event(Event::Activated {
        held: true,
        probe: None,
        asked: false,
    });
    assert!(h.failure().is_none());
    assert!(matches!(h.view_body(), ViewBody::Detail(_)));
}

#[test]
fn a_running_command_shows_a_busy_ticker_until_it_fails_or_resets() {
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
    assert_eq!(notice_text(&h).as_deref(), Some("Can't open Zed"));
    h.event(Event::Activated {
        held: true,
        probe: None,
        asked: false,
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
    h.press("ArrowDown");
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
    h.results("Res", results(3));
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

fn finish_with_ticker(h: &mut Harness, text: &str) -> u64 {
    h.press("Enter");
    let request = h.runs().last().unwrap().clone();
    let effects = h.finish(request, ActionOutcome::Ticker(text.into()));
    match effects.as_slice() {
        [Effect::ExpireTicker { id, .. }] => *id,
        other => panic!("expected one expiry, got {other:?}"),
    }
}

#[test]
fn a_success_ticker_shows_until_it_expires() {
    let mut h = Harness::view(list_tree((1..=3).map(list_item).collect()));
    let id = finish_with_ticker(&mut h, "Entry pinned");
    assert_eq!(
        h.ticker(TickerStyle::Success).as_deref(),
        Some("Entry pinned")
    );
    assert_eq!(h.launcher.stack.len(), 1);
    h.event(Event::TickerExpired { id });
    assert!(h.snapshot().ticker.is_none());
}

#[test]
fn a_stale_expiry_leaves_a_newer_ticker() {
    let mut h = Harness::view(list_tree((1..=3).map(list_item).collect()));
    let first = finish_with_ticker(&mut h, "Entry pinned");
    finish_with_ticker(&mut h, "Entry unpinned");
    h.event(Event::TickerExpired { id: first });
    assert_eq!(
        h.ticker(TickerStyle::Success).as_deref(),
        Some("Entry unpinned")
    );
}

#[test]
fn a_failure_ticker_stays_until_a_key() {
    let mut h = Harness::view(list_tree((1..=3).map(list_item).collect()));
    h.event(Event::Failed("Nope".into()));
    h.event(Event::TickerExpired { id: u64::MAX });
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
        asked: false,
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
        asked: false,
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
        asked: false,
    });
    let snapshot = h.snapshot();
    assert_eq!(snapshot.focus, Focus::RootInput);
    let root = h.root_screen();
    assert_eq!(root.query.text, "");
    assert!(root.sections.is_empty(), "the list starts closed");
    h.press("ArrowDown");
    assert_eq!(h.root_screen().sections[0].items.len(), 18);
}

#[test]
fn a_reset_does_not_abandon_the_running_command() {
    let mut h = Harness::view(detail_tree("x"));
    h.event(Event::Reset);
    h.render(1, detail_tree("after work"));
    assert!(matches!(h.view_body(), ViewBody::Detail(_)));
}

// Notice

fn copy_from_root(h: &mut Harness) -> Vec<Effect> {
    h.press("Enter");
    let request = h.runs().last().unwrap().clone();
    h.finish(request, ActionOutcome::CopyToClipboard("x".into()))
}

fn notice_text(h: &Harness) -> Option<String> {
    h.snapshot().notice.map(|notice| notice.text)
}

fn notice_expiry(effects: &[Effect]) -> u64 {
    match effects {
        [Effect::ExpireTicker { id, .. }] => *id,
        other => panic!("expected one expiry, got {other:?}"),
    }
}

#[test]
fn a_copy_dismisses_and_shows_its_notice_only_once_hidden() {
    let mut h = Harness::root(3);
    let effects = copy_from_root(&mut h);
    assert_eq!(effects, vec![Effect::CopyText("x".into()), Effect::Dismiss]);
    assert!(h.snapshot().ticker.is_none());
    assert_eq!(notice_text(&h), None);
    let effects = h.event(Event::Hidden);
    let id = notice_expiry(&effects);
    assert_eq!(notice_text(&h).as_deref(), Some("Copied to clipboard"));
    assert_eq!(
        effects,
        vec![Effect::ExpireTicker {
            id,
            style: TickerStyle::Success
        }]
    );
    h.event(Event::TickerExpired { id });
    assert_eq!(notice_text(&h), None);
}

#[test]
fn an_action_that_copied_itself_dismisses_and_shows_the_copy_notice() {
    let mut h = Harness::root(3);
    h.press("Enter");
    let request = h.runs().last().unwrap().clone();
    let effects = h.finish(request, ActionOutcome::Copied);
    assert_eq!(effects, vec![Effect::Dismiss]);
    h.event(Event::Hidden);
    assert_eq!(notice_text(&h).as_deref(), Some("Copied to clipboard"));
}

#[test]
fn a_stale_expiry_leaves_a_newer_notice() {
    let mut h = Harness::root(3);
    copy_from_root(&mut h);
    let first = notice_expiry(&h.event(Event::Hidden));
    h.event(Event::Activated {
        held: false,
        probe: None,
        asked: false,
    });
    copy_from_root(&mut h);
    let second = notice_expiry(&h.event(Event::Hidden));
    assert_ne!(first, second);
    h.event(Event::TickerExpired { id: first });
    assert_eq!(notice_text(&h).as_deref(), Some("Copied to clipboard"));
    assert_eq!(h.snapshot().notice.unwrap().id, second);
}

#[test]
fn a_newer_notice_replaces_the_one_showing() {
    let mut h = Harness::root(3);
    copy_from_root(&mut h);
    let first = notice_expiry(&h.event(Event::Hidden));
    let effects = copy_from_root(&mut h);
    let second = notice_expiry(&effects[2..]);
    assert_ne!(first, second);
    assert_eq!(h.snapshot().notice.unwrap().id, second);
}

#[test]
fn showing_the_launcher_removes_the_notice() {
    let mut h = Harness::root(3);
    copy_from_root(&mut h);
    h.event(Event::Hidden);
    h.event(Event::Activated {
        held: false,
        probe: None,
        asked: false,
    });
    assert_eq!(notice_text(&h), None);
    h.event(Event::Hidden);
    assert_eq!(notice_text(&h), None);
}

fn running(h: &mut Harness) {
    let mut command = result(1);
    command.title = "Clipboard History".into();
    command.actions.clear();
    h.results("", vec![command]);
    h.press("ArrowDown");
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
fn a_fast_command_shows_no_busy_ticker() {
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

fn retry_ticker(h: &mut Harness) {
    h.launcher.ticker = Some(super::state::HeldTicker {
        id: 9_000,
        style: TickerStyle::Failure,
        text: "Couldn't download the update. Check your connection and try again.".into(),
        action: Some(TickerAction {
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
fn a_ticker_chord_runs_its_action_once_and_clears_it() {
    let mut h = Harness::root(3);
    retry_ticker(&mut h);
    let effects = h.command("r");
    assert_eq!(effects, vec![Effect::RunAction(retry_request())]);
    assert!(h.failure().is_none());
    let again = h.command("r");
    assert!(!again.iter().any(
        |effect| matches!(effect, Effect::RunAction(request) if request.action_id == "install")
    ));
}

#[test]
fn another_key_clears_a_ticker_without_running_its_action() {
    let mut h = Harness::root(3);
    retry_ticker(&mut h);
    let effects = h.press("ArrowDown");
    assert!(h.failure().is_none());
    assert!(!effects
        .iter()
        .any(|effect| matches!(effect, Effect::RunAction(_))));
}

#[test]
fn a_ticker_chord_beats_an_item_action_with_the_same_chord() {
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
    retry_ticker(&mut h);
    assert!(h
        .snapshot()
        .bindings
        .iter()
        .any(|binding| binding.chord == Chord::command("r")));
    let effects = h.command("r");
    assert_eq!(effects, vec![Effect::RunAction(retry_request())]);
}

#[test]
fn the_panel_keeps_a_ticker_chord_from_running() {
    let mut h = Harness::root(3);
    h.command("k");
    retry_ticker(&mut h);
    assert!(h.launcher.ticker_action().is_none());
    let effects = h.command("r");
    assert!(!effects.contains(&Effect::RunAction(retry_request())));
}

fn task(id: u64, status: crate::tasks::TaskStatus) -> crate::tasks::TaskState {
    crate::tasks::TaskState {
        id,
        owner: "dango.updates".into(),
        title: "Dango 0.5.0".into(),
        line: format!("Downloading Dango 0.5.{id} · 42%"),
        status,
        progress: Some(0.42),
        steps: Vec::new(),
        subtitle: String::new(),
        detail: String::new(),
        cancel: Some("cancel".into()),
        retry: Some("install".into()),
        show: None,
        item_id: "update".into(),
    }
}

fn report(h: &mut Harness, state: crate::tasks::TaskState) {
    h.event(Event::Task(Box::new(state)));
}

fn running_task() -> crate::tasks::TaskStatus {
    crate::tasks::TaskStatus::Running
}

fn failed_task() -> crate::tasks::TaskStatus {
    crate::tasks::TaskStatus::Failed {
        sentence: "Couldn't download the update. Check your connection and try again.".into(),
    }
}

fn ready_task() -> crate::tasks::TaskStatus {
    crate::tasks::TaskStatus::NeedsInput {
        sentence: "Dango 0.5.0 is ready to restart".into(),
    }
}

#[test]
fn a_running_task_shows_in_the_busy_ticker_at_root() {
    let mut h = Harness::root(3);
    report(&mut h, task(1, running_task()));
    assert_eq!(h.status(), "", "the busy-mark delay applies");
    h.wake_busy();
    assert_eq!(h.status(), "Downloading Dango 0.5.1 · 42%");
}

#[test]
fn the_oldest_running_task_leads_the_busy_ticker() {
    let mut h = Harness::root(3);
    report(&mut h, task(1, running_task()));
    report(&mut h, task(2, running_task()));
    h.wake_busy();
    assert_eq!(h.status(), "Downloading Dango 0.5.1 · 42%");
}

#[test]
fn a_task_waiting_for_input_shows_no_busy_ticker() {
    let mut h = Harness::root(3);
    report(&mut h, task(1, running_task()));
    report(&mut h, task(1, ready_task()));
    h.wake_busy();
    assert_eq!(h.status(), "");
}

#[test]
fn the_cancel_chord_runs_the_owner_s_cancel_action() {
    let mut h = Harness::root(3);
    report(&mut h, task(1, running_task()));
    h.wake_busy();
    let ticker = h.snapshot().ticker.unwrap();
    assert_eq!(ticker.action.as_ref().unwrap().title, "Cancel");
    let effects = h.command(".");
    let Some(Effect::RunAction(request)) = effects.first() else {
        panic!("expected the cancel action, got {effects:?}");
    };
    assert_eq!(
        (request.extension_id.as_str(), request.action_id.as_str()),
        ("dango.updates", "cancel")
    );
}

#[test]
fn a_running_task_draws_no_loading_bar_in_another_view() {
    let mut h = Harness::view(done_tree());
    report(&mut h, task(1, running_task()));
    h.wake_busy();
    assert_eq!(h.status(), "Downloading Dango 0.5.1 · 42%");
    let Screen::View(view) = h.snapshot().screen else {
        panic!("expected a view");
    };
    assert!(!view.busy);
}

#[test]
fn a_task_failing_while_hidden_shows_in_the_notice_and_on_the_next_open() {
    let mut h = Harness::root(3);
    report(&mut h, task(1, running_task()));
    h.event(Event::Hidden);
    report(&mut h, task(1, failed_task()));
    assert!(notice_text(&h).is_some_and(|text| text.starts_with("Couldn't download")));
    h.event(Event::Activated {
        held: false,
        probe: None,
        asked: false,
    });
    let ticker = h.snapshot().ticker.unwrap();
    assert_eq!(ticker.style, TickerStyle::Failure);
    assert_eq!(ticker.action.unwrap().title, "Retry");
}

#[test]
fn a_task_ready_while_hidden_shows_a_success_ticker_on_the_next_open() {
    let mut h = Harness::root(3);
    report(&mut h, task(1, running_task()));
    h.event(Event::Hidden);
    report(&mut h, task(1, ready_task()));
    assert_eq!(
        notice_text(&h).as_deref(),
        Some("Dango 0.5.0 is ready to restart")
    );
    h.event(Event::Activated {
        held: false,
        probe: None,
        asked: false,
    });
    assert_eq!(
        h.ticker(TickerStyle::Success).as_deref(),
        Some("Dango 0.5.0 is ready to restart")
    );
}

#[test]
fn reopening_onto_the_task_s_own_view_shows_no_result_ticker() {
    let mut h = Harness::view(progress_tree());
    let own = |status| crate::tasks::TaskState {
        owner: "fixture".into(),
        ..task(1, status)
    };
    report(&mut h, own(running_task()));
    h.event(Event::Hidden);
    report(&mut h, own(failed_task()));
    h.event(Event::Activated {
        held: true,
        probe: None,
        asked: false,
    });
    assert!(h.failure().is_none());
}

#[test]
fn a_task_ready_while_at_root_shows_a_ticker_at_once() {
    let mut h = Harness::root(3);
    report(&mut h, task(1, running_task()));
    report(&mut h, task(1, ready_task()));
    assert_eq!(
        h.ticker(TickerStyle::Success).as_deref(),
        Some("Dango 0.5.0 is ready to restart")
    );
}

#[test]
fn a_view_from_another_extension_starts_its_own_stack() {
    let mut h = Harness::view(list_tree(vec![list_item(1)]));
    h.event(Event::Rendered {
        owner: "dango.updates".into(),
        context: Context::default(),
        invocation: 7,
        tree: Box::new(progress_tree()),
    });
    assert_eq!(h.launcher.stack.len(), 1);
    h.press("Escape");
    assert!(matches!(h.snapshot().screen, Screen::Root(_)));
}

#[test]
fn a_task_s_lingering_marks_draw_no_loading_bar() {
    let mut h = Harness::view(done_tree());
    report(&mut h, task(1, running_task()));
    h.wake_busy();
    report(&mut h, task(1, crate::tasks::TaskStatus::Cancelled));
    assert!(h.launcher.busy_marks_shown().is_some(), "the marks linger");
    let Screen::View(view) = h.snapshot().screen else {
        panic!("expected a view");
    };
    assert!(!view.busy);
    assert!(h.snapshot().ticker.unwrap().action.is_none());
}

fn answer_item() -> ResultItem {
    ResultItem {
        extension_id: "dango.calculator".into(),
        context: Context {
            title: "Calculator".into(),
            icon: None,
        },
        id: "4".into(),
        title: "4".into(),
        subtitle: None,
        icon: None,
        actions: vec![
            Action {
                id: "copy".into(),
                title: "Copy answer".into(),
                ..Default::default()
            },
            Action {
                id: "paste".into(),
                title: "Paste answer".into(),
                ..Default::default()
            },
        ],
        match_positions: Vec::new(),
        suggested: false,
        pinned: false,
        answer: true,
        web_search: false,
        hotkey: None,
    }
}

fn answered(results: Vec<ResultItem>) -> Harness {
    let mut h = Harness::new();
    h.typed("2+2");
    let mut items = vec![answer_item()];
    items.extend(results);
    h.event(Event::Results {
        query: "2+2".into(),
        items,
        complete: true,
    });
    h
}

#[test]
fn a_query_with_an_answer_selects_the_card() {
    let h = answered(results(2));
    let root = h.root_screen();
    let card = root.answer.expect("the answer card");
    assert_eq!(
        (card.expression.as_str(), card.answer.as_str()),
        ("2+2", "4")
    );
    assert!(card.selected);
    assert!(root
        .sections
        .iter()
        .all(|section| section.items.iter().all(|item| !item.answer)));
    assert_eq!(root.selected.as_deref(), Some("4"));
}

#[test]
fn arrows_move_between_the_card_and_the_results() {
    let mut h = answered(results(2));
    h.press("ArrowDown");
    assert_eq!(h.root_screen().selected.as_deref(), Some("result-1"));
    h.press("ArrowUp");
    assert_eq!(h.root_screen().selected.as_deref(), Some("4"));
    h.press("ArrowUp");
    assert_eq!(h.root_screen().selected.as_deref(), Some("4"));
}

#[test]
fn enter_on_the_card_copies_the_answer() {
    let mut h = answered(results(2));
    let snapshot = h.snapshot();
    assert_eq!(snapshot.footer.context.title, "Calculator");
    assert_eq!(
        snapshot.footer.primary_label.as_deref(),
        Some("Copy answer")
    );
    h.press("Enter");
    let runs = h.runs();
    assert_eq!(runs.len(), 1);
    assert_eq!(
        (
            runs[0].extension_id.as_str(),
            runs[0].item_id.as_str(),
            runs[0].action_id.as_str()
        ),
        ("dango.calculator", "4", "copy")
    );
}

#[test]
fn the_panel_offers_copy_and_paste() {
    let mut h = answered(results(2));
    h.command("k");
    let titles: Vec<String> = h
        .snapshot()
        .panel
        .actions
        .iter()
        .map(|action| action.title.clone())
        .collect();
    assert_eq!(titles, ["Copy answer", "Paste answer"]);
}

#[test]
fn an_answer_alone_shows_no_empty_view() {
    let h = answered(Vec::new());
    let root = h.root_screen();
    assert!(root.answer.is_some());
    assert!(root.empty.is_none());
}

#[test]
fn a_finished_task_offers_to_show_what_it_kept() {
    let mut h = Harness::root(3);
    report(&mut h, task(1, running_task()));
    report(
        &mut h,
        crate::tasks::TaskState {
            show: Some("show".into()),
            ..task(
                1,
                crate::tasks::TaskStatus::Done {
                    sentence: "Answer ready: Summarize".into(),
                },
            )
        },
    );
    let ticker = h.snapshot().ticker.unwrap();
    assert_eq!(ticker.text, "Answer ready: Summarize");
    let action = ticker.action.expect("Show answer");
    assert_eq!(action.title, "Show answer");
    let effects = h.command("Enter");
    let Some(Effect::RunAction(request)) = effects.first() else {
        panic!("expected the show action, got {effects:?}");
    };
    assert_eq!(request.action_id, "show");
}

fn answer_ready(id: u64) -> crate::tasks::TaskState {
    crate::tasks::TaskState {
        owner: "dango.ai".into(),
        show: Some("show".into()),
        ..task(
            id,
            crate::tasks::TaskStatus::Done {
                sentence: "Answer ready: Summarize".into(),
            },
        )
    }
}

fn expires(effects: &[Effect]) -> bool {
    effects
        .iter()
        .any(|effect| matches!(effect, Effect::ExpireTicker { .. }))
}

#[test]
fn a_ticker_with_an_action_waits_for_the_user() {
    let mut h = Harness::root(3);
    report(&mut h, task(1, running_task()));
    let effects = h.event(Event::Task(Box::new(answer_ready(1))));
    assert!(!expires(&effects), "no timer on a ticker you may act on");

    let effects = h.press("ArrowDown");
    assert!(
        !effects
            .iter()
            .any(|effect| matches!(effect, Effect::RunAction(_))),
        "another key does not run the action"
    );
    assert!(h.snapshot().ticker.is_none(), "but it clears the ticker");
}

#[test]
fn a_ticker_without_an_action_still_expires() {
    let mut h = Harness::root(3);
    report(&mut h, task(1, running_task()));
    let effects = h.event(Event::Task(Box::new(task(
        1,
        crate::tasks::TaskStatus::Done {
            sentence: "Dango 0.5.0 is ready to restart".into(),
        },
    ))));
    assert!(expires(&effects));
}

#[test]
fn a_task_finishing_while_hidden_shows_a_success_notice() {
    let mut h = Harness::root(3);
    report(&mut h, task(1, running_task()));
    h.event(Event::Hidden);
    report(&mut h, answer_ready(1));
    let notice = h.snapshot().notice.expect("a notice");
    assert_eq!(notice.style, TickerStyle::Success);
    assert_eq!(notice.text, "Answer ready: Summarize");
}

#[test]
fn a_task_finishing_in_view_shows_no_notice() {
    let mut h = Harness::root(3);
    report(&mut h, task(1, running_task()));
    report(&mut h, answer_ready(1));
    assert!(notice_text(&h).is_none());
}

#[test]
fn needing_input_again_shows_the_notice_once() {
    let mut h = Harness::root(3);
    report(&mut h, task(1, running_task()));
    h.event(Event::Hidden);
    let first = h.event(Event::Task(Box::new(task(1, ready_task()))));
    let again = h.event(Event::Task(Box::new(task(1, ready_task()))));
    assert!(expires(&first), "the notice shows and times out");
    assert!(!expires(&again), "a repeated report shows nothing new");
}

fn open(h: &mut Harness, held: bool, asked: bool) -> Vec<Effect> {
    h.event(Event::Activated {
        held,
        probe: None,
        asked,
    })
}

fn hidden_answer(h: &mut Harness) {
    report(h, answer_ready(1));
}

fn ran_show(effects: &[Effect]) -> usize {
    effects
        .iter()
        .filter(
            |effect| matches!(effect, Effect::RunAction(request) if request.action_id == "show"),
        )
        .count()
}

#[test]
fn opening_during_an_answer_s_notice_opens_the_answer() {
    let mut h = Harness::root(3);
    h.event(Event::Hidden);
    hidden_answer(&mut h);
    let effects = open(&mut h, false, true);
    assert_eq!(ran_show(&effects), 1);
    assert!(h.snapshot().ticker.is_none());
    assert!(h.snapshot().notice.is_none());
}

#[test]
fn opening_after_an_answer_s_notice_shows_the_ticker() {
    let mut h = Harness::root(3);
    h.event(Event::Hidden);
    let shown = h.event(Event::Task(Box::new(answer_ready(1))));
    h.event(Event::TickerExpired {
        id: notice_expiry(&shown),
    });
    let effects = open(&mut h, false, true);
    assert_eq!(ran_show(&effects), 0);
    let ticker = h.snapshot().ticker.expect("the ticker");
    assert_eq!(ticker.action.unwrap().title, "Show answer");
}

#[test]
fn a_command_showing_the_launcher_does_not_open_the_answer() {
    let mut h = Harness::root(3);
    h.event(Event::Hidden);
    hidden_answer(&mut h);
    let effects = open(&mut h, false, false);
    assert_eq!(ran_show(&effects), 0);
    assert_eq!(
        h.ticker(TickerStyle::Success).as_deref(),
        Some("Answer ready: Summarize")
    );
}

#[test]
fn a_newer_notice_takes_the_answer_s_place() {
    let mut h = Harness::root(3);
    h.event(Event::Hidden);
    hidden_answer(&mut h);
    h.event(Event::Failed("Couldn't paste".into()));
    let effects = open(&mut h, false, true);
    assert_eq!(ran_show(&effects), 0);
}

#[test]
fn a_failure_never_opens_anything() {
    let mut h = Harness::root(3);
    report(&mut h, task(1, running_task()));
    h.event(Event::Hidden);
    report(&mut h, task(1, failed_task()));
    let effects = open(&mut h, false, true);
    assert!(!effects
        .iter()
        .any(|effect| matches!(effect, Effect::RunAction(_))));
    assert!(h.failure().is_some());
}

fn held_answer_view() -> Harness {
    let mut h = Harness::view(detail_tree("Better."));
    h.event(Event::Hidden);
    report(
        &mut h,
        crate::tasks::TaskState {
            owner: "fixture".into(),
            title: "Fixture Command".into(),
            ..answer_ready(1)
        },
    );
    h
}

#[test]
fn reopening_held_onto_the_answer_s_own_view_shows_nothing_more() {
    let mut h = held_answer_view();
    let effects = open(&mut h, true, true);
    assert_eq!(ran_show(&effects), 0, "its notice still shows, yet no jump");
    assert!(h.snapshot().ticker.is_none());
}

#[test]
fn after_a_reset_the_answer_is_offered_again() {
    let mut h = held_answer_view();
    h.event(Event::Reset);
    let shown = h.snapshot().notice.is_some();
    assert!(shown);
    let effects = open(&mut h, false, false);
    assert_eq!(ran_show(&effects), 0);
    let ticker = h.snapshot().ticker.expect("the ticker");
    assert_eq!(ticker.action.unwrap().title, "Show answer");
}

#[test]
fn another_view_of_the_same_extension_still_gets_the_ticker() {
    let mut h = Harness::view(detail_tree("A form"));
    h.event(Event::Hidden);
    report(
        &mut h,
        crate::tasks::TaskState {
            owner: "fixture".into(),
            title: "Another Command".into(),
            ..answer_ready(1)
        },
    );
    open(&mut h, true, false);
    assert_eq!(
        h.ticker(TickerStyle::Success).as_deref(),
        Some("Answer ready: Summarize")
    );
}

#[test]
fn leaving_a_running_task_s_view_shows_its_busy_ticker() {
    let mut tree = detail_tree("Bet");
    if let crate::protocol::View::Detail(detail) = &mut tree.view {
        detail.loading = true;
    }
    let mut h = Harness::view(tree);
    report(
        &mut h,
        crate::tasks::TaskState {
            owner: "fixture".into(),
            line: "Running Fixture Command…".into(),
            ..task(1, running_task())
        },
    );
    h.press("Escape");
    h.wake_busy();
    assert_eq!(
        h.ticker(TickerStyle::Busy).as_deref(),
        Some("Running Fixture Command…")
    );
}

#[test]
fn a_success_message_shows_in_the_notice_once_hidden() {
    let mut h = Harness::root(3);
    let effects = h.event(Event::Succeeded("Brightness set to 60%".into()));
    assert!(effects.is_empty());
    assert!(h.snapshot().ticker.is_none());
    let effects = h.event(Event::Hidden);
    notice_expiry(&effects);
    assert_eq!(notice_text(&h).as_deref(), Some("Brightness set to 60%"));
}

fn web_search(name: &str, query: &str) -> ResultItem {
    ResultItem {
        id: format!("web:{name}:{query}"),
        title: format!("Search {name} for “{query}”"),
        web_search: true,
        ..result(0)
    }
}

fn headings(h: &Harness) -> Vec<Option<String>> {
    h.root_screen()
        .sections
        .iter()
        .map(|section| section.heading.clone())
        .collect()
}

#[test]
fn web_searches_sit_under_their_own_heading_below_the_results() {
    let mut h = Harness::root(0);
    h.typed("fire");
    let mut items = results(2);
    items.push(web_search("DuckDuckGo", "fire"));
    h.results("fire", items);

    assert_eq!(headings(&h), [None, Some("Search the web".to_string())]);
    let root = h.root_screen();
    assert_eq!(
        root.sections[1].items[0].title,
        "Search DuckDuckGo for “fire”"
    );
    assert_eq!(root.selected.as_deref(), Some("result-1"));
}

#[test]
fn a_web_search_stands_in_for_no_results() {
    let mut h = Harness::root(0);
    h.typed("zzqx");
    h.results("zzqx", vec![web_search("DuckDuckGo", "zzqx")]);

    let root = h.root_screen();
    assert_eq!(root.empty, None);
    assert_eq!(root.selected.as_deref(), Some("web:DuckDuckGo:zzqx"));
}

#[test]
fn without_a_web_search_nothing_found_still_says_so() {
    let mut h = Harness::root(0);
    h.typed("zzqx");
    h.results("zzqx", Vec::new());
    assert!(h.root_screen().empty.is_some());
}

// A view that searches through its command

fn searching_tree(items: Vec<ListItem>) -> ViewTree {
    ViewTree::new(View::List(ListView {
        filtering: crate::protocol::Filtering::Command,
        items,
        ..Default::default()
    }))
}

fn search_request(query: &str) -> super::effect::ActionRequest {
    super::effect::ActionRequest {
        origin: super::effect::Origin::View,
        extension_id: "fixture".into(),
        item_id: query.into(),
        action_id: crate::protocol::SEARCH_ACTION.into(),
        values: None,
        query: Some(query.into()),
    }
}

fn titled(title: &str) -> ListItem {
    ListItem {
        id: title.to_lowercase(),
        title: title.into(),
        ..Default::default()
    }
}

#[test]
fn each_query_reaches_a_command_that_searches() {
    let mut h = Harness::view(searching_tree(vec![list_item(1)]));

    let effects = h.typed("rust");

    let [Effect::RunAction(request)] = effects.as_slice() else {
        panic!("expected one action, got {effects:?}");
    };
    assert_eq!(request.action_id, crate::protocol::SEARCH_ACTION);
    assert_eq!(request.item_id, "rust");
    assert_eq!(request.query.as_deref(), Some("rust"));
}

#[test]
fn a_list_that_searches_is_not_narrowed_again() {
    let mut h = Harness::view(searching_tree(vec![list_item(1)]));
    h.typed("rust");

    h.event(Event::ActionFinished {
        request: search_request("rust"),
        outcome: ActionOutcome::Replaced(Box::new(searching_tree(vec![titled("Cargo")]))),
    });

    assert_eq!(shown_ids(&h.list()), ["cargo"]);
}

#[test]
fn an_answer_for_an_older_query_is_dropped() {
    let mut h = Harness::view(searching_tree(vec![list_item(1)]));
    h.typed("ru");
    h.typed("rust");

    h.event(Event::ActionFinished {
        request: search_request("rust"),
        outcome: ActionOutcome::Replaced(Box::new(searching_tree(vec![titled("Cargo")]))),
    });
    h.event(Event::ActionFinished {
        request: search_request("ru"),
        outcome: ActionOutcome::Replaced(Box::new(searching_tree(vec![titled("Old")]))),
    });

    assert_eq!(shown_ids(&h.list()), ["cargo"]);
}

#[test]
fn a_searching_list_shows_the_first_fifty_in_the_command_s_order() {
    let mut h = Harness::view(searching_tree(vec![list_item(1)]));
    h.typed("rust");

    h.event(Event::ActionFinished {
        request: search_request("rust"),
        outcome: ActionOutcome::Replaced(Box::new(searching_tree(
            (1..=80).rev().map(list_item).collect(),
        ))),
    });

    let shown = shown_ids(&h.list());
    assert_eq!(shown.len(), 50);
    assert_eq!(shown[0], "item-80");
}

#[test]
fn a_searching_list_keeps_its_query_when_going_back() {
    let mut h = Harness::view(searching_tree(vec![list_item(1)]));
    h.typed("rust");
    h.render(2, ViewTree::new(View::List(ListView::default())));

    h.press("Escape");

    assert_eq!(h.list().query.text, "rust");
}

#[test]
fn a_running_action_says_so_until_it_finishes() {
    let mut h = Harness::view(list_tree(vec![list_item(1)]));
    let effects = h.press("Enter");
    let Some(Effect::RunAction(request)) = effects.first() else {
        panic!("expected an action, got {effects:?}");
    };
    let request = request.clone();

    assert!(effects
        .iter()
        .any(|effect| matches!(effect, Effect::WakeBusyMarks { .. })));
    assert_eq!(h.launcher.busy_text().as_deref(), Some("Running Open…"));

    h.event(Event::ActionFinished {
        request,
        outcome: ActionOutcome::Ticker("Done".into()),
    });

    assert_eq!(h.launcher.busy_text(), None);
}
