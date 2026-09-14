pub mod config;
pub mod extension;
pub mod extensions;
pub mod hotkeys;
pub mod invocation;
mod latency;
pub mod platform;
pub mod protocol;
pub mod ranking;
pub mod search;
pub mod store;
pub mod templates;
pub mod text;

use std::sync::Arc;
use std::sync::Mutex;

use tauri::{
    menu::{IsMenuItem, Menu, MenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager, WebviewWindow,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use extension::{ActionOutcome, EnabledStore, ExtensionHost, FormValues, HostResolver};
use extensions::applications::{AppIndex, ApplicationsExtension, IconCache};
use extensions::clipboard::{ClipboardExtension, History, PreferencePolicy, Watcher};
use extensions::snippets::{self, SnippetsExtension};
use extensions::system::SystemExtension;
use extensions::window_management::WindowManagementExtension;
use invocation::{InvokeError, Invoker, Outcome, Output};
use latency::LatencyProbe;
use platform::LauncherWindow;
use ranking::{now_millis, DangoRanker, FrecencyTable};
use search::{Candidate, SearchPipeline, StaticCommandSource};
use store::{Opened, Store};
use text::TextExchange;

/// The most results streamed to the frontend for one query.
const RESULT_LIMIT: usize = 50;

/// Event carrying a streamed search snapshot to the frontend.
const EVENT_RESULTS: &str = "dango://results";

/// Event carrying the message from a command that failed. Success needs no
/// event: the launcher simply gets out of the way.
const EVENT_FAILED: &str = "dango://failed";

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ResultItem {
    extension_id: String,
    id: String,
    title: String,
    subtitle: Option<String>,
    icon: Option<String>,
    actions: Vec<protocol::Action>,
    match_positions: Vec<usize>,
}

impl From<Candidate> for ResultItem {
    fn from(c: Candidate) -> Self {
        Self {
            extension_id: c.extension_id,
            id: c.id,
            title: c.title,
            subtitle: c.subtitle,
            icon: c.icon,
            actions: c.actions,
            match_positions: c.match_positions,
        }
    }
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ResultsPayload {
    query: String,
    items: Vec<ResultItem>,
    complete: bool,
}

/// A view tree with the extension whose command produced it. The frontend needs
/// the owner to send an action back, and a command started from its own hotkey
/// never goes through `invoke_command`, so the tree has to carry it.
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct RenderPayload {
    owner: String,
    tree: protocol::ViewTree,
}

#[derive(serde::Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum ActionResponse {
    Done,
    Copy { text: String },
    Replaced { tree: protocol::ViewTree },
    Failed { message: String },
}

/// Hops a closure onto the main thread and waits for it.
///
/// macOS traps if key synthesis runs anywhere else: mapping a character to a
/// keycode goes through the Text Services Manager, which asserts the main
/// queue. The rest of an insertion has to stay off the main thread, because it
/// sleeps and the hide it waits for needs the main run loop to turn.
struct OnMainThread(tauri::AppHandle);

impl text::MainThread for OnMainThread {
    fn run(&self, work: Box<dyn FnOnce() + Send>) {
        // Queuing from the main thread would deadlock: the caller waits for a
        // result the main thread cannot produce until the caller yields.
        #[cfg(target_os = "macos")]
        if objc2_foundation::MainThreadMarker::new().is_some() {
            work();
            return;
        }
        if self.0.run_on_main_thread(work).is_err() {
            eprintln!("[dango] could not reach the main thread for a keystroke");
        }
    }
}

/// Getting the launcher off screen before a keystroke is sent to another
/// application. The window has to be hidden from the main thread, and the
/// insertion runs off it, so this hops back.
struct HideLauncher(tauri::AppHandle);

impl text::Launcher for HideLauncher {
    fn dismiss(&self) {
        hide_after_launch(&self.0);
    }
}

/// Opening a quicklink through the official plugin rather than by shelling out
/// to `open` or `cmd /c start`, which is the project's standing preference and
/// also the only route that behaves the same on both platforms.
struct DefaultBrowser(tauri::AppHandle);

impl snippets::OpenUrl for DefaultBrowser {
    fn open(&self, url: &str) -> Result<(), String> {
        use tauri_plugin_opener::OpenerExt;
        self.0
            .opener()
            .open_url(url, None::<&str>)
            .map_err(|error| error.to_string())
    }
}

#[cfg(target_os = "macos")]
const SHORTCUT_LABEL: &str = "Option+Space";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_LABEL: &str = "Alt+Space";

struct AppState {
    launcher: Mutex<Box<dyn LauncherWindow>>,
}

/// Runs window work on the main thread, inline when already there.
///
/// Tauri does not marshal this for you, and AppKit traps rather than
/// misbehaving: ordering a window from another thread aborts the process. Every
/// show and hide goes through here, because `run_action` is async and so runs
/// on a worker, and a hide that happens to be a no-op will not trap while the
/// one that actually moves a window will.
fn on_main_thread(app: &tauri::AppHandle, work: impl FnOnce(&tauri::AppHandle) + Send + 'static) {
    #[cfg(target_os = "macos")]
    if objc2_foundation::MainThreadMarker::new().is_some() {
        work(app);
        return;
    }
    let handle = app.clone();
    if app.run_on_main_thread(move || work(&handle)).is_err() {
        eprintln!("[dango] could not reach the main thread for a window operation");
    }
}

fn show(app: &tauri::AppHandle, probe_id: Option<u64>) {
    on_main_thread(app, move |app| {
        let Some(window) = app.get_webview_window("main") else {
            return;
        };
        let state = app.state::<AppState>();
        let mut launcher = state.launcher.lock().unwrap();
        launcher.position_on_active_display(&window);
        launcher.show(&window);
        let _ = app.emit_to("main", "dango://activate", probe_id);
    });
}

/// Caller must already be on the main thread; `toggle` and `dismiss` are what
/// guarantee that. Returning a value is why this cannot marshal itself.
fn hide(app: &tauri::AppHandle) -> bool {
    let Some(window) = app.get_webview_window("main") else {
        return false;
    };
    // Hiding drops the webview's focus, which fires blur and a second dismiss
    // from the frontend. Nothing must happen twice for that.
    if !window.is_visible().unwrap_or(false) {
        return false;
    }
    let state = app.state::<AppState>();
    let mut launcher = state.launcher.lock().unwrap();
    launcher.hide(&window);
    launcher.restore_previous_focus();
    abandon_running_command(app);
    let _ = app.emit_to("main", "dango://reset", ());
    true
}

/// A command still working when the launcher goes away has nowhere to put its
/// output, and the user has moved on. Its later output is dropped rather than
/// waiting to surprise them on the next activation.
fn abandon_running_command(app: &tauri::AppHandle) {
    if let Some(invoker) = app.try_state::<Arc<Invoker>>() {
        invoker.abandon();
    }
}

fn toggle(app: &tauri::AppHandle) {
    on_main_thread(app, |app| {
        let visible = app
            .get_webview_window("main")
            .and_then(|w| w.is_visible().ok())
            .unwrap_or(false);
        if visible {
            hide(app);
        } else {
            let probe_id = app.state::<LatencyProbe>().start();
            show(app, probe_id);
        }
    });
}

#[tauri::command]
fn report_paint(app: tauri::AppHandle, id: u64) {
    app.state::<LatencyProbe>().finish(id);
}

#[tauri::command]
fn dismiss(app: tauri::AppHandle) {
    on_main_thread(&app, |app| {
        if hide(app) {
            app.state::<LatencyProbe>().note("dismissed by frontend");
        }
    });
}

/// Runs a query and streams merged snapshots to the frontend as providers
/// answer. Async so it executes on Tauri's tokio runtime, where the pipeline
/// can spawn its per-query task. Returns at once; results arrive on the event.
#[tauri::command]
async fn search(app: tauri::AppHandle, query: String) {
    let Some(pipeline) = app.try_state::<SearchPipeline>() else {
        return;
    };
    let mut rx = pipeline.query(query);
    tauri::async_runtime::spawn(async move {
        while let Some(results) = rx.recv().await {
            let payload = ResultsPayload {
                query: results.query,
                items: results.items.into_iter().map(ResultItem::from).collect(),
                complete: results.complete,
            };
            if app.emit_to("main", EVENT_RESULTS, payload).is_err() {
                break;
            }
        }
    });
}

/// Runs an action on a result, through the extension that contributed it.
/// Success hides the launcher; copy hands the text back for the frontend to
/// place on the clipboard; failure keeps the launcher open with a message.
#[tauri::command]
async fn run_action(
    app: tauri::AppHandle,
    extension_id: String,
    item_id: String,
    action_id: String,
    values: Option<FormValues>,
) -> ActionResponse {
    let Some(host) = app.try_state::<Arc<Mutex<ExtensionHost>>>() else {
        return ActionResponse::Failed {
            message: "no extensions are loaded".into(),
        };
    };
    let outcome = host.lock().unwrap().perform_action(
        &extension_id,
        &item_id,
        &action_id,
        &values.unwrap_or_default(),
    );
    match outcome {
        ActionOutcome::Done => {
            // Any successful action counts as use. Revealing an application is
            // reaching for it just as much as launching it is.
            if let Some(frecency) = app.try_state::<Arc<FrecencyTable>>() {
                frecency.record_launch(&item_id, now_millis());
            }
            hide_after_launch(&app);
            ActionResponse::Done
        }
        ActionOutcome::CopyToClipboard(text) => ActionResponse::Copy { text },
        ActionOutcome::Replaced(tree) => ActionResponse::Replaced { tree: *tree },
        ActionOutcome::Failed(message) => ActionResponse::Failed { message },
    }
}

/// What a template field's text will ask for. Pure: nothing is stored and
/// nothing is rendered, so the form can call it on every keystroke.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct TemplateInspection {
    arguments: Vec<String>,
    error: Option<String>,
}

/// Parses a template and answers with the arguments it would ask for, or why it
/// cannot be read. This is what makes a doubled-brace expression pasted from
/// another templating system visible before it is saved.
#[tauri::command]
fn inspect_template(source: String) -> TemplateInspection {
    match templates::Template::parse(&source) {
        Ok(template) => TemplateInspection {
            arguments: template.prompts(),
            error: None,
        },
        Err(error) => TemplateInspection {
            arguments: Vec::new(),
            error: Some(error.to_string()),
        },
    }
}

#[cfg(test)]
mod inspect_tests {
    use super::inspect_template;

    #[test]
    fn a_template_with_arguments_reports_them_in_order() {
        let inspection = inspect_template("{{ zebra }} and {{ apple }}".into());
        assert_eq!(inspection.arguments, vec!["zebra", "apple"]);
        assert_eq!(inspection.error, None);
    }

    #[test]
    fn a_template_with_no_arguments_reports_none() {
        let inspection = inspect_template("just {{ date }}, no questions".into());
        assert!(inspection.arguments.is_empty());
        assert_eq!(inspection.error, None);
    }

    #[test]
    fn a_template_that_will_not_parse_reports_why() {
        let inspection = inspect_template("unclosed {{ name".into());
        assert!(inspection.arguments.is_empty());
        assert!(inspection.error.is_some());
    }

    #[test]
    fn a_pasted_doubled_brace_expression_is_visible_before_it_is_saved() {
        let inspection = inspect_template("runs-on: ${{ matrix.os }}".into());
        assert_eq!(inspection.arguments, vec!["matrix"]);
    }
}

/// Invokes a command and streams what it produces to the frontend. Returns the
/// extension that owns it, so an action chosen inside a view the command pushes
/// can be sent back to the right place, or the reason it could not start.
#[tauri::command]
async fn invoke_command(app: tauri::AppHandle, command_id: String) -> Result<(), String> {
    run_command(&app, &command_id).map_err(|error| error.to_string())
}

/// Runs a command through the invoker and streams its output the same way for a
/// launcher selection and a hotkey press: a view goes to the webview, success
/// hides the launcher, a failure is reported.
fn run_command(app: &tauri::AppHandle, command_id: &str) -> Result<(), InvokeError> {
    let owner = app
        .try_state::<Arc<Mutex<ExtensionHost>>>()
        .and_then(|host| {
            let host = host.lock().ok()?;
            Some(host.registry().get(command_id)?.extension_id.clone())
        })
        .ok_or(InvokeError::Unavailable)?;
    let invoker = app
        .try_state::<Arc<Invoker>>()
        .ok_or(InvokeError::HostUnavailable)?;
    let mut rx = invoker.invoke(command_id)?;
    if let Some(frecency) = app.try_state::<Arc<FrecencyTable>>() {
        frecency.record_launch(command_id, now_millis());
    }

    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(output) = rx.recv().await {
            match output {
                Output::View(tree) => {
                    let payload = RenderPayload {
                        owner: owner.clone(),
                        tree,
                    };
                    if app.emit_to("main", protocol::EVENT_RENDER, payload).is_err() {
                        break;
                    }
                }
                Output::Finished(Outcome::Success) => {
                    hide_after_launch(&app);
                    break;
                }
                Output::Finished(Outcome::Failure(message)) => {
                    let _ = app.emit_to("main", EVENT_FAILED, message);
                    break;
                }
            }
        }
    });
    Ok(())
}

/// Invokes a command from its hotkey. A no-view command runs without showing the
/// launcher, acting on the window that was focused at the press; a view command
/// shows the launcher first so its pushed view is displayed.
fn dispatch_command(app: &tauri::AppHandle, command_id: &str) {
    let shows_view = app
        .try_state::<Arc<Mutex<ExtensionHost>>>()
        .and_then(|host| {
            let host = host.lock().ok()?;
            Some(host.registry().get(command_id)?.decl.mode == extension::InvocationMode::View)
        })
        .unwrap_or(false);

    if shows_view {
        show(app, None);
        let _ = run_command(app, command_id);
        return;
    }

    // A headless command that reshapes or types into the user's window needs the
    // window that was focused at the press; the launcher never showed to record
    // it, so capture it now.
    #[cfg(target_os = "windows")]
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
        let foreground = GetForegroundWindow();
        if !foreground.is_null() {
            platform::remember_previous_foreground(foreground as isize);
        }
    }
    let _ = run_command(app, command_id);
}

/// Hides the launcher without restoring the previous foreground, because the
/// just-launched application should keep focus rather than the window that was
/// in front before.
fn hide_after_launch(app: &tauri::AppHandle) {
    on_main_thread(app, |app| {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.hide();
        }
        abandon_running_command(app);
        let _ = app.emit_to("main", "dango://reset", ());
    });
}

/// The webview does not allocate its drawing surface until the window is shown
/// once, and paying that cost on the user's first hotkey pushed cold activation
/// to the edge of the budget. Showing far offscreen at startup pays it early
/// without a visible flash.
#[tauri::command]
fn warmup_done(app: tauri::AppHandle) {
    // Called once per frontend mount. More than one line here across a session
    // means the webview reloaded, which would defeat the warm-window design.
    eprintln!("[dango] frontend mounted");
    finish_warmup(&app);
}

fn finish_warmup(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let _ = window.hide();
    // Leaving the window parked offscreen would strand it there if positioning
    // ever fails on the next show.
    let state = app.state::<AppState>();
    state
        .launcher
        .lock()
        .unwrap()
        .position_on_active_display(&window);
}

fn open_store(app: &tauri::App) -> Result<Opened, Box<dyn std::error::Error>> {
    let path = app.path().app_data_dir()?.join("dango.sqlite");
    Ok(Store::open(&path)?)
}

fn warm_up(app: &tauri::AppHandle, window: &WebviewWindow) {
    let _ = window.set_position(tauri::LogicalPosition::new(-10_000.0, -10_000.0));
    let _ = window.show();

    // Backstop: if the frontend never reports back, the window must not be left
    // shown offscreen, because every later toggle would flip an invisible
    // window instead of presenting the launcher.
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let visible = app
            .get_webview_window("main")
            .and_then(|w| w.is_visible().ok())
            .unwrap_or(false);
        if visible {
            eprintln!("[dango] frontend never reported mounting; hiding warmup window");
            finish_warmup(&app);
        }
    });
}

/// The platform's answer to which application did the copying, for spikes
/// that need to exercise the real one outside the application.
pub fn platform_attribution() -> Arc<dyn extensions::clipboard::Attribution> {
    platform::attribution()
}

/// The launcher shortcut from the config file, or the platform default when it
/// is unset or invalid. Option+Space on macOS and Alt+Space on Windows are the
/// same chord: Option is Alt.
fn launcher_shortcut(config: &config::Config) -> Shortcut {
    if let Some(hotkey) = &config.launcher.hotkey {
        match hotkey.parse_with_hyper(config.hyper_modifiers()) {
            Ok(parsed) => return Shortcut::new(Some(parsed.modifiers), parsed.code),
            Err(error) => eprintln!("[dango] invalid launcher hotkey, using the default: {error}"),
        }
    }
    Shortcut::new(Some(Modifiers::ALT), Code::Space)
}

/// The tray line describing the config file's health.
fn config_status_text(error: bool) -> &'static str {
    if error {
        "Config error, see log"
    } else {
        "Config loaded"
    }
}

/// A handle to the tray's config status line, so a reload can update it.
struct ConfigStatusItem(MenuItem<tauri::Wry>);

fn set_config_status(app: &tauri::AppHandle, error: bool) {
    if let Some(item) = app.try_state::<ConfigStatusItem>() {
        let _ = item.0.set_text(config_status_text(error));
    }
}

/// Appends a config problem to a log file beside the config. A release build has
/// `windows_subsystem = "windows"`, so stderr is not visible; the log is.
fn log_config(message: &str) {
    let path = config::config_dir().join("dango.log");
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        use std::io::Write;
        let _ = writeln!(file, "{message}");
    }
}

/// Command candidates with the config file's aliases applied over the manifest's.
fn aliased_candidates(host: &ExtensionHost, file_config: &config::FileConfig) -> Vec<Candidate> {
    let shared = file_config.shared();
    let config = shared.read().unwrap();
    host.command_candidates()
        .into_iter()
        .map(|mut candidate| {
            let prefix = format!("{}.", candidate.extension_id);
            if let Some(command_id) = candidate.id.strip_prefix(&prefix) {
                if let Some(alias) = config.alias(&candidate.extension_id, command_id) {
                    candidate.alias = Some(alias);
                }
            }
            candidate
        })
        .collect()
}

/// Applies a reloaded configuration, on the main thread. Re-registers the
/// launcher hotkey if it changed, brings enabled state and search in line, and
/// updates the tray. A parse failure keeps the running config and is surfaced.
fn apply_config_reload(
    app: &tauri::AppHandle,
    file_config: &Arc<config::FileConfig>,
    launcher_hotkey: &Arc<Mutex<Shortcut>>,
    bindings: &Arc<Mutex<Vec<(Shortcut, String)>>>,
    hyperkey: &HyperkeyHandle,
    result: Result<config::Config, config::ConfigError>,
) {
    let config = match result {
        Ok(config) => config,
        Err(error) => {
            log_config(&format!("config reload failed: {error}"));
            set_config_status(app, true);
            return;
        }
    };

    let new_shortcut = launcher_shortcut(&config);
    file_config.replace(config);

    {
        let mut current = launcher_hotkey.lock().unwrap();
        if *current != new_shortcut {
            let _ = app.global_shortcut().unregister(*current);
            if let Err(error) = app.global_shortcut().register(new_shortcut) {
                log_config(&format!(
                    "could not register the new launcher hotkey: {error}"
                ));
            }
            *current = new_shortcut;
        }
    }

    if let Some(host) = app.try_state::<Arc<Mutex<ExtensionHost>>>() {
        let mut host = host.lock().unwrap();
        host.reload_enabled();
        if let Some(pipeline) = app.try_state::<SearchPipeline>() {
            let candidates = aliased_candidates(&host, file_config);
            pipeline.reload(
                Arc::new(StaticCommandSource(candidates)),
                host.root_providers(),
            );
        }
    }

    let conflicts = {
        let config = file_config.shared();
        let config = config.read().unwrap();
        let mut conflicts = apply_command_hotkeys(app, &config, new_shortcut, bindings);
        if let Some(message) = apply_hyperkey(&config, hyperkey) {
            conflicts.push(message);
        }
        conflicts
    };
    for conflict in &conflicts {
        log_config(conflict);
    }
    set_config_status(app, !conflicts.is_empty());
}

/// (Re)registers the command hotkeys from a config: unregisters the ones the app
/// currently holds, rebuilds the table first-wins, registers the survivors, and
/// returns the conflicts to surface (collisions plus any OS registration
/// refusal).
fn apply_command_hotkeys(
    app: &tauri::AppHandle,
    config: &config::Config,
    launcher: Shortcut,
    bindings: &Arc<Mutex<Vec<(Shortcut, String)>>>,
) -> Vec<String> {
    let previous: Vec<(Shortcut, String)> = bindings.lock().unwrap().drain(..).collect();
    for (chord, _) in previous {
        let _ = app.global_shortcut().unregister(chord);
    }

    let built = crate::hotkeys::command_bindings(config, launcher);
    let mut conflicts = built.conflicts;
    let mut table = bindings.lock().unwrap();
    for binding in built.bindings {
        match app.global_shortcut().register(binding.chord) {
            Ok(()) => table.push((binding.chord, binding.command_id)),
            Err(error) => conflicts.push(format!(
                "{}: could not register its hotkey, {error}",
                binding.command_id
            )),
        }
    }
    conflicts
}

/// The running hyperkey, held for the app's lifetime and swapped on a reload.
type HyperkeyHandle = Arc<Mutex<Option<Box<dyn platform::Hyperkey>>>>;

/// Keeps the hyperkey handle in Tauri state so it lives as long as the app.
struct HyperkeyState(#[allow(dead_code)] HyperkeyHandle);

/// (Re)starts the hyperkey from a config: drops the current one, which stops its
/// hook and releases any modifiers it holds, then starts a fresh one if the
/// config asks for it. Returns a message to surface when a configured hyperkey
/// could not start.
fn apply_hyperkey(config: &config::Config, handle: &HyperkeyHandle) -> Option<String> {
    let mut current = handle.lock().unwrap();
    *current = None;

    let hyperkey = config.hyperkey.as_ref()?;
    let Some(trigger) = platform::HyperkeyTrigger::from_key(&hyperkey.key) else {
        return Some(format!(
            "hyperkey: '{}' is not a remappable key",
            hyperkey.key
        ));
    };
    let mods = config.hyper_modifiers();
    let spec = platform::HyperkeySpec {
        trigger,
        emit: platform::HyperModifiers {
            ctrl: mods.contains(Modifiers::CONTROL),
            alt: mods.contains(Modifiers::ALT),
            shift: mods.contains(Modifiers::SHIFT),
            meta: mods.contains(Modifiers::SUPER),
        },
        tap: hyperkey.tap.clone(),
    };
    match platform::start_hyperkey(spec) {
        Some(running) => {
            *current = Some(running);
            None
        }
        None => Some(
            "hyperkey: could not start; unavailable on this platform or refused by the OS"
                .to_string(),
        ),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config_path = config::config_path();
    let (initial_config, config_error) = match config::Config::load(&config_path) {
        Ok(config) => (config, None),
        Err(error) => {
            eprintln!("[dango] config file error, using defaults: {error}");
            (config::Config::default(), Some(error.to_string()))
        }
    };
    let shortcut = launcher_shortcut(&initial_config);
    let file_config = Arc::new(config::FileConfig::new(config_path, initial_config));
    // The current launcher chord, shared so a reload can re-register it and the
    // handler can compare against whatever is registered now.
    let launcher_hotkey = Arc::new(Mutex::new(shortcut));
    let handler_hotkey = launcher_hotkey.clone();
    // Command hotkeys, as (chord, qualified command id). Shared so the handler
    // dispatches on a press and a reload can rebuild the set.
    let bindings: Arc<Mutex<Vec<(Shortcut, String)>>> = Arc::new(Mutex::new(Vec::new()));
    let handler_bindings = bindings.clone();
    let hyperkey: HyperkeyHandle = Arc::new(Mutex::new(None));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            show(app, None);
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, received, event| {
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }
                    if *received == *handler_hotkey.lock().unwrap() {
                        toggle(app);
                        return;
                    }
                    let command = handler_bindings
                        .lock()
                        .unwrap()
                        .iter()
                        .find(|(chord, _)| chord == received)
                        .map(|(_, id)| id.clone());
                    if let Some(command_id) = command {
                        dispatch_command(app, &command_id);
                    }
                })
                .build(),
        )
        .manage(LatencyProbe::default())
        .invoke_handler(tauri::generate_handler![
            report_paint,
            dismiss,
            warmup_done,
            search,
            run_action,
            invoke_command,
            inspect_template
        ])
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let window: WebviewWindow = app.get_webview_window("main").expect("main window");
            let launcher = platform::launcher_window(&window);
            app.manage(AppState {
                launcher: Mutex::new(launcher),
            });

            // Clicking away from a non-activating panel does not reliably blur
            // the webview, so the frontend's own blur handler is not enough on
            // its own: the launcher would stay on screen without keyboard focus,
            // visible but dead. The window's focus event is the reliable signal.
            {
                let handle = app.handle().clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::Focused(false) = event {
                        hide_after_launch(&handle);
                    }
                });
            }

            // The tray menu is the only surface for startup problems, so they
            // are collected before it is built. None of them stops Dango from
            // starting.
            let mut notices = Vec::new();
            app.manage(file_config.clone());

            let store: Option<Arc<Store>> = match open_store(app) {
                Ok(Opened {
                    store,
                    recovered_from,
                }) => {
                    if let Some(aside) = recovered_from {
                        eprintln!("[dango] database was corrupt, moved to {}", aside.display());
                        notices.push("Database was corrupt, started empty".to_string());
                    }
                    eprintln!(
                        "[dango] store ready at schema version {}",
                        store.schema_version().unwrap_or(0)
                    );
                    let store = Arc::new(store);
                    app.manage(store.clone());
                    Some(store)
                }
                Err(error) => {
                    eprintln!("[dango] database unavailable: {error}");
                    notices.push("Database unavailable, see log".to_string());
                    None
                }
            };

            // Enable/disable now lives in the config file, so the host reads it
            // from there rather than the database.
            let enabled: Arc<dyn EnabledStore> = file_config.clone();
            let mut host = ExtensionHost::new(enabled);

            let indexer = platform::app_indexer();
            let index = AppIndex::new(indexer.clone(), store.clone());
            let cache_dir = app
                .path()
                .app_cache_dir()
                .unwrap_or_else(|_| std::env::temp_dir())
                .join("icons");
            // The webview loads cached icon files through the asset protocol.
            let _ = app.asset_protocol_scope().allow_directory(&cache_dir, true);
            let icons = IconCache::new(cache_dir);
            let extension = Arc::new(ApplicationsExtension::new(index, icons.clone()));
            match host.register(extension.clone()) {
                Ok(report) if report.is_clean() => {}
                Ok(report) => eprintln!("[dango] applications loaded with issues: {report:?}"),
                Err(error) => eprintln!("[dango] applications failed to load: {error}"),
            }
            app.manage(extension);

            // Registering for application activations needs the main thread,
            // which this is; the watcher itself runs on its own.
            let attribution = platform::attribution();
            if let Some(store) = &store {
                let images = app
                    .path()
                    .app_cache_dir()
                    .unwrap_or_else(|_| std::env::temp_dir())
                    .join("clipboard");
                // The webview loads image thumbnails through the asset protocol.
                let _ = app.asset_protocol_scope().allow_directory(&images, true);

                let history = Arc::new(History::new(store.clone(), images));
                let policy = Arc::new(PreferencePolicy(extension::Preferences::new(
                    extensions::clipboard::EXTENSION_ID,
                    extensions::clipboard::preference_declarations(),
                    file_config.clone(),
                )));
                match extensions::clipboard::CrateClipboard::new() {
                    Some(clipboard) => {
                        let watcher = Arc::new(Watcher::new(
                            clipboard.clone(),
                            attribution,
                            history.clone(),
                            policy,
                        ));
                        // The watcher is what keeps a paste out of the history,
                        // so the exchange is built against this one.
                        let dismisser: Arc<dyn text::Launcher> =
                            Arc::new(HideLauncher(app.handle().clone()));
                        let exchange = platform::text_exchange(
                            clipboard,
                            watcher.clone(),
                            dismisser,
                            Arc::new(OnMainThread(app.handle().clone())),
                        )
                        .map(Arc::new);
                        match &exchange {
                            Some(exchange) => {
                                app.manage(exchange.clone());
                            }
                            None => eprintln!("[dango] no key injection, so pasting is off"),
                        }
                        let preferences = extension::Preferences::new(
                            extensions::clipboard::EXTENSION_ID,
                            extensions::clipboard::preference_declarations(),
                            file_config.clone(),
                        );
                        let extension = Arc::new(ClipboardExtension::new(
                            history,
                            watcher,
                            preferences,
                            exchange.map(|exchange| exchange as Arc<dyn text::TextTarget>),
                        ));
                        match host.register(extension) {
                            Ok(report) if report.is_clean() => {}
                            Ok(report) => {
                                eprintln!("[dango] clipboard loaded with issues: {report:?}")
                            }
                            Err(error) => eprintln!("[dango] clipboard failed to load: {error}"),
                        }
                    }
                    None => eprintln!("[dango] no clipboard available, so history is off"),
                }
            } else {
                eprintln!("[dango] no database, so clipboard history is off");
            }

            {
                // Snippets and quicklinks are file-backed now, so they load from
                // the config directory rather than the database and share one
                // extension type, differing in their file and what they do on
                // confirm.
                let exchange: Option<Arc<dyn text::TextTarget>> = app
                    .try_state::<Arc<TextExchange>>()
                    .map(|state| state.inner().clone() as Arc<dyn text::TextTarget>);
                let opener: Arc<dyn snippets::OpenUrl> =
                    Arc::new(DefaultBrowser(app.handle().clone()));
                // Keyword expansion skips the applications the clipboard history
                // is told to skip: one list, read fresh each keystroke.
                let excluded: snippets::ExcludedApps = {
                    use extensions::clipboard::Policy;
                    let policy = PreferencePolicy(extension::Preferences::new(
                        extensions::clipboard::EXTENSION_ID,
                        extensions::clipboard::preference_declarations(),
                        file_config.clone(),
                    ));
                    Arc::new(move || policy.excluded_applications())
                };
                let records_dir = config::config_dir();
                for kind in [snippets::Kind::Snippet, snippets::Kind::Quicklink] {
                    let (records, load_error) = snippets::Records::open(&records_dir, kind);
                    if let Some(error) = load_error {
                        log_config(&format!("{}: {error}", kind.file_name()));
                        notices.push(format!("{} has an error, see log", kind.file_name()));
                    }
                    // Apply hand-edits to the file live, on the main thread for
                    // the tray, keeping the last good records on a parse failure.
                    {
                        let records = records.clone();
                        let handle = app.handle().clone();
                        let file = kind.file_name();
                        config::watch::watch(
                            records.path().to_path_buf(),
                            records.own_write(),
                            move |text| {
                                if let Err(error) = records.reload(&text) {
                                    let handle = handle.clone();
                                    let message = format!("{file}: {error}");
                                    let _ = handle.clone().run_on_main_thread(move || {
                                        log_config(&message);
                                        set_config_status(&handle, true);
                                    });
                                }
                            },
                        );
                    }
                    let extension = Arc::new(SnippetsExtension::new(
                        records,
                        exchange.clone(),
                        Some(opener.clone()),
                        excluded.clone(),
                    ));
                    match host.register(extension) {
                        Ok(report) if report.is_clean() => {}
                        Ok(report) => eprintln!("[dango] {kind:?} loaded with issues: {report:?}"),
                        Err(error) => eprintln!("[dango] {kind:?} failed to load: {error}"),
                    }
                }
            }

            let system = Arc::new(SystemExtension::new(
                platform::system_control(),
                icons.clone(),
            ));
            match host.register(system) {
                Ok(report) if report.is_clean() => {}
                Ok(report) => eprintln!("[dango] system loaded with issues: {report:?}"),
                Err(error) => eprintln!("[dango] system failed to load: {error}"),
            }

            let window_management = Arc::new(WindowManagementExtension::new(
                platform::window_manager(Arc::new(OnMainThread(app.handle().clone()))),
            ));
            match host.register(window_management) {
                Ok(report) if report.is_clean() => {}
                Ok(report) => eprintln!("[dango] window-management loaded with issues: {report:?}"),
                Err(error) => eprintln!("[dango] window-management failed to load: {error}"),
            }

            let frecency = Arc::new(match &store {
                Some(store) => FrecencyTable::load(Box::new(store.clone())),
                None => FrecencyTable::in_memory(),
            });
            let ranker = Arc::new(DangoRanker::new(frecency.clone()));
            let commands = StaticCommandSource(aliased_candidates(&host, &file_config));
            let pipeline = SearchPipeline::new(
                Arc::new(commands),
                host.root_providers(),
                ranker,
                RESULT_LIMIT,
            );
            app.manage(pipeline);
            app.manage(frecency);

            // Shared rather than owned by the invoker, because resolving a
            // command has to see the host as it is now: an extension disabled
            // since startup must no longer be invocable.
            let host = Arc::new(Mutex::new(host));
            app.manage(Arc::new(Invoker::new(Arc::new(HostResolver(host.clone())))));
            app.manage(host);

            let current_shortcut = *launcher_hotkey.lock().unwrap();
            if let Err(error) = app.global_shortcut().register(current_shortcut) {
                eprintln!("[dango] could not register the launcher hotkey: {error}");
                notices.push(format!("{SHORTCUT_LABEL} unavailable, already in use"));
            }

            // Register the command hotkeys from the config, surfacing conflicts.
            {
                let handle = app.handle().clone();
                let config = file_config.shared();
                let config = config.read().unwrap();
                let conflicts =
                    apply_command_hotkeys(&handle, &config, current_shortcut, &bindings);
                if !conflicts.is_empty() {
                    for conflict in &conflicts {
                        log_config(conflict);
                    }
                    notices.push("Hotkey conflict, see log".to_string());
                }
            }

            // Start the hyperkey from the config, kept alive in state so it runs
            // even if config watching later fails to start.
            {
                let config = file_config.shared();
                let config = config.read().unwrap();
                if let Some(message) = apply_hyperkey(&config, &hyperkey) {
                    log_config(&message);
                    notices.push("Hyperkey unavailable, see log".to_string());
                }
            }
            app.manage(HyperkeyState(hyperkey.clone()));

            let notice_items = notices
                .iter()
                .enumerate()
                .map(|(i, text)| {
                    MenuItem::with_id(app, format!("notice-{i}"), text, false, None::<&str>)
                })
                .collect::<Result<Vec<_>, _>>()?;
            // A live status line for the config file, updated as it is reloaded.
            let config_status = MenuItem::with_id(
                app,
                "config-status",
                config_status_text(config_error.is_some()),
                false,
                None::<&str>,
            )?;
            let toggle_item = MenuItem::with_id(app, "toggle", "Toggle Dango", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit Dango", true, None::<&str>)?;
            let mut items: Vec<&dyn IsMenuItem<tauri::Wry>> = notice_items
                .iter()
                .map(|item| item as &dyn IsMenuItem<tauri::Wry>)
                .collect();
            items.push(&config_status);
            items.push(&toggle_item);
            items.push(&quit_item);
            let menu = Menu::with_items(app, &items)?;
            app.manage(ConfigStatusItem(config_status.clone()));
            if config_error.is_some() {
                log_config("config file did not parse at startup, using defaults");
            }

            // Apply edits to the config file live, on the main thread.
            {
                let handle = app.handle().clone();
                let file_config = file_config.clone();
                let launcher_hotkey = launcher_hotkey.clone();
                let bindings = bindings.clone();
                let hyperkey = hyperkey.clone();
                let started = config::watch::watch(
                    config::config_path(),
                    file_config.own_write(),
                    move |text| {
                        let app = handle.clone();
                        let file_config = file_config.clone();
                        let launcher_hotkey = launcher_hotkey.clone();
                        let bindings = bindings.clone();
                        let hyperkey = hyperkey.clone();
                        let _ = app.clone().run_on_main_thread(move || {
                            apply_config_reload(
                                &app,
                                &file_config,
                                &launcher_hotkey,
                                &bindings,
                                &hyperkey,
                                config::Config::parse(&text),
                            );
                        });
                    },
                );
                if !started {
                    log_config("config file watching is off; edits need a restart");
                }
            }

            TrayIconBuilder::new()
                .icon(app.default_window_icon().expect("bundled icon").clone())
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "toggle" => toggle(app),
                    "quit" => {
                        // Hand the combination back before going away, or it
                        // stays claimed until the session ends.
                        let _ = app.global_shortcut().unregister_all();
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            warm_up(&app.handle().clone(), &window);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
