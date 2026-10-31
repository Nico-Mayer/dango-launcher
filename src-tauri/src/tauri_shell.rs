//! The webview shell: Tauri draws the launcher and runs the tray.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager, WebviewWindow};

use crate::app::{self, Core, Startup};
use crate::extensions::updates::PluginSource;
use crate::platform::{self, LauncherWindow, Permission};
use crate::shell::{self, Shell, TrayCommand, TrayStatus};
use crate::ui;

/// Event carrying the launcher's full UI state to the renderer.
const EVENT_STATE: &str = "dango://state";

const TRAY_ID: &str = "main";

struct TauriShell {
    app: tauri::AppHandle,
    launcher: Mutex<Box<dyn LauncherWindow>>,
    /// The window is parked offscreen until the renderer mounts.
    warming: AtomicBool,
}

impl TauriShell {
    fn window(&self) -> Option<WebviewWindow> {
        self.app.get_webview_window("main")
    }
}

fn raw(window: &WebviewWindow) -> Option<RawWindowHandle> {
    Some(window.window_handle().ok()?.as_raw())
}

impl Shell for TauriShell {
    fn publish(&self, snapshot: &ui::Snapshot) {
        let _ = self.app.emit_to("main", EVENT_STATE, snapshot);
    }

    fn show(&self) {
        let Some(window) = self.window() else {
            return;
        };
        let Some(handle) = raw(&window) else {
            return;
        };
        self.warming.store(false, Ordering::SeqCst);
        let mut launcher = self.launcher.lock().unwrap();
        launcher.will_show();
        launcher.position_on_active_display(handle);
        let _ = window.show();
        launcher.did_show(handle);
    }

    fn hide(&self, restore_focus: bool) {
        let Some(window) = self.window() else {
            return;
        };
        let _ = window.hide();
        if restore_focus {
            self.launcher.lock().unwrap().restore_previous_focus();
        }
    }

    fn on_main_thread(&self, work: Box<dyn FnOnce() + Send>) {
        if self.app.run_on_main_thread(work).is_err() {
            eprintln!("[dango] could not reach the main thread");
        }
    }

    fn set_tray(&self, tray: &TrayStatus) {
        let app = self.app.clone();
        let items = tray.items();
        self.on_main_thread(Box::new(move || {
            let Some(icon) = app.tray_by_id(TRAY_ID) else {
                return;
            };
            let Ok(menu) = Menu::new(&app) else {
                return;
            };
            for (index, item) in items.into_iter().enumerate() {
                let id = match item.command {
                    Some(command) => menu_id(command).to_string(),
                    None => format!("status-{index}"),
                };
                let enabled = item.command.is_some();
                if let Ok(entry) = MenuItem::with_id(&app, id, item.text, enabled, None::<&str>) {
                    let _ = menu.append(&entry);
                }
            }
            let _ = icon.set_menu(Some(menu));
        }));
    }

    fn quit(&self) {
        self.app.exit(0);
    }

    /// Leaving the window parked offscreen would strand it there if positioning
    /// ever fails on the next show.
    fn finish_warmup(&self) {
        if !self.warming.swap(false, Ordering::SeqCst) {
            return;
        }
        let Some(window) = self.window() else {
            return;
        };
        let _ = window.hide();
        if let Some(handle) = raw(&window) {
            self.launcher
                .lock()
                .unwrap()
                .position_on_active_display(handle);
        }
    }
}

fn menu_id(command: TrayCommand) -> &'static str {
    match command {
        TrayCommand::Toggle => "toggle",
        TrayCommand::Quit => "quit",
        TrayCommand::Grant(Permission::Accessibility) => "grant-accessibility",
        TrayCommand::Grant(Permission::InputMonitoring) => "grant-input-monitoring",
    }
}

fn tray_command(id: &str) -> Option<TrayCommand> {
    [
        TrayCommand::Toggle,
        TrayCommand::Quit,
        TrayCommand::Grant(Permission::Accessibility),
        TrayCommand::Grant(Permission::InputMonitoring),
    ]
    .into_iter()
    .find(|command| menu_id(*command) == id)
}

fn core(app: &tauri::AppHandle) -> Option<Arc<Core>> {
    app.try_state::<Arc<Core>>()
        .map(|core| core.inner().clone())
}

/// Everything the renderer does arrives here. Sync on purpose: Tauri runs sync
/// commands on the main thread in the order they were sent, and a search
/// cancels the one before it, so two keystrokes must not swap places.
#[tauri::command]
fn dispatch(app: tauri::AppHandle, intent: ui::Intent) {
    if let Some(core) = core(&app) {
        core.dispatch(intent);
    }
}

/// The webview does not allocate its drawing surface until the window is shown
/// once, and paying that cost on the user's first hotkey pushed cold activation
/// to the edge of the budget. Showing far offscreen at startup pays it early
/// without a visible flash, and the renderer's `Ready` hides it again.
fn warm_up(shell: &Arc<TauriShell>, window: &WebviewWindow, core: &Arc<Core>) {
    let _ = window.set_position(tauri::LogicalPosition::new(-10_000.0, -10_000.0));
    let _ = window.show();

    // Backstop: if the renderer never reports back, the window must not be left
    // shown offscreen.
    let shell = shell.clone();
    core.runtime().spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        if shell.warming.load(Ordering::SeqCst) {
            eprintln!("[dango] renderer never reported mounting; hiding warmup window");
            let inner = shell.clone();
            shell.on_main_thread(Box::new(move || inner.finish_warmup()));
        }
    });
}

pub fn run() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    tauri::async_runtime::set(runtime.handle().clone());

    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(core) = core(app) {
                core.show(None);
            }
        }))
        .invoke_handler(tauri::generate_handler![dispatch])
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let window: WebviewWindow = app.get_webview_window("main").expect("main window");
            let handle = raw(&window).expect("main window handle");
            let shell = Arc::new(TauriShell {
                app: app.handle().clone(),
                launcher: Mutex::new(platform::launcher_window(handle)),
                warming: AtomicBool::new(true),
            });

            // The webview loads cached icons through the asset protocol.
            let _ = app
                .asset_protocol_scope()
                .allow_directory(app::cache_dir(), true);

            TrayIconBuilder::with_id(TRAY_ID)
                .icon(app.default_window_icon().expect("bundled icon").clone())
                .tooltip(shell::tooltip())
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| {
                    if let (Some(core), Some(command)) =
                        (core(app), tray_command(event.id().as_ref()))
                    {
                        core.tray_command(command);
                    }
                })
                .build(app)?;

            let core = app::start(Startup {
                shell: shell.clone(),
                runtime: tauri::async_runtime::handle().inner().clone(),
                updates: Arc::new(PluginSource::new(app.handle().clone())),
            });
            app.manage(core.clone());

            // Clicking away from a non-activating panel does not reliably blur
            // the webview, so the renderer's own blur handler is not enough on
            // its own: the launcher would stay on screen without keyboard focus,
            // visible but dead. The window's focus event is the reliable signal.
            {
                let core = core.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::Focused(false) = event {
                        core.hide_on_focus_loss();
                    }
                });
            }

            warm_up(&shell, &window, &core);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tray_command_round_trips_through_its_menu_id() {
        for command in [
            TrayCommand::Toggle,
            TrayCommand::Quit,
            TrayCommand::Grant(Permission::Accessibility),
            TrayCommand::Grant(Permission::InputMonitoring),
        ] {
            assert_eq!(tray_command(menu_id(command)), Some(command));
        }
    }
}
