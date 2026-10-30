//! Keeps the login entry in step with `launcher.autostart`. On means the entry
//! is written again, so it follows the installed path; off removes it only when
//! one exists.

use tauri_plugin_autostart::ManagerExt;

pub fn apply(app: &tauri::AppHandle, enabled: bool) -> Result<(), String> {
    if cfg!(debug_assertions) {
        if enabled {
            crate::log::append("autostart: ignored in a development build");
        }
        return Ok(());
    }
    let manager = app.autolaunch();
    if enabled {
        return manager.enable().map_err(|error| enable_failure(&error));
    }
    match manager.is_enabled() {
        Ok(true) => manager.disable().map_err(|error| disable_failure(&error)),
        Ok(false) => Ok(()),
        Err(error) => Err(disable_failure(&error)),
    }
}

fn enable_failure(error: &dyn std::fmt::Display) -> String {
    format!("autostart: could not register Dango to start at login: {error}")
}

fn disable_failure(error: &dyn std::fmt::Display) -> String {
    format!("autostart: could not remove the login entry: {error}")
}
