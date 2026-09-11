pub mod extension;
pub mod extensions;
mod latency;
mod platform;
pub mod protocol;
pub mod ranking;
pub mod search;
pub mod store;

use std::sync::Arc;
use std::sync::Mutex;

use tauri::{
    menu::{IsMenuItem, Menu, MenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager, WebviewWindow,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use extension::{EnabledStore, ExtensionHost};
use extensions::applications::{ActionOutcome, AppIndex, ApplicationsExtension, IconCache};
use latency::LatencyProbe;
use platform::LauncherWindow;
use ranking::{now_millis, DangoRanker, FrecencyTable};
use search::{Candidate, SearchPipeline, StaticCommandSource};
use store::{Opened, Store};

/// The most results streamed to the frontend for one query.
const RESULT_LIMIT: usize = 50;

/// Event carrying a streamed search snapshot to the frontend.
const EVENT_RESULTS: &str = "dango://results";

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ResultItem {
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
    Launched,
    Revealed,
    Copy { text: String },
    Failed { message: String },
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
    let _ = app.emit_to("main", "dango://reset", ());
    true
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

/// Runs an item's action. Launch and reveal hide the launcher; copy hands the
/// text back for the frontend to place on the clipboard; failure keeps the
/// launcher open with a message.
#[tauri::command]
fn run_action(app: tauri::AppHandle, item_id: String, action_id: String) -> ActionResponse {
    let Some(extension) = app.try_state::<Arc<ApplicationsExtension>>() else {
        return ActionResponse::Failed {
            message: "no application handler is available".into(),
        };
    };
    match extension.perform(&item_id, &action_id) {
        ActionOutcome::Launched => {
            if let Some(frecency) = app.try_state::<Arc<FrecencyTable>>() {
                frecency.record_launch(&item_id, now_millis());
            }
            hide_after_launch(&app);
            ActionResponse::Launched
        }
        ActionOutcome::Revealed => {
            hide_after_launch(&app);
            ActionResponse::Revealed
        }
        ActionOutcome::CopyToClipboard(text) => ActionResponse::Copy { text },
        ActionOutcome::Failed(message) => ActionResponse::Failed { message },
    }
}

/// Hides the launcher without restoring the previous foreground, because the
/// just-launched application should keep focus rather than the window that was
/// in front before.
fn hide_after_launch(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let shortcut = Shortcut::new(Some(Modifiers::ALT), Code::Space);

    tauri::Builder::default()
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
            run_action
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
            let icons = IconCache::new(cache_dir, indexer);
            let extension = Arc::new(ApplicationsExtension::new(index, icons));
            match host.register(extension.clone()) {
                Ok(report) if report.is_clean() => {}
                Ok(report) => eprintln!("[dango] applications loaded with issues: {report:?}"),
                Err(error) => eprintln!("[dango] applications failed to load: {error}"),
            }
            app.manage(extension);

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
            app.manage(Mutex::new(host));

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
