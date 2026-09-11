mod latency;
mod platform;

use std::sync::Mutex;

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager, WebviewWindow,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use latency::LatencyProbe;
use platform::LauncherWindow;

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

fn hide(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let state = app.state::<AppState>();
    let mut launcher = state.launcher.lock().unwrap();
    launcher.hide(&window);
    launcher.restore_previous_focus();
    let _ = app.emit_to("main", "dango://reset", ());
}

fn toggle(app: &tauri::AppHandle, probe_id: Option<u64>) {
    let visible = app
        .get_webview_window("main")
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false);
    if visible {
        hide(app);
    } else {
        show(app, probe_id);
    }
}

#[tauri::command]
fn report_paint(app: tauri::AppHandle, id: u64) {
    app.state::<LatencyProbe>().finish(id);
}

#[tauri::command]
fn dismiss(app: tauri::AppHandle) {
    hide(&app);
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
            warmup_done(app);
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
                    let id = app.state::<LatencyProbe>().start();
                    toggle(app, id);
                })
                .build(),
        )
        .manage(LatencyProbe::default())
        .invoke_handler(tauri::generate_handler![report_paint, dismiss, warmup_done])
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let window: WebviewWindow = app.get_webview_window("main").expect("main window");
            let launcher = platform::launcher_window(&window);
            app.manage(AppState {
                launcher: Mutex::new(launcher),
            });

            // Registration is attempted before the tray is built so the menu can
            // report the failure. Another application holding the combination
            // must not stop Dango from starting.
            let shortcut_registered = match app.global_shortcut().register(shortcut) {
                Ok(()) => true,
                Err(error) => {
                    eprintln!("[dango] could not register {SHORTCUT_LABEL}: {error}");
                    false
                }
            };

            let toggle_item = MenuItem::with_id(app, "toggle", "Toggle Dango", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit Dango", true, None::<&str>)?;
            let menu = if shortcut_registered {
                Menu::with_items(app, &[&toggle_item, &quit_item])?
            } else {
                let notice = MenuItem::with_id(
                    app,
                    "shortcut-unavailable",
                    format!("{SHORTCUT_LABEL} unavailable, already in use"),
                    false,
                    None::<&str>,
                )?;
                Menu::with_items(app, &[&notice, &toggle_item, &quit_item])?
            };

            TrayIconBuilder::new()
                .icon(app.default_window_icon().expect("bundled icon").clone())
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "toggle" => toggle(app, None),
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
