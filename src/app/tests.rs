use std::sync::{Arc, Mutex};
use std::time::Duration;

use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use tokio::runtime::Handle;

use super::{Core, Services, ViewRunner, RESULT_LIMIT};
use crate::config::{Config, FileConfig, LauncherDisplay};
use crate::extension::{Extension, ExtensionHost, HostResolver};
use crate::extensions::applications::{AppIndex, ApplicationsExtension, FakeIndexer};
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
    core_with_ai(config_json, grab, extensions, None)
}

fn core_with_ai(
    config_json: &str,
    grab: FakeGrab,
    extensions: Vec<Arc<dyn Extension>>,
    ai: Option<super::AiParts>,
) -> (Arc<Core>, Arc<RecordingShell>) {
    core_with_parts(
        config_json,
        grab,
        extensions,
        ai,
        Arc::new(RecordingShell::default()),
    )
}

fn core_with_shell(
    config_json: &str,
    extensions: Vec<Arc<dyn Extension>>,
    shell: Arc<RecordingShell>,
) -> (Arc<Core>, Arc<RecordingShell>) {
    core_with_parts(config_json, FakeGrab::default(), extensions, None, shell)
}

fn core_with_parts(
    config_json: &str,
    grab: FakeGrab,
    extensions: Vec<Arc<dyn Extension>>,
    ai: Option<super::AiParts>,
    shell: Arc<RecordingShell>,
) -> (Arc<Core>, Arc<RecordingShell>) {
    core_with_applications(config_json, grab, extensions, ai, shell, None)
}

fn core_with_applications(
    config_json: &str,
    grab: FakeGrab,
    mut extensions: Vec<Arc<dyn Extension>>,
    ai: Option<super::AiParts>,
    shell: Arc<RecordingShell>,
    applications: Option<Arc<ApplicationsExtension>>,
) -> (Arc<Core>, Arc<RecordingShell>) {
    if let Some(applications) = &applications {
        extensions.push(applications.clone());
    }
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
        contexts: std::sync::RwLock::new(contexts),
        js: None,
        applications,
        ai,
        clipboard: None,
        hotkeys: Registry::new(Box::new(grab), launcher),
        hyperkey: Mutex::default(),
        tasks: crate::tasks::Tasks::new(|_| {}),
    };
    let core = Arc::new_cyclic(|weak| {
        let mut services = services;
        services.tasks = super::tasks_for(weak.clone());
        Core::new(
            shell.clone(),
            Handle::current(),
            config,
            TrayStatus::default(),
            services,
        )
    });
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
    start_titled(core, None)
}

fn start_titled(core: &Arc<Core>, title: Option<&str>) -> Arc<InvocationContext> {
    ViewRunner {
        core: Arc::downgrade(core),
        owner: "test".to_string(),
    }
    .start(title)
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
    let (core, shell) = core("");
    core.toggle();
    assert_eq!(shell.calls(), [Call::Show]);
    assert_eq!(shell.last_snapshot().activation.count, 1);
}

#[tokio::test]
async fn an_action_without_an_item_records_no_use() {
    let (core, _) = core("");
    core.record_use("dango.snippets", "", Some("sig"));
    core.record_use("dango.snippets", "sig", None);

    let now = crate::ranking::now_millis();
    assert_eq!(core.services.frecency.score("dango.snippets", "", now), 0.0);
    assert!(core.services.frecency.score("dango.snippets", "sig", now) > 0.0);
}

#[tokio::test]
async fn the_hotkey_again_hides_and_hands_focus_back() {
    let (core, shell) = core("");
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
    let (core, shell) = core(r#"launcher = { reset-after-seconds = 5 }"#);
    core.toggle();
    type_query(&core, "fire");
    core.toggle();
    assert_eq!(query(&shell), "fire");

    tokio::time::sleep(Duration::from_secs(6)).await;
    assert_eq!(query(&shell), "");
}

#[tokio::test(start_paused = true)]
async fn showing_within_the_hold_continues_where_the_user_left_off() {
    let (core, shell) = core(r#"launcher = { reset-after-seconds = 5 }"#);
    core.toggle();
    type_query(&core, "fire");
    core.toggle();
    tokio::time::sleep(Duration::from_secs(2)).await;
    core.toggle();
    tokio::time::sleep(Duration::from_secs(10)).await;
    assert_eq!(query(&shell), "fire");
}

#[tokio::test(start_paused = true)]
async fn a_click_outside_hides_and_holds_the_query() {
    let (core, shell) = core(r#"launcher = { reset-after-seconds = 5 }"#);
    core.toggle();
    type_query(&core, "fire");
    core.dispatch(Intent::ClickedOutside);
    assert_eq!(
        shell.calls().last(),
        Some(&Call::Hide {
            restore_focus: true
        })
    );
    core.toggle();
    assert_eq!(query(&shell), "fire");
}

#[tokio::test]
async fn a_zero_hold_resets_at_once() {
    let (core, shell) = core(r#"launcher = { reset-after-seconds = 0 }"#);
    core.toggle();
    type_query(&core, "fire");
    core.toggle();
    assert_eq!(query(&shell), "");
}

#[tokio::test]
async fn escape_on_an_empty_root_dismisses() {
    let (core, shell) = core("");
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
    let (core, shell) = core("");
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

/// The reload builds the index the fake commands are found in.
fn with_commands(config: &str) -> (Arc<Core>, Arc<RecordingShell>) {
    let (core, shell) = core_with(config, FakeGrab::default(), vec![two_commands()]);
    core.apply_config_reload(Config::parse(config));
    (core, shell)
}

fn list_open(shell: &RecordingShell) -> bool {
    match shell.last_snapshot().screen {
        Screen::Root(root) => !root.bare,
        other => panic!("expected root search, got {other:?}"),
    }
}

#[tokio::test(start_paused = true)]
async fn open_list_opens_the_list_at_start() {
    let (core, shell) = with_commands(r#"launcher = { open-list = true }"#);
    core.dispatch(Intent::Ready);
    core.toggle();
    settle().await;
    assert!(list_open(&shell));
}

#[tokio::test(start_paused = true)]
async fn open_list_opens_the_list_after_a_dismiss() {
    let (core, shell) = with_commands(r#"launcher = { open-list = true }"#);
    core.toggle();
    settle().await;
    escape(&core);
    assert_eq!(
        shell.calls().last(),
        Some(&Call::Hide {
            restore_focus: true
        })
    );
    core.toggle();
    settle().await;
    assert!(list_open(&shell));
}

#[tokio::test(start_paused = true)]
async fn open_list_opens_the_list_when_the_hold_runs_out() {
    let (core, shell) =
        with_commands(r#"launcher = { open-list = true, reset-after-seconds = 5 }"#);
    core.toggle();
    settle().await;
    type_query(&core, "fi");
    core.toggle();
    tokio::time::sleep(Duration::from_secs(6)).await;
    core.toggle();
    settle().await;
    assert_eq!(query(&shell), "");
    assert!(list_open(&shell));
}

#[tokio::test(start_paused = true)]
async fn a_held_show_keeps_the_typed_prompt_despite_open_list() {
    let (core, shell) =
        with_commands(r#"launcher = { open-list = true, reset-after-seconds = 5 }"#);
    core.toggle();
    settle().await;
    type_query(&core, "fi");
    core.toggle();
    core.toggle();
    settle().await;
    assert_eq!(query(&shell), "fi");
}

#[tokio::test(start_paused = true)]
async fn a_reload_changes_what_the_next_reset_opens() {
    let (core, shell) = with_commands("");
    core.toggle();
    settle().await;
    assert!(!list_open(&shell));
    core.apply_config_reload(Config::parse(r#"launcher = { open-list = true }"#));
    escape(&core);
    core.toggle();
    settle().await;
    assert!(list_open(&shell));
}

fn shows_a_view(shell: &RecordingShell) -> bool {
    matches!(shell.last_snapshot().screen, Screen::View(_))
}

#[tokio::test(start_paused = true)]
async fn a_held_launcher_keeps_a_commands_view_for_the_next_activation() {
    let (core, shell) = core(r#"launcher = { reset-after-seconds = 5 }"#);
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
    let (core, shell) = core(r#"launcher = { reset-after-seconds = 5 }"#);
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
    let (core, shell) = core("");
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
    let (core, shell) = core("");
    core.toggle();
    type_query(&core, "fire");
    core.toggle();
    core.hide_for_work();
    core.toggle();
    assert_eq!(query(&shell), "fire");
}

#[tokio::test]
async fn showing_a_visible_launcher_does_not_show_it_again() {
    let (core, shell) = core("");
    core.toggle();
    core.show(None, true);
    assert_eq!(shell.calls(), [Call::Show]);
}

#[tokio::test]
async fn focus_loss_hides_without_taking_focus_back() {
    let (core, shell) = core("");
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

/// Counts how often it is told to stop everything.
#[derive(Default)]
struct StopCounter(std::sync::atomic::AtomicUsize);

impl crate::extensions::ai::Completions for StopCounter {
    fn stream(&self, _request: crate::extensions::ai::Request) -> crate::extensions::ai::Chunks {
        crate::extensions::ai::Chunks::new(std::sync::mpsc::channel().1)
    }

    fn stop_all(&self) {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

struct NoRunner;

impl Runner for NoRunner {
    fn start(&self, _title: Option<&str>) -> Option<Arc<InvocationContext>> {
        None
    }
}

struct NothingSelected;

impl crate::text::TextTarget for NothingSelected {
    fn insert(&self, _text: &str, _caret: Option<usize>) -> Result<(), crate::text::TextError> {
        Ok(())
    }
    fn expand(
        &self,
        _backspaces: usize,
        _text: &str,
        _caret: Option<usize>,
    ) -> Result<(), crate::text::TextError> {
        Ok(())
    }
    fn paste_content(
        &self,
        _content: &crate::extensions::clipboard::Content,
    ) -> Result<(), crate::text::TextError> {
        Ok(())
    }
    fn selection(&self) -> Result<Option<String>, crate::text::TextError> {
        Ok(None)
    }
    fn clipboard_text(&self) -> Option<String> {
        None
    }
}

#[tokio::test]
async fn a_failed_selection_read_shows_in_the_launcher_at_once() {
    let ai = super::AiParts {
        keys: Arc::new(crate::extensions::ai::AuthFile::new(
            std::env::temp_dir().join("dango-no-auth.toml"),
        )),
        client: Arc::new(StopCounter::default()),
        text: Some(Arc::new(NothingSelected)),
        clipboard: None,
        runner: Some(Arc::new(NoRunner)),
        tasks: crate::tasks::Tasks::new(|_| {}),
        visible: Arc::new(|| true),
    };
    let config = r#"
        ai = { providers = { local = { kind = "ollama", model = "qwen3:8b" } } }
    "#;
    let (extension, _) = super::build_ai(&ai, &Config::parse(config).unwrap());
    let (core, shell) = core_with_ai(config, FakeGrab::default(), vec![extension], Some(ai));
    core.toggle();
    core.run_command("dango.ai.improve-writing").unwrap();
    settle().await;

    assert!(!shell
        .calls()
        .iter()
        .any(|call| matches!(call, Call::Hide { .. })));
    let ui = crate::slint_shell::render_headless(shell.last_snapshot());
    assert!(
        i_slint_backend_testing::ElementHandle::find_by_accessible_label(
            &ui,
            "Select some text first"
        )
        .next()
        .is_some(),
        "the failure is on screen"
    );
}

#[tokio::test]
async fn quitting_stops_running_agent_programs() {
    let client = Arc::new(StopCounter::default());
    let ai = super::AiParts {
        keys: Arc::new(crate::extensions::ai::AuthFile::new(
            std::env::temp_dir().join("dango-no-auth.toml"),
        )),
        client: client.clone(),
        text: None,
        clipboard: None,
        runner: Some(Arc::new(NoRunner)),
        tasks: crate::tasks::Tasks::new(|_| {}),
        visible: Arc::new(|| true),
    };
    let (core, _) = core_with_ai("", FakeGrab::default(), Vec::new(), Some(ai));
    core.quit();
    assert_eq!(client.0.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[tokio::test]
async fn quitting_releases_the_hotkeys_and_quits_the_shell() {
    let (core, shell) = core("");
    core.quit();
    assert_eq!(shell.calls(), [Call::Quit]);
}

#[tokio::test]
async fn a_refused_launcher_hotkey_on_reload_keeps_the_old_one_and_reports_it() {
    let refused = HotKey::new(Some(Modifiers::CONTROL), Code::Space);
    let (core, _) = core_with_grab(
        "",
        FakeGrab {
            refused: vec![refused],
            ..Default::default()
        },
    );
    core.apply_config_reload(Config::parse(r#"launcher = { hotkey = "ctrl+space" }"#));
    assert_ne!(core.services.hotkeys.launcher(), refused);
    assert!(core.tray().config_error);
}

#[tokio::test]
async fn a_hotkey_for_a_missing_command_is_reported_in_the_notice() {
    let (core, shell) = core("");
    core.dispatch_command("x.missing");
    assert_eq!(
        notice(&shell).as_deref(),
        Some("That command is no longer available")
    );
    assert!(!core.tray().config_error);
}

#[tokio::test]
async fn an_application_hotkey_without_its_application_is_reported_in_the_notice() {
    let (core, shell) = core("");
    core.dispatch_application("Visual Studio Code");
    assert_eq!(
        notice(&shell).as_deref(),
        Some("Can't open Visual Studio Code")
    );
    assert!(!core.tray().config_error);
}

#[tokio::test]
async fn the_launcher_opens_under_the_pointer_by_default() {
    let (_, shell) = core("");
    assert_eq!(
        *shell.display.lock().unwrap(),
        Some(LauncherDisplay::Pointer)
    );
}

#[tokio::test]
async fn a_reload_moves_the_launcher_to_the_primary_display() {
    let (core, shell) = core("");
    core.apply_config_reload(Config::parse(r#"launcher = { display = "primary" }"#));
    assert_eq!(
        *shell.display.lock().unwrap(),
        Some(LauncherDisplay::Primary)
    );
}

#[tokio::test]
async fn a_clean_reload_keeps_a_reported_problem_on_the_tray() {
    let (core, _) = core("");
    core.report_problem("snippets.toml: broken");
    core.apply_config_reload(Config::parse(""));
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
        load_errors: vec![("gh-prs".into(), "main.js is missing".into())],
    };
    let lines = super::load_problems(&report);
    assert_eq!(lines.len(), 3);
    assert!(lines[0].contains("boom"));
    assert!(lines[1].contains("x.a"));
    assert_eq!(lines[2], "Extension gh-prs didn't load, see dango.log");
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

/// Records whether the launcher had been shown when the command started.
struct ShowsFirst {
    manifest: crate::extension::Manifest,
    shell: Arc<RecordingShell>,
    shown_at_start: Arc<Mutex<Option<bool>>>,
}

impl crate::extension::Extension for ShowsFirst {
    fn manifest(&self) -> &crate::extension::Manifest {
        &self.manifest
    }
    fn command(&self, _command_id: &str) -> Option<Arc<dyn crate::invocation::Command>> {
        struct Probe(Arc<RecordingShell>, Arc<Mutex<Option<bool>>>);
        impl crate::invocation::Command for Probe {
            fn invoke(&self, _ctx: &InvocationContext) {
                *self.1.lock().unwrap() = Some(self.0.calls().contains(&Call::Show));
            }
        }
        Some(Arc::new(Probe(
            self.shell.clone(),
            self.shown_at_start.clone(),
        )))
    }
}

/// A view command from its hotkey may read the selection, which lends the
/// foreground and so needs the launcher shown first. The show is queued on
/// the main thread, so the command has to be queued behind it.
#[tokio::test]
async fn a_view_command_hotkey_starts_the_command_after_the_show() {
    let mut manifest = two_commands().0.clone();
    manifest.commands.truncate(1);
    let shell = Arc::new(RecordingShell::default());
    let shown_at_start = Arc::new(Mutex::new(None));
    let extension = Arc::new(ShowsFirst {
        manifest,
        shell: shell.clone(),
        shown_at_start: shown_at_start.clone(),
    });
    let (core, _) = core_with_shell("", vec![extension], shell.clone());
    shell.queue_main_thread();

    core.dispatch_command("x.keep");
    // Room for a command started too early to run before the queue drains.
    std::thread::sleep(Duration::from_millis(50));
    shell.run_main_thread();
    std::thread::sleep(Duration::from_millis(50));

    assert_eq!(*shown_at_start.lock().unwrap(), Some(true));
}

fn two_commands() -> Arc<TwoCommands> {
    use crate::extension::{CommandDecl, InvocationMode, Manifest};
    let command = |id: &str| CommandDecl {
        id: id.into(),
        title: format!("Fire {id}"),
        mode: InvocationMode::View,
        icon: None,
        keywords: vec![],
        alias: None,
        repeats: false,
    };
    Arc::new(TwoCommands(Manifest {
        manifest_version: 1,
        id: "x".into(),
        name: "X".into(),
        icon: None,
        commands: vec![command("keep"), command("drop")],
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
    let (core, shell) = core("");
    core.services
        .host
        .lock()
        .unwrap()
        .register(two_commands())
        .unwrap();
    core.apply_config_reload(Config::parse(""));

    core.toggle();
    type_query(&core, "fire");
    settle().await;
    assert_eq!(root_ids(&shell), ["x.drop", "x.keep"]);

    core.apply_config_reload(Config::parse(
        r#"extensions = { x = { commands = { drop = { enabled = false } } } }"#,
    ));

    if selected(&shell).as_deref() != Some("x.drop") {
        press(&core, "ArrowDown");
    }
    assert_eq!(selected(&shell).as_deref(), Some("x.drop"));
    press(&core, "Enter");
    assert_eq!(
        shell
            .last_snapshot()
            .ticker
            .map(|ticker| ticker.text)
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
    let (core, shell) = core_with("", FakeGrab::default(), vec![extension]);
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
    let (core, _) = core_with("", FakeGrab::default(), vec![two_commands()]);
    let item = core.result_item(x_candidate(crate::search::Source::RootItem, false));
    assert_eq!(item.subtitle.as_deref(), Some("X"));
}

#[tokio::test]
async fn a_command_shows_its_extension_name() {
    let (core, _) = core_with("", FakeGrab::default(), vec![two_commands()]);
    let item = core.result_item(x_candidate(crate::search::Source::Command, false));
    assert_eq!(item.subtitle.as_deref(), Some("X"));
}

#[tokio::test]
async fn a_pinned_item_shows_its_status_line() {
    let (core, _) = core_with("", FakeGrab::default(), vec![two_commands()]);
    let item = core.result_item(x_candidate(crate::search::Source::RootItem, true));
    assert_eq!(item.subtitle.as_deref(), Some("Needs attention"));
}

fn copied(core: &Arc<Core>) {
    core.apply(crate::ui::Input::Event(crate::ui::Event::ActionFinished {
        request: crate::ui::ActionRequest {
            origin: crate::ui::Origin::Root,
            extension_id: "test".into(),
            item_id: "item".into(),
            action_id: "copy".into(),
            values: None,
            query: None,
        },
        outcome: crate::extension::ActionOutcome::CopyToClipboard("x".into()),
    }));
}

fn notice(shell: &RecordingShell) -> Option<String> {
    let published = shell.published.lock().unwrap();
    published
        .last()?
        .notice
        .as_ref()
        .map(|notice| notice.text.clone())
}

type Hide = fn(&Arc<Core>);

#[tokio::test]
async fn every_hide_path_tells_the_launcher_it_is_hidden() {
    let hides: [(&str, Hide); 5] = [
        ("hotkey", |core| core.toggle()),
        ("escape", escape),
        ("launch", |core| core.hide_after_launch()),
        ("focus loss", |core| core.hide_on_focus_loss()),
        ("for work", |core| core.hide_for_work()),
    ];
    for (name, hide) in hides {
        let (core, shell) = core("");
        core.toggle();
        hide(&core);
        copied(&core);
        assert_eq!(
            notice(&shell).as_deref(),
            Some("Copied to clipboard"),
            "{name}"
        );
    }
}

#[tokio::test(start_paused = true)]
async fn a_success_notice_goes_after_two_seconds() {
    let (core, shell) = core("");
    copied(&core);
    tokio::time::sleep(Duration::from_millis(1900)).await;
    assert!(notice(&shell).is_some());
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(notice(&shell), None);
}

#[tokio::test(start_paused = true)]
async fn a_failure_while_hidden_shows_in_the_notice_for_four_seconds() {
    let (core, shell) = core("");
    core.apply(crate::ui::Input::Event(crate::ui::Event::Failed(
        "Nope".into(),
    )));
    tokio::time::sleep(Duration::from_millis(3900)).await;
    assert_eq!(notice(&shell).as_deref(), Some("Nope"));
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(notice(&shell), None);
    core.toggle();
    assert!(shell.last_snapshot().ticker.is_none());
}

fn with_code(launch_fails: bool) -> (Arc<Core>, Arc<RecordingShell>) {
    let indexer = Arc::new(FakeIndexer::with_apps(&[("code", "Code", None)]));
    if launch_fails {
        indexer.make_launch_fail_not_found();
    }
    let index = AppIndex::new(indexer, None);
    index.rebuild();
    let applications = Arc::new(ApplicationsExtension::new(
        index,
        crate::icon_cache::IconCache::in_temp(),
    ));
    core_with_applications(
        "",
        FakeGrab::default(),
        Vec::new(),
        None,
        Arc::new(RecordingShell::default()),
        Some(applications),
    )
}

async fn wait_for_notice(shell: &RecordingShell) -> Option<String> {
    for _ in 0..200 {
        if let Some(text) = notice(shell) {
            return Some(text);
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    None
}

#[tokio::test]
async fn an_application_hotkey_for_an_app_not_installed_is_reported_in_the_notice() {
    let (core, shell) = with_code(false);
    core.dispatch_application("Slack");
    assert_eq!(
        notice(&shell).as_deref(),
        Some("Can't open Slack because it isn't installed")
    );
    assert!(!core.tray().config_error);
}

#[tokio::test]
async fn an_application_hotkey_whose_launch_fails_is_reported_in_the_notice() {
    let (core, shell) = with_code(true);
    core.dispatch_application("Code");
    assert_eq!(
        wait_for_notice(&shell).await.as_deref(),
        Some("Couldn't open Code")
    );
    assert!(!core.tray().config_error);
}

fn a_loading_view() -> ViewTree {
    ViewTree::new(View::List(ListView {
        loading: true,
        ..ListView::default()
    }))
}

fn busy(shell: &RecordingShell) -> bool {
    matches!(shell.last_snapshot().screen, Screen::View(view) if view.busy)
}

#[tokio::test(start_paused = true)]
async fn a_view_loading_past_the_delay_shows_busy_marks() {
    let (core, shell) = core("");
    core.toggle();
    let work = start_work(&core);
    work.push_view(a_loading_view());
    settle().await;
    assert!(!busy(&shell), "not before the delay");
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(busy(&shell));
}

#[tokio::test(start_paused = true)]
async fn a_view_done_before_the_delay_never_shows_busy_marks() {
    let (core, shell) = core("");
    core.toggle();
    let work = start_work(&core);
    work.push_view(a_loading_view());
    settle().await;
    work.push_view(a_view());
    for _ in 0..10 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(!busy(&shell));
    }
}

#[tokio::test(start_paused = true)]
async fn a_task_report_reaches_the_launcher() {
    let (core, shell) = core("");
    core.toggle();
    let task = core
        .services
        .tasks
        .start("dango.updates", "update", "Dango 0.5.0");
    task.report(|state| state.line = "Downloading Dango 0.5.0 · 42%".into());
    tokio::time::sleep(Duration::from_millis(200)).await;
    let ticker = shell
        .last_snapshot()
        .ticker
        .expect("the task's busy ticker");
    assert_eq!(ticker.text, "Downloading Dango 0.5.0 · 42%");
}

#[tokio::test]
async fn an_answer_is_never_learned() {
    let (core, _) = core("");
    core.record_use(
        crate::extensions::calculator::EXTENSION_ID,
        "4",
        Some("2+2"),
    );
    let now = crate::ranking::now_millis();
    assert_eq!(
        core.services
            .frecency
            .score(crate::extensions::calculator::EXTENSION_ID, "4", now),
        0.0
    );
}

#[tokio::test]
async fn a_web_search_teaches_nothing() {
    use crate::extensions::snippets::QUICKLINKS_ID;
    let (core, _) = core("");
    let id = "web:duckduckgo:rust";
    core.record_use(QUICKLINKS_ID, id, Some("rust"));

    let now = crate::ranking::now_millis();
    assert_eq!(core.services.frecency.score(QUICKLINKS_ID, id, now), 0.0);
    let learned = core.services.learned.lookup("rust", now);
    assert_eq!(learned.exact(QUICKLINKS_ID, id), None);
}

#[tokio::test]
async fn a_command_shows_its_registered_hotkey() {
    let (core, _) = core_with("", FakeGrab::default(), vec![two_commands()]);
    let config = Config::parse(
        r#"[extensions."x".commands.first]
hotkey = "ctrl+alt+v""#,
    )
    .unwrap();
    core.services.hotkeys.apply(&config);
    let candidate = crate::search::Candidate {
        id: "x.first".into(),
        ..x_candidate(crate::search::Source::Command, false)
    };
    let item = core.result_item(candidate);
    let chord = item.hotkey.expect("the registered hotkey");
    assert!(chord.mods.contains(Modifiers::CONTROL | Modifiers::ALT));
    assert_eq!(chord.key, Code::KeyV);

    let refused = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyV);
    let (core, _) = core_with(
        "",
        FakeGrab {
            refused: vec![refused],
            ..FakeGrab::default()
        },
        vec![two_commands()],
    );
    core.services.hotkeys.apply(&config);
    let candidate = crate::search::Candidate {
        id: "x.first".into(),
        ..x_candidate(crate::search::Source::Command, false)
    };
    assert_eq!(core.result_item(candidate).hotkey, None);
}

fn answer_ready() -> crate::tasks::TaskState {
    crate::tasks::TaskState {
        id: 1,
        owner: "x".into(),
        title: "Summarize".into(),
        line: String::new(),
        status: crate::tasks::TaskStatus::Done {
            sentence: "Answer ready: Summarize".into(),
        },
        progress: None,
        steps: Vec::new(),
        subtitle: String::new(),
        detail: String::new(),
        cancel: None,
        retry: None,
        show: Some("show".into()),
        item_id: "summarize".into(),
    }
}

fn answer_ticker(shell: &RecordingShell) -> bool {
    shell
        .last_snapshot()
        .ticker
        .is_some_and(|ticker| ticker.text == "Answer ready: Summarize")
}

/// Only an open the user asked for may run the answer's "Show answer".
#[tokio::test]
async fn the_launcher_hotkey_opens_an_answer_but_a_command_hotkey_does_not() {
    let (core, shell) = core_with("", FakeGrab::default(), vec![two_commands()]);
    core.apply(crate::ui::Input::Event(crate::ui::Event::Task(Box::new(
        answer_ready(),
    ))));
    core.toggle();
    settle().await;
    assert!(!answer_ticker(&shell), "the answer opened instead");

    let (core, shell) = core_with("", FakeGrab::default(), vec![two_commands()]);
    core.apply(crate::ui::Input::Event(crate::ui::Event::Task(Box::new(
        answer_ready(),
    ))));
    core.dispatch_command("x.keep");
    settle().await;
    assert!(answer_ticker(&shell), "the ticker offers it instead");
}

#[tokio::test]
async fn work_started_for_a_command_is_named_after_it() {
    let (core, shell) = core("");
    core.toggle();
    start_titled(&core, Some("Summarize")).push_view(a_view());
    settle().await;
    assert_eq!(shell.last_snapshot().footer.context.title, "Summarize");
}

/// A no-view command that succeeds, with `message` when it has one.
struct Reports(crate::extension::Manifest, Option<&'static str>);

impl crate::extension::Extension for Reports {
    fn manifest(&self) -> &crate::extension::Manifest {
        &self.0
    }
    fn command(&self, _command_id: &str) -> Option<Arc<dyn crate::invocation::Command>> {
        struct Succeed(Option<&'static str>);
        impl crate::invocation::Command for Succeed {
            fn invoke(&self, ctx: &InvocationContext) {
                match self.0 {
                    Some(message) => ctx.succeed_with(message),
                    None => ctx.succeed(),
                }
            }
        }
        Some(Arc::new(Succeed(self.1)))
    }
}

fn reporting(message: Option<&'static str>) -> Arc<Reports> {
    let mut manifest = two_commands().0.clone();
    manifest.commands.truncate(1);
    manifest.commands[0].mode = crate::extension::InvocationMode::NoView;
    Arc::new(Reports(manifest, message))
}

#[tokio::test]
async fn a_success_message_from_a_hotkey_shows_in_the_notice() {
    let shell = Arc::new(RecordingShell::default());
    let (core, _) = core_with_shell(
        "",
        vec![reporting(Some("Brightness set to 60%"))],
        shell.clone(),
    );

    core.dispatch_command("x.keep");

    assert_eq!(
        wait_for_notice(&shell).await.as_deref(),
        Some("Brightness set to 60%")
    );
    assert!(!shell.calls().contains(&Call::Show));
}

#[tokio::test]
async fn a_success_message_from_the_launcher_hides_it_and_shows_in_the_notice() {
    let shell = Arc::new(RecordingShell::default());
    let (core, _) = core_with_shell(
        "",
        vec![reporting(Some("Brightness set to 60%"))],
        shell.clone(),
    );
    core.toggle();

    core.run_command("x.keep").unwrap();

    assert_eq!(
        wait_for_notice(&shell).await.as_deref(),
        Some("Brightness set to 60%")
    );
    assert!(shell
        .calls()
        .iter()
        .any(|call| matches!(call, Call::Hide { .. })));
}

#[tokio::test]
async fn a_plain_success_shows_no_notice() {
    let shell = Arc::new(RecordingShell::default());
    let (core, _) = core_with_shell("", vec![reporting(None)], shell.clone());
    core.toggle();

    core.run_command("x.keep").unwrap();

    for _ in 0..200 {
        if shell
            .calls()
            .iter()
            .any(|call| matches!(call, Call::Hide { .. }))
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert!(shell
        .calls()
        .iter()
        .any(|call| matches!(call, Call::Hide { .. })));
    assert_eq!(notice(&shell), None);
}

struct NoMonitors;

impl crate::platform::BrightnessMonitors for NoMonitors {
    fn external(&self) -> Vec<Box<dyn crate::platform::Monitor>> {
        Vec::new()
    }
    fn under_pointer(&self) -> Vec<Box<dyn crate::platform::Monitor>> {
        Vec::new()
    }
}

fn display_core(config: &str) -> (Arc<Core>, Arc<RecordingShell>) {
    let display = Arc::new(crate::extensions::display::DisplayExtension::new(
        Arc::new(NoMonitors),
        Arc::default(),
    ));
    let (core, shell) = core_with("", FakeGrab::default(), vec![display]);
    core.apply_config_reload(Config::parse(config));
    core.toggle();
    (core, shell)
}

#[tokio::test]
async fn brightness_commands_are_found_by_name_and_keyword() {
    let (core, shell) = display_core("");
    let both = [
        "dango.display.decrease-brightness",
        "dango.display.increase-brightness",
    ];
    for query in ["bright", "monitor", "display"] {
        type_query(&core, query);
        settle().await;
        assert_eq!(root_ids(&shell), both, "{query}");
    }
    type_query(&core, "dim");
    settle().await;
    assert_eq!(root_ids(&shell), ["dango.display.decrease-brightness"]);
}

#[tokio::test]
async fn a_disabled_display_extension_offers_no_brightness_command() {
    let (core, shell) = display_core("display = { enabled = false }");
    type_query(&core, "bright");
    settle().await;
    assert!(root_ids(&shell).is_empty());
}

/// Waits to be released, then reports through `report`, like a brightness step
/// that a quicker press superseded.
struct ReportsLate {
    manifest: crate::extension::Manifest,
    release: Arc<Mutex<bool>>,
    outcome: crate::invocation::Outcome,
}

impl crate::extension::Extension for ReportsLate {
    fn manifest(&self) -> &crate::extension::Manifest {
        &self.manifest
    }
    fn command(&self, command_id: &str) -> Option<Arc<dyn crate::invocation::Command>> {
        struct Step(Arc<Mutex<bool>>, crate::invocation::Outcome);
        impl crate::invocation::Command for Step {
            fn invoke(&self, ctx: &InvocationContext) {
                while !*self.0.lock().unwrap() {
                    std::thread::sleep(Duration::from_millis(2));
                }
                ctx.report(self.1.clone());
            }
        }
        struct Quick;
        impl crate::invocation::Command for Quick {
            fn invoke(&self, ctx: &InvocationContext) {
                ctx.succeed();
            }
        }
        if command_id == "drop" {
            return Some(Arc::new(Quick));
        }
        Some(Arc::new(Step(self.release.clone(), self.outcome.clone())))
    }
}

fn reports_late(outcome: crate::invocation::Outcome) -> (Arc<ReportsLate>, Arc<Mutex<bool>>) {
    let mut manifest = two_commands().0.clone();
    for command in &mut manifest.commands {
        command.mode = crate::extension::InvocationMode::NoView;
    }
    let release = Arc::new(Mutex::new(false));
    (
        Arc::new(ReportsLate {
            manifest,
            release: release.clone(),
            outcome,
        }),
        release,
    )
}

#[tokio::test]
async fn a_superseded_success_message_still_shows_in_the_notice() {
    let (extension, release) = reports_late(crate::invocation::Outcome::Success(Some(
        "Brightness set to 70%".into(),
    )));
    let shell = Arc::new(RecordingShell::default());
    let (core, _) = core_with_shell("", vec![extension], shell.clone());

    core.dispatch_command("x.keep");
    core.dispatch_command("x.drop");
    *release.lock().unwrap() = true;

    assert_eq!(
        wait_for_notice(&shell).await.as_deref(),
        Some("Brightness set to 70%")
    );
}

#[tokio::test]
async fn a_superseded_failure_still_shows_in_the_notice() {
    let (extension, release) =
        reports_late(crate::invocation::Outcome::Failure("No monitor".into()));
    let shell = Arc::new(RecordingShell::default());
    let (core, _) = core_with_shell("", vec![extension], shell.clone());

    core.dispatch_command("x.keep");
    core.dispatch_command("x.drop");
    *release.lock().unwrap() = true;

    assert_eq!(wait_for_notice(&shell).await.as_deref(), Some("No monitor"));
}

#[tokio::test]
async fn a_late_success_leaves_the_open_launcher_alone() {
    let (extension, release) = reports_late(crate::invocation::Outcome::Success(Some(
        "Brightness set to 70%".into(),
    )));
    let shell = Arc::new(RecordingShell::default());
    let (core, _) = core_with_shell("", vec![extension], shell.clone());
    core.toggle();

    core.run_command("x.keep").unwrap();
    core.services.invoker.abandon();
    *release.lock().unwrap() = true;
    tokio::time::sleep(Duration::from_millis(100)).await;

    assert!(!shell
        .calls()
        .iter()
        .any(|call| matches!(call, Call::Hide { .. })));
}

/// Two no-view commands counting their runs; `keep` repeats while held when
/// `repeats` is set, `drop` never does.
struct Counts(
    crate::extension::Manifest,
    Arc<std::sync::atomic::AtomicUsize>,
);

impl crate::extension::Extension for Counts {
    fn manifest(&self) -> &crate::extension::Manifest {
        &self.0
    }
    fn command(&self, _command_id: &str) -> Option<Arc<dyn crate::invocation::Command>> {
        struct Count(Arc<std::sync::atomic::AtomicUsize>);
        impl crate::invocation::Command for Count {
            fn invoke(&self, ctx: &InvocationContext) {
                self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                ctx.succeed();
            }
        }
        Some(Arc::new(Count(self.1.clone())))
    }
}

const HELD_KEYS: &str =
    r#"extensions = { x = { commands = { keep = { hotkey = "f2" }, drop = { hotkey = "f3" } } } }"#;

fn counting(repeats: bool) -> (Arc<Core>, Arc<std::sync::atomic::AtomicUsize>) {
    let mut manifest = two_commands().0.clone();
    for command in &mut manifest.commands {
        command.mode = crate::extension::InvocationMode::NoView;
    }
    manifest.commands[0].repeats = repeats;
    let runs = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let (core, _) = core_with(
        "",
        FakeGrab::default(),
        vec![Arc::new(Counts(manifest, runs.clone()))],
    );
    core.apply_config_reload(Config::parse(HELD_KEYS));
    (core, runs)
}

fn key(core: &Arc<Core>, command_id: &str, state: global_hotkey::HotKeyState) {
    let id = core
        .services
        .hotkeys
        .chord_for(&crate::hotkeys::Target::Command(command_id.into()))
        .expect("bound")
        .id();
    core.on_hotkey(global_hotkey::GlobalHotKeyEvent { id, state });
}

/// Lets paused time run, then gives the command threads real time to count.
async fn hold_for(millis: u64) {
    tokio::time::sleep(Duration::from_millis(millis)).await;
    std::thread::sleep(Duration::from_millis(30));
}

fn runs(count: &std::sync::atomic::AtomicUsize) -> usize {
    count.load(std::sync::atomic::Ordering::SeqCst)
}

#[tokio::test(start_paused = true)]
async fn a_tap_on_a_repeating_command_runs_it_once() {
    let (core, count) = counting(true);
    key(&core, "x.keep", global_hotkey::HotKeyState::Pressed);
    hold_for(100).await;
    key(&core, "x.keep", global_hotkey::HotKeyState::Released);
    hold_for(1000).await;
    assert_eq!(runs(&count), 1);
}

#[tokio::test(start_paused = true)]
async fn holding_a_repeating_command_runs_it_until_release() {
    let (core, count) = counting(true);
    key(&core, "x.keep", global_hotkey::HotKeyState::Pressed);
    hold_for(1000).await;
    let held = runs(&count);
    assert!(held >= 4, "ran {held} times in a second");
    key(&core, "x.keep", global_hotkey::HotKeyState::Released);
    hold_for(1000).await;
    assert_eq!(runs(&count), held, "stopped at the release");
}

#[tokio::test(start_paused = true)]
async fn holding_any_other_command_runs_it_once() {
    let (core, count) = counting(false);
    key(&core, "x.keep", global_hotkey::HotKeyState::Pressed);
    hold_for(1000).await;
    assert_eq!(runs(&count), 1);
}

#[tokio::test(start_paused = true)]
async fn another_hotkey_ends_a_repeat() {
    let (core, count) = counting(true);
    key(&core, "x.keep", global_hotkey::HotKeyState::Pressed);
    hold_for(600).await;
    key(&core, "x.drop", global_hotkey::HotKeyState::Pressed);
    hold_for(0).await;
    let after = runs(&count);
    hold_for(1000).await;
    assert_eq!(runs(&count), after);
}

#[tokio::test(flavor = "multi_thread")]
async fn leaving_a_js_view_stops_its_timer_and_its_program() {
    let dir = crate::js::load::tests::temp();
    let ticks = dir.join("ticks.txt");
    let finished = dir.join("finished.txt");
    let program = if cfg!(windows) {
        let line = format!(
            "ping -n 3 127.0.0.1 >nul & echo done > \"{}\"",
            finished.display()
        );
        format!("dango.exec('cmd', ['/c', {line:?}])")
    } else {
        let line = format!("sleep 2; echo done > '{}'", finished.display());
        format!("dango.exec('sh', ['-c', {line:?}])")
    };
    let main = format!(
        r#"export function run() {{
            dango.push({{ kind: "list", items: [{{ id: "a", title: "A" }}] }});
            let n = 0;
            setInterval(() => dango.fs.writeText({ticks:?}, String(n++)), 20);
            {program};
        }}"#,
        ticks = ticks.to_string_lossy()
    );
    let folder = crate::js::load::tests::write_extension(&dir, "ext", &main);
    let core_slot: Arc<std::sync::OnceLock<std::sync::Weak<Core>>> = Arc::default();
    let showing_core = core_slot.clone();
    let host = Arc::new(crate::js::JsHost {
        showing: Arc::new(move |owner| {
            showing_core
                .get()
                .and_then(std::sync::Weak::upgrade)
                .is_some_and(|core| core.view_owner().as_deref() == Some(owner))
        }),
        ..Arc::into_inner(crate::js::load::tests::test_host(Handle::current())).unwrap()
    });
    let extension = crate::js::load::load(&folder, &host).unwrap().unwrap();
    let (core, _) = core_with("", FakeGrab::default(), vec![Arc::new(extension)]);
    core_slot.set(Arc::downgrade(&core)).unwrap();
    core.toggle();

    core.run_command("ext.run").unwrap();
    for _ in 0..200 {
        if core.view_owner().is_some() && ticks.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(core.view_owner().as_deref(), Some("ext"));
    escape(&core);
    tokio::time::sleep(Duration::from_millis(300)).await;
    let after_escape = std::fs::read_to_string(&ticks).unwrap();
    tokio::time::sleep(Duration::from_millis(3000)).await;

    assert_eq!(std::fs::read_to_string(&ticks).unwrap(), after_escape);
    assert!(!finished.exists(), "the program ran to the end");
}
