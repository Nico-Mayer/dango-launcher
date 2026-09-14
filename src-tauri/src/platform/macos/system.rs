//! The macOS side of the `system` commands.

use objc2_app_kit::NSWorkspace;

use super::{RunningApp, SystemControl, SystemError};

pub struct MacSystemControl;

impl SystemControl for MacSystemControl {
    /// `CGSession -suspend`, the classic route, stopped shipping. This is what
    /// the system's own lock menu item calls. It is private, so it is looked up
    /// at run time and its absence is reported rather than assumed away.
    fn lock(&self) -> Result<(), SystemError> {
        const FRAMEWORK: &std::ffi::CStr =
            c"/System/Library/PrivateFrameworks/login.framework/login";
        const SYMBOL: &std::ffi::CStr = c"SACLockScreenImmediate";

        unsafe {
            let handle = libc::dlopen(FRAMEWORK.as_ptr(), libc::RTLD_LAZY);
            if handle.is_null() {
                return Err(SystemError::Failed(
                    "Lock screen isn't supported on this version of macOS.".into(),
                ));
            }
            let symbol = libc::dlsym(handle, SYMBOL.as_ptr());
            if symbol.is_null() {
                return Err(SystemError::Failed(
                    "Lock screen isn't supported on this version of macOS.".into(),
                ));
            }
            let lock_screen: extern "C" fn() -> i32 = std::mem::transmute(symbol);
            match lock_screen() {
                0 => Ok(()),
                status => {
                    eprintln!("[dango] the lock call returned {status}");
                    Err(SystemError::Failed(
                        "The screen didn't lock. Try again.".into(),
                    ))
                }
            }
        }
    }

    fn sleep(&self) -> Result<(), SystemError> {
        run(
            "/usr/bin/pmset",
            &["sleepnow"],
            "Sleep didn't start. Try again.",
        )
    }

    /// Through Finder, not the filesystem. `~/.Trash` is behind Full Disk
    /// Access, which a launcher has no business asking for, while Finder
    /// already has it. The cost is that the first call prompts once for
    /// permission to control Finder.
    fn trash_count(&self) -> Result<usize, SystemError> {
        let count = osascript(
            "tell application \"Finder\" to count items of trash",
            "Couldn't check the Trash.",
        )?;
        count.trim().parse().map_err(|_| {
            eprintln!("[dango] Finder answered '{}'", count.trim());
            SystemError::Failed("Couldn't check the Trash.".into())
        })
    }

    fn empty_trash(&self) -> Result<(), SystemError> {
        osascript(
            "tell application \"Finder\" to empty trash",
            "Couldn't empty the Trash.",
        )
        .map(|_| ())
    }

    fn running_apps(&self) -> Vec<RunningApp> {
        let workspace = NSWorkspace::sharedWorkspace();
        let ours = std::process::id() as i32;
        workspace
            .runningApplications()
            .iter()
            .filter(|app| {
                // Regular means it has a menu bar and appears in the switcher.
                // Agents and daemons are not things a user thinks of quitting.
                app.activationPolicy() == objc2_app_kit::NSApplicationActivationPolicy::Regular
                    && app.processIdentifier() != ours
            })
            .map(|app| {
                let path = app
                    .bundleURL()
                    .and_then(|url| url.path())
                    .map(|path| path.to_string());
                RunningApp {
                    name: app
                        .localizedName()
                        .map(|name| name.to_string())
                        .or_else(|| path.as_deref().map(display_name))
                        .unwrap_or_else(|| "Unknown".into()),
                    icon: path.clone(),
                    id: app.processIdentifier().to_string(),
                }
            })
            .collect()
    }

    fn quit(&self, app_id: &str) -> Result<(), SystemError> {
        let pid: i32 = app_id
            .parse()
            .map_err(|_| SystemError::Failed("That isn't an app Dango can quit.".into()))?;
        let app = objc2_app_kit::NSRunningApplication::runningApplicationWithProcessIdentifier(pid)
            .ok_or(SystemError::Gone)?;
        // terminate is a request. An application holding an unsaved document
        // answers false and prompts instead, which is correct behaviour, so the
        // return value is deliberately not treated as failure.
        let _ = app.terminate();
        Ok(())
    }
}

/// Finder's own error text, including a refused permission to control it, goes
/// to the log; the user sees `failure`.
fn osascript(script: &str, failure: &str) -> Result<String, SystemError> {
    let output = std::process::Command::new("/usr/bin/osascript")
        .args(["-e", script])
        .output()
        .map_err(|error| {
            eprintln!("[dango] osascript did not start: {error}");
            SystemError::Failed(failure.into())
        })?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        let message = String::from_utf8_lossy(&output.stderr);
        eprintln!("[dango] osascript failed: {}", message.trim());
        Err(SystemError::Failed(failure.into()))
    }
}

fn display_name(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_owned())
}

fn run(program: &str, args: &[&str], failure: &str) -> Result<(), SystemError> {
    let status = std::process::Command::new(program)
        .args(args)
        .status()
        .map_err(|error| {
            eprintln!("[dango] {program} did not start: {error}");
            SystemError::Failed(failure.into())
        })?;
    if status.success() {
        Ok(())
    } else {
        eprintln!("[dango] {program} exited with {status}");
        Err(SystemError::Failed(failure.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bundle_path_falls_back_to_its_filename() {
        assert_eq!(display_name("/Applications/Safari.app"), "Safari");
    }

    /// Needs a desktop session: there is no window server in CI and no
    /// applications running to enumerate.
    mod live {
        use super::*;

        #[test]
        #[ignore]
        fn running_applications_are_listed_without_dango() {
            let apps = MacSystemControl.running_apps();
            assert!(!apps.is_empty(), "something must be running");
            assert!(apps.iter().all(|app| app.name != "dango"));
            assert!(apps.iter().all(|app| !app.id.is_empty()));
        }

        #[test]
        #[ignore]
        fn the_trash_can_be_counted() {
            assert!(MacSystemControl.trash_count().is_ok());
        }

        /// Calculator again, for the same reason the applications indexer uses
        /// it: no documents, so a stray instance costs nothing.
        #[test]
        #[ignore]
        fn a_running_application_is_asked_to_quit() {
            fn instances() -> usize {
                let output = std::process::Command::new("pgrep")
                    .args(["-x", "Calculator"])
                    .output()
                    .unwrap();
                String::from_utf8_lossy(&output.stdout).lines().count()
            }
            assert_eq!(instances(), 0, "Calculator must not be running already");

            std::process::Command::new("/usr/bin/open")
                .args(["-a", "Calculator"])
                .status()
                .unwrap();
            for _ in 0..40 {
                std::thread::sleep(std::time::Duration::from_millis(250));
                if instances() > 0 {
                    break;
                }
            }
            assert_eq!(instances(), 1, "Calculator did not start");

            let app = MacSystemControl
                .running_apps()
                .into_iter()
                .find(|app| app.name == "Calculator")
                .expect("Calculator must be listed as running");
            MacSystemControl.quit(&app.id).unwrap();

            for _ in 0..40 {
                std::thread::sleep(std::time::Duration::from_millis(250));
                if instances() == 0 {
                    break;
                }
            }
            assert_eq!(instances(), 0, "Calculator was not asked to quit");
        }

        #[test]
        #[ignore]
        fn quitting_something_that_is_not_running_reports_it_as_gone() {
            assert!(matches!(
                MacSystemControl.quit("999999"),
                Err(SystemError::Gone)
            ));
        }
    }
}
