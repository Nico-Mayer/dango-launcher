use std::sync::{Arc, Mutex};
use std::time::Duration;

use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use tokio::runtime::Handle;

use super::{Core, Services, ViewRunner, RESULT_LIMIT};
use crate::config::{Config, FileConfig};
use crate::extension::{ExtensionHost, HostResolver};
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
    let config = Config::parse(config_json).unwrap();
    let launcher = hotkeys::launcher(&config);
    let config = Arc::new(FileConfig::new(
        std::env::temp_dir().join("dango-core-test.json"),
        config,
    ));
    let host = ExtensionHost::new(config.clone());
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
        hotkeys: Registry::new(Box::new(grab), launcher),
        hyperkey: Mutex::new(None),
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
