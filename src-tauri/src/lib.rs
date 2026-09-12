pub mod extension;
pub mod extensions;
pub mod invocation;
mod latency;
mod platform;
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

#[derive(serde::Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum ActionResponse {
    Done,
    Copy { text: String },
    Replaced { tree: protocol::ViewTree },
    Failed { message: String },
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

/// Fallback enabled store used only when the database could not be opened, so
/// extensions still load with their default state.
struct AlwaysEnabled;

impl EnabledStore for AlwaysEnabled {
    fn is_enabled(&self, _extension_id: &str) -> bool {
        true
    }
    fn set_enabled(&self, _extension_id: &str, _enabled: bool) {}
}

#[cfg(target_os = "macos")]
const SHORTCUT_LABEL: &str = "Option+Space";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_LABEL: &str = "Alt+Space";

struct AppState {
    launcher: Mutex<Box<dyn LauncherWindow>>,
}

fn show(app: &tauri::AppHandle, probe_id: Option<u64>) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let state = app.state::<AppState>();
    let mut launcher = state.launcher.lock().unwrap();
    launcher.position_on_active_display(&window);
    launcher.show(&window);
    let _ = app.emit_to("main", "dango://activate", probe_id);
}

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
}

#[tauri::command]
fn report_paint(app: tauri::AppHandle, id: u64) {
    app.state::<LatencyProbe>().finish(id);
}

#[tauri::command]
fn dismiss(app: tauri::AppHandle) {
    if hide(&app) {
        app.state::<LatencyProbe>().note("dismissed by frontend");
    }
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
fn run_action(
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
async fn invoke_command(app: tauri::AppHandle, command_id: String) -> Result<String, String> {
    let Some(invoker) = app.try_state::<Arc<Invoker>>() else {
        return Err("no extensions are loaded".into());
    };
    let owner = app
        .try_state::<Arc<Mutex<ExtensionHost>>>()
        .and_then(|host| {
            let host = host.lock().ok()?;
            Some(host.registry().get(&command_id)?.extension_id.clone())
        })
        .ok_or_else(|| InvokeError::Unavailable.to_string())?;
    let mut rx = match invoker.invoke(&command_id) {
        Ok(rx) => rx,
        Err(error @ (InvokeError::Unavailable | InvokeError::HostUnavailable)) => {
            return Err(error.to_string())
        }
    };
    if let Some(frecency) = app.try_state::<Arc<FrecencyTable>>() {
        frecency.record_launch(&command_id, now_millis());
    }

    tauri::async_runtime::spawn(async move {
        while let Some(output) = rx.recv().await {
            match output {
                Output::View(tree) => {
                    if app.emit_to("main", protocol::EVENT_RENDER, tree).is_err() {
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
    Ok(owner)
}

/// Hides the launcher without restoring the previous foreground, because the
/// just-launched application should keep focus rather than the window that was
/// in front before.
fn hide_after_launch(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
    abandon_running_command(app);
    let _ = app.emit_to("main", "dango://reset", ());
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let shortcut = Shortcut::new(Some(Modifiers::ALT), Code::Space);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            show(app, None);
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, received, event| {
                    if received != &shortcut || event.state() != ShortcutState::Pressed {
                        return;
                    }
                    toggle(app);
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

            // The tray menu is the only surface for startup problems, so they
            // are collected before it is built. None of them stops Dango from
            // starting.
            let mut notices = Vec::new();

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

            // Without a store, extensions default to enabled and their state
            // simply does not persist.
            let enabled: Arc<dyn EnabledStore> = match &store {
                Some(store) => store.clone(),
                None => Arc::new(AlwaysEnabled),
            };
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
                    store.clone(),
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
                        if let Some(exchange) = platform::text_exchange(clipboard, watcher.clone())
                        {
                            app.manage(Arc::new(exchange));
                        } else {
                            eprintln!("[dango] no key injection, so pasting is off");
                        }
                        let extension = Arc::new(ClipboardExtension::new(history, watcher));
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

            if let Some(store) = &store {
                // Both kinds share one store and one extension type; they
                // differ in which table they own and what they do on confirm.
                let exchange: Option<Arc<dyn text::TextTarget>> = app
                    .try_state::<Arc<TextExchange>>()
                    .map(|state| state.inner().clone() as Arc<dyn text::TextTarget>);
                let opener: Arc<dyn snippets::OpenUrl> =
                    Arc::new(DefaultBrowser(app.handle().clone()));
                for kind in [snippets::Kind::Snippet, snippets::Kind::Quicklink] {
                    let records = Arc::new(snippets::Records::new(store.clone(), kind));
                    let extension = Arc::new(SnippetsExtension::new(
                        records,
                        exchange.clone(),
                        Some(opener.clone()),
                    ));
                    match host.register(extension) {
                        Ok(report) if report.is_clean() => {}
                        Ok(report) => eprintln!("[dango] {kind:?} loaded with issues: {report:?}"),
                        Err(error) => eprintln!("[dango] {kind:?} failed to load: {error}"),
                    }
                }
            } else {
                eprintln!("[dango] no database, so snippets and quicklinks are off");
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

            let frecency = Arc::new(match &store {
                Some(store) => FrecencyTable::load(Box::new(store.clone())),
                None => FrecencyTable::in_memory(),
            });
            let ranker = Arc::new(DangoRanker::new(frecency.clone()));
            let commands = StaticCommandSource(host.command_candidates());
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

            if let Err(error) = app.global_shortcut().register(shortcut) {
                eprintln!("[dango] could not register {SHORTCUT_LABEL}: {error}");
                notices.push(format!("{SHORTCUT_LABEL} unavailable, already in use"));
            }

            let notice_items = notices
                .iter()
                .enumerate()
                .map(|(i, text)| {
                    MenuItem::with_id(app, format!("notice-{i}"), text, false, None::<&str>)
                })
                .collect::<Result<Vec<_>, _>>()?;
            let toggle_item = MenuItem::with_id(app, "toggle", "Toggle Dango", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit Dango", true, None::<&str>)?;
            let mut items: Vec<&dyn IsMenuItem<tauri::Wry>> = notice_items
                .iter()
                .map(|item| item as &dyn IsMenuItem<tauri::Wry>)
                .collect();
            items.push(&toggle_item);
            items.push(&quit_item);
            let menu = Menu::with_items(app, &items)?;

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
