//! Keeps the login entry in step with `launcher.autostart`. On means the entry
//! is written again, so it follows the installed path; off removes it only when
//! one exists.

use auto_launch::{AutoLaunch, AutoLaunchBuilder};

const APP_NAME: &str = "Dango";

pub fn apply(enabled: bool) -> Result<(), String> {
    if cfg!(debug_assertions) {
        if enabled {
            crate::log::append("autostart: ignored in a development build");
        }
        return Ok(());
    }
    let manager = entry().map_err(|error| enable_failure(&error))?;
    if enabled {
        return manager.enable().map_err(|error| enable_failure(&error));
    }
    match manager.is_enabled() {
        Ok(true) => manager.disable().map_err(|error| disable_failure(&error)),
        Ok(false) => Ok(()),
        Err(error) => Err(disable_failure(&error)),
    }
}

/// A launch agent on macOS rather than a login item, which needs no
/// AppleScript permission.
fn entry() -> Result<AutoLaunch, String> {
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    AutoLaunchBuilder::new()
        .set_app_name(APP_NAME)
        .set_app_path(&exe.display().to_string())
        .set_use_launch_agent(true)
        .build()
        .map_err(|error| error.to_string())
}

fn enable_failure(error: &dyn std::fmt::Display) -> String {
    format!("autostart: could not register Dango to start at login: {error}")
}

fn disable_failure(error: &dyn std::fmt::Display) -> String {
    format!("autostart: could not remove the login entry: {error}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(debug_assertions)]
    #[test]
    fn a_development_build_never_writes_a_login_entry() {
        assert_eq!(apply(true), Ok(()));
        assert_eq!(apply(false), Ok(()));
    }

    #[test]
    fn the_entry_points_at_this_executable() {
        let entry = entry().unwrap();
        assert_eq!(entry.get_app_name(), APP_NAME);
        let exe = std::env::current_exe().unwrap();
        assert_eq!(entry.get_app_path(), exe.display().to_string());
    }
}
