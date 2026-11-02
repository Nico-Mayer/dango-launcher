use std::sync::{Arc, Mutex};
use std::time::Duration;

use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use tokio::runtime::Handle;

use super::{Core, Services, ViewRunner, RESULT_LIMIT};
use crate::config::{Config, FileConfig};
use crate::extension::{Extension, ExtensionHost, HostResolver};
use crate::hotkeys::{self, testing::FakeGrab, Registry};
use crate::invocation::{InvocationContext, Invoker, Runner};
use crate::protocol::{ListView, View, ViewTree};
use crate::ranking::{DangoRanker, FrecencyTable, InputHistory};
use crate::search::{SearchPipeline, StaticCommandSource};
use crate::shell::testing::{Call, RecordingShell};
use crate::shell::TrayStatus;
use crate::ui::{snapshot::Screen, Chord, Intent};

fn core(config_json: &str) -> (Arc<Core>, Arc<RecordingShell>) {
    core_with_grab(config_json, FakeGrab::default())
}

fn core_with_grab(config_json: &str, grab: FakeGrab) -> (Arc<Core>, Arc<RecordingShell>) {
    core_with(config_json, grab, Vec::new())
}

fn core_with(
    config_json: &str,
    grab: FakeGrab,
    extensions: Vec<Arc<dyn Extension>>,
) -> (Arc<Core>, Arc<RecordingShell>) {
    let config = Config::parse(config_json).unwrap();
    let (launcher, _) = hotkeys::launcher(&config);
    let config = Arc::new(FileConfig::new(
        std::env::temp_dir().join("dango-core-test.json"),
        config,
    ));
    let mut host = ExtensionHost::new(config.clone());
    for extension in extensions {
        host.register(extension).unwrap();
    }
    let frecency = Arc::new(FrecencyTable::in_memory());
    let learned = Arc::new(InputHistory::in_memory());
    let pipeline = SearchPipeline::new(
        Arc::new(StaticCommandSource(Vec::new())),
        host.root_providers(),
        Arc::new(DangoRanker::new(frecency.clone(), learned.clone())),
        RESULT_LIMIT,
    );
    let contexts = super::extension_contexts(&host);
    let host = Arc::new(Mutex::new(host));
    let services = Services {
        invoker: Arc::new(Invoker::new(Arc::new(HostResolver(host.clone())))),
        host,
        pipeline,
        frecency,
        learned,
        contexts,
        applications: None,
        ai: None,
        clipboard: None,
        hotkeys: Registry::new(Box::new(grab), launcher),
        hyperkey: Mutex::default(),
    };
    let shell = Arc::new(RecordingShell::default());
    let core = Arc::new(Core::new(
        shell.clone(),
        Handle::current(),
        config,
        TrayStatus::default(),
        services,
    ));
    (core, shell)
}

fn query(shell: &RecordingShell) -> String {
    match shell.last_snapshot().screen {
        Screen::Root(root) => root.query.text,
        other => panic!("expected root search, got {other:?}"),
    }
}

fn type_query(core: &Arc<Core>, text: &str) {
    core.dispatch(Intent::QueryChanged { text: text.into() });
}

fn escape(core: &Arc<Core>) {
    core.dispatch(Intent::Key {
        chord: Chord::plain("Escape"),
    });
}

fn start_work(core: &Arc<Core>) -> Arc<InvocationContext> {
    ViewRunner {
        core: Arc::downgrade(core),
        owner: "test",
    }
    .start()
    .unwrap()
}

fn a_view() -> ViewTree {
    ViewTree::new(View::List(ListView::default()))
}

async fn settle() {
    tokio::time::sleep(Duration::from_millis(10)).await;
}

#[tokio::test]
async fn the_hotkey_shows_the_launcher_and_activates_it() {
    let (core, shell) = core("{}");
    core.toggle();
    assert_eq!(shell.calls(), [Call::Show]);
    assert_eq!(shell.last_snapshot().activation.count, 1);
}

#[tokio::test]
async fn an_action_without_an_item_records_no_use() {
    let (core, _) = core("{}");
    core.record_use("dango.snippets", "", Some("sig"));
    core.record_use("dango.snippets", "sig", None);

    let now = crate::ranking::now_millis();
    assert_eq!(core.services.frecency.score("dango.snippets", "", now), 0.0);
    assert!(core.services.frecency.score("dango.snippets", "sig", now) > 0.0);
}

#[tokio::test]
async fn the_hotkey_again_hides_and_hands_focus_back() {
    let (core, shell) = core("{}");
    core.toggle();
    core.toggle();
    assert_eq!(
        shell.calls(),
        [
            Call::Show,
            Call::Hide {
                restore_focus: true
            }
        ]
    );
}

#[tokio::test(start_paused = true)]
async fn a_held_launcher_keeps_the_query_until_the_hold_runs_out() {
    let (core, shell) = core(r#"{ "launcher": { "reset-after": 5 } }"#);
    core.toggle();
    type_query(&core, "fire");
    core.toggle();
    assert_eq!(query(&shell), "fire");

    tokio::time::sleep(Duration::from_secs(6)).await;
    assert_eq!(query(&shell), "");
}

#[tokio::test(start_paused = true)]
async fn showing_within_the_hold_continues_where_the_user_left_off() {
    let (core, shell) = core(r#"{ "launcher": { "reset-after": 5 } }"#);
    core.toggle();
    type_query(&core, "fire");
    core.toggle();
    tokio::time::sleep(Duration::from_secs(2)).await;
    core.toggle();
    tokio::time::sleep(Duration::from_secs(10)).await;
    assert_eq!(query(&shell), "fire");
}

#[tokio::test]
async fn a_zero_hold_resets_at_once() {
    let (core, shell) = core(r#"{ "launcher": { "reset-after": 0 } }"#);
    core.toggle();
    type_query(&core, "fire");
    core.toggle();
    assert_eq!(query(&shell), "");
}

#[tokio::test]
async fn escape_on_an_empty_root_dismisses() {
    let (core, shell) = core("{}");
    core.toggle();
    core.dispatch(Intent::Key {
        chord: Chord::plain("Escape"),
    });
    assert_eq!(
        shell.calls(),
        [
            Call::Show,
            Call::Hide {
                restore_focus: true
            }
        ]
    );
}

#[tokio::test(start_paused = true)]
async fn a_dismiss_resets_at_once_and_drops_a_running_command() {
    let (core, shell) = core("{}");
    core.toggle();
    type_query(&core, "fire");
    let work = start_work(&core);
    escape(&core);
    escape(&core);
    work.push_view(a_view());
    settle().await;
    core.toggle();
    assert_eq!(query(&shell), "");
}

fn shows_a_view(shell: &RecordingShell) -> bool {
    matches!(shell.last_snapshot().screen, Screen::View(_))
}

#[tokio::test(start_paused = true)]
async fn a_held_launcher_keeps_a_commands_view_for_the_next_activation() {
    let (core, shell) = core(r#"{ "launcher": { "reset-after": 5 } }"#);
    core.toggle();
    let work = start_work(&core);
    core.toggle();
    work.push_view(a_view());
    settle().await;
    assert_eq!(
        shell.calls(),
        [
            Call::Show,
            Call::Hide {
                restore_focus: true
            }
        ]
    );
    core.toggle();
    assert!(shows_a_view(&shell));
}

#[tokio::test(start_paused = true)]
async fn a_command_still_working_when_the_hold_runs_out_is_dropped() {
    let (core, shell) = core(r#"{ "launcher": { "reset-after": 5 } }"#);
    core.toggle();
    let work = start_work(&core);
    core.toggle();
    tokio::time::sleep(Duration::from_secs(6)).await;
    work.push_view(a_view());
    settle().await;
    core.toggle();
    assert_eq!(query(&shell), "");
}

#[tokio::test(start_paused = true)]
async fn a_hide_for_work_comes_back_with_the_commands_view() {
    let (core, shell) = core("{}");
    core.toggle();
    let work = start_work(&core);
    core.hide_for_work();
    work.push_view(a_view());
    settle().await;
    assert_eq!(
        shell.calls(),
        [
            Call::Show,
            Call::Hide {
                restore_focus: false
            },
            Call::Show
        ]
    );
    assert!(shows_a_view(&shell));
}

#[tokio::test(start_paused = true)]
async fn a_hide_for_work_while_hidden_keeps_the_held_query() {
    let (core, shell) = core("{}");
    core.toggle();
    type_query(&core, "fire");
    core.toggle();
    core.hide_for_work();
    core.toggle();
    assert_eq!(query(&shell), "fire");
}

#[tokio::test]
async fn showing_a_visible_launcher_does_not_show_it_again() {
    let (core, shell) = core("{}");
    core.toggle();
    core.show(None);
    assert_eq!(shell.calls(), [Call::Show]);
}

#[tokio::test]
async fn focus_loss_hides_without_taking_focus_back() {
    let (core, shell) = core("{}");
    core.toggle();
    core.hide_on_focus_loss();
    core.hide_on_focus_loss();
    assert_eq!(
        shell.calls(),
        [
            Call::Show,
            Call::Hide {
                restore_focus: false
            }
        ]
    );
}

#[tokio::test]
async fn quitting_releases_the_hotkeys_and_quits_the_shell() {
    let (core, shell) = core("{}");
    core.quit();
    assert_eq!(shell.calls(), [Call::Quit]);
}

#[tokio::test]
async fn a_refused_launcher_hotkey_on_reload_keeps_the_old_one_and_reports_it() {
    let refused = HotKey::new(Some(Modifiers::CONTROL), Code::Space);
    let (core, _) = core_with_grab(
        "{}",
        FakeGrab {
            refused: vec![refused],
            ..Default::default()
        },
    );
    core.apply_config_reload(Config::parse(
        r#"{ "launcher": { "hotkey": "ctrl+space" } }"#,
    ));
    assert_ne!(core.services.hotkeys.launcher(), refused);
    assert!(core.tray().config_error);
}

#[tokio::test]
async fn a_hotkey_for_a_missing_command_is_reported() {
    let (core, _) = core("{}");
    core.dispatch_command("x.missing");
    assert!(core.tray().config_error);
}

#[tokio::test]
async fn a_clean_reload_keeps_a_reported_problem_on_the_tray() {
    let (core, _) = core("{}");
    core.report_problem("snippets.json: broken");
    core.apply_config_reload(Config::parse("{}"));
    assert!(core.tray().config_error);
}

#[test]
fn every_load_problem_gets_a_line() {
    let report = crate::extension::LoadReport {
        activation_errors: vec![("x".into(), "boom".into())],
        collisions: vec![crate::extension::Collision {
            qualified_id: "x.a".into(),
            existing_extension: "x".into(),
            rejected_extension: "y".into(),
        }],
    };
    let lines = super::load_problems(&report);
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("boom"));
    assert!(lines[1].contains("x.a"));
}

mod inspect_tests {
    use super::super::inspect_template;

    #[test]
    fn a_template_with_arguments_reports_them_in_order() {
        let inspection = inspect_template("{{ zebra }} and {{ apple }}");
        assert_eq!(inspection.arguments, vec!["zebra", "apple"]);
        assert_eq!(inspection.error, None);
    }

    #[test]
    fn a_template_with_no_arguments_reports_none() {
        let inspection = inspect_template("just {{ date }}, no questions");
        assert!(inspection.arguments.is_empty());
        assert_eq!(inspection.error, None);
    }

    #[test]
    fn a_template_that_will_not_parse_reports_why() {
        let inspection = inspect_template("unclosed {{ name");
        assert!(inspection.arguments.is_empty());
        assert!(inspection.error.is_some());
    }

    #[test]
    fn a_pasted_doubled_brace_expression_is_visible_before_it_is_saved() {
        let inspection = inspect_template("runs-on: ${{ matrix.os }}");
        assert_eq!(inspection.arguments, vec!["matrix"]);
    }
}

struct TwoCommands(crate::extension::Manifest);

impl crate::extension::Extension for TwoCommands {
    fn manifest(&self) -> &crate::extension::Manifest {
        &self.0
    }
    fn command(&self, _command_id: &str) -> Option<Arc<dyn crate::invocation::Command>> {
        struct Noop;
        impl crate::invocation::Command for Noop {
            fn invoke(&self, _ctx: &InvocationContext) {}
        }
        Some(Arc::new(Noop))
    }
}

fn two_commands() -> Arc<TwoCommands> {
    use crate::extension::{CommandDecl, InvocationMode, Manifest};
    let command = |id: &str| CommandDecl {
        id: id.into(),
        title: format!("Fire {id}"),
        mode: InvocationMode::View,
        subtitle: None,
        icon: None,
        keywords: vec![],
        alias: None,
    };
    Arc::new(TwoCommands(Manifest {
        manifest_version: 1,
        id: "x".into(),
        name: "X".into(),
        icon: None,
        commands: vec![command("keep"), command("drop")],
        preferences: vec![],
        root_items: false,
        services: false,
    }))
}

fn root_ids(shell: &RecordingShell) -> Vec<String> {
    match shell.last_snapshot().screen {
        Screen::Root(root) => {
            let mut ids: Vec<String> = root
                .sections
                .into_iter()
                .flat_map(|section| section.items)
                .map(|item| item.id)
                .collect();
            ids.sort();
            ids
        }
        other => panic!("expected root search, got {other:?}"),
    }
}

fn selected(shell: &RecordingShell) -> Option<String> {
    match shell.last_snapshot().screen {
        Screen::Root(root) => root.selected,
        other => panic!("expected root search, got {other:?}"),
    }
}

fn press(core: &Arc<Core>, key: &str) {
    core.dispatch(Intent::Key {
        chord: Chord::plain(key),
    });
}

#[tokio::test]
async fn a_command_disabled_by_a_reload_leaves_search_and_a_stale_result_fails() {
    let (core, shell) = core("{}");
    core.services
        .host
        .lock()
        .unwrap()
        .register(two_commands())
        .unwrap();
    core.apply_config_reload(Config::parse("{}"));

    core.toggle();
    type_query(&core, "fire");
    settle().await;
    assert_eq!(root_ids(&shell), ["x.drop", "x.keep"]);

    core.apply_config_reload(Config::parse(
        r#"{ "extensions": { "x": { "commands": { "drop": { "enabled": false } } } } }"#,
    ));

    if selected(&shell).as_deref() != Some("x.drop") {
        press(&core, "ArrowDown");
    }
    assert_eq!(selected(&shell).as_deref(), Some("x.drop"));
    press(&core, "Enter");
    assert_eq!(
        shell
            .last_snapshot()
            .toast
            .map(|toast| toast.text)
            .as_deref(),
        Some("That command is no longer available")
    );
    assert!(!shell
        .calls()
        .iter()
        .any(|call| matches!(call, Call::Hide { .. })));

    type_query(&core, "fir");
    settle().await;
    assert_eq!(root_ids(&shell), ["x.keep"]);
}

#[tokio::test]
async fn launching_a_game_hides_the_launcher() {
    use crate::extensions::games::fake::{FakeOpener, FakeSource};
    use crate::extensions::games::{self, GameList, GamesExtension};

    let list = GameList::new(FakeSource::with(&[(1091500, "Cyberpunk 2077")]));
    let opener = Arc::new(FakeOpener::default());
    let extension = Arc::new(GamesExtension::new(list, opener.clone()));
    let (core, shell) = core_with("{}", FakeGrab::default(), vec![extension]);
    core.toggle();
    let outcome = core.run_action(&crate::ui::ActionRequest {
        origin: crate::ui::Origin::View,
        extension_id: games::EXTENSION_ID.into(),
        item_id: "1091500".into(),
        action_id: games::ACTION_LAUNCH.into(),
        values: None,
        query: None,
    });
    settle().await;
    assert_eq!(outcome, crate::extension::ActionOutcome::Done);
    assert_eq!(
        *opener.opened.lock().unwrap(),
        ["steam://rungameid/1091500"]
    );
    assert_eq!(
        shell.calls(),
        [
            Call::Show,
            Call::Hide {
                restore_focus: false
            }
        ]
    );
}

fn x_candidate(source: crate::search::Source, pinned: bool) -> crate::search::Candidate {
    crate::search::Candidate {
        extension_id: "x".into(),
        id: "x.item".into(),
        title: "Item".into(),
        status: Some("Needs attention".into()),
        icon: None,
        keywords: vec![],
        alias: None,
        source,
        actions: vec![],
        match_positions: vec![],
        suggested: false,
        pinned,
    }
}

#[tokio::test]
async fn a_root_item_shows_its_extension_name() {
    let (core, _) = core_with("{}", FakeGrab::default(), vec![two_commands()]);
    let item = core.result_item(x_candidate(crate::search::Source::RootItem, false));
    assert_eq!(item.subtitle.as_deref(), Some("X"));
}

#[tokio::test]
async fn a_command_shows_its_extension_name() {
    let (core, _) = core_with("{}", FakeGrab::default(), vec![two_commands()]);
    let item = core.result_item(x_candidate(crate::search::Source::Command, false));
    assert_eq!(item.subtitle.as_deref(), Some("X"));
}

#[tokio::test]
async fn a_pinned_item_shows_its_status_line() {
    let (core, _) = core_with("{}", FakeGrab::default(), vec![two_commands()]);
    let item = core.result_item(x_candidate(crate::search::Source::RootItem, true));
    assert_eq!(item.subtitle.as_deref(), Some("Needs attention"));
}
