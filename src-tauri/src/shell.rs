//! What the core asks of whatever draws the launcher. The core owns every rule;
//! a shell only puts pixels on screen, moves the window, and runs the tray.

use crate::platform::Permission;
use crate::ui::Snapshot;

pub trait Shell: Send + Sync {
    /// Called from any thread.
    fn publish(&self, snapshot: &Snapshot);
    /// Positions the launcher on the active display, shows it, and gives it
    /// the keyboard. Main thread only.
    fn show(&self);
    /// `restore_focus` hands the foreground back to the window that had it
    /// before the launcher. Main thread only.
    fn hide(&self, restore_focus: bool);
    fn on_main_thread(&self, work: Box<dyn FnOnce() + Send>);
    /// Called from any thread.
    fn set_tray(&self, tray: &TrayStatus);
    fn quit(&self);
    /// The renderer has mounted. Only a shell that warms its window up at
    /// startup has anything to do.
    fn finish_warmup(&self) {}
}

/// Runs work on the main thread, inline when already there.
///
/// AppKit traps rather than misbehaving: ordering a window from another thread
/// aborts the process. Queuing from the main thread would deadlock a caller
/// that waits for the result.
pub fn on_main_thread(shell: &dyn Shell, work: impl FnOnce() + Send + 'static) {
    #[cfg(target_os = "macos")]
    if objc2_foundation::MainThreadMarker::new().is_some() {
        work();
        return;
    }
    shell.on_main_thread(Box::new(work));
}

pub fn tooltip() -> &'static str {
    if cfg!(debug_assertions) {
        "Dango (development build)"
    } else {
        "Dango"
    }
}

/// What the tray menu shows. Startup problems and the config file's health
/// have no other surface while the launcher is hidden.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TrayStatus {
    pub notices: Vec<String>,
    pub config_error: bool,
    pub missing: Vec<Permission>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayCommand {
    Toggle,
    Grant(Permission),
    Quit,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrayItem {
    pub text: String,
    /// `None` for a status line, which is shown disabled.
    pub command: Option<TrayCommand>,
}

impl TrayStatus {
    pub fn items(&self) -> Vec<TrayItem> {
        let grants = [
            (Permission::Accessibility, "Grant Accessibility"),
            (Permission::InputMonitoring, "Grant Input Monitoring"),
        ]
        .into_iter()
        .filter(|(permission, _)| self.missing.contains(permission))
        .map(|(permission, text)| TrayItem {
            text: text.into(),
            command: Some(TrayCommand::Grant(permission)),
        });
        let notices = self.notices.iter().map(|text| TrayItem {
            text: text.clone(),
            command: None,
        });
        let config = TrayItem {
            text: config_status_text(self.config_error).into(),
            command: None,
        };
        let commands = [
            ("Show or hide Dango", TrayCommand::Toggle),
            ("Quit Dango", TrayCommand::Quit),
        ]
        .map(|(text, command)| TrayItem {
            text: text.into(),
            command: Some(command),
        });
        grants
            .chain(notices)
            .chain([config])
            .chain(commands)
            .collect()
    }
}

fn config_status_text(error: bool) -> &'static str {
    if error {
        "Config file has an error, see dango.log"
    } else {
        "Config file loaded"
    }
}

#[cfg(test)]
pub mod testing {
    use std::sync::Mutex;

    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    pub enum Call {
        Show,
        Hide { restore_focus: bool },
        Tray(TrayStatus),
        Quit,
    }

    /// Runs main-thread work inline and records everything else.
    #[derive(Default)]
    pub struct RecordingShell {
        pub calls: Mutex<Vec<Call>>,
        pub published: Mutex<Vec<Snapshot>>,
    }

    impl RecordingShell {
        pub fn calls(&self) -> Vec<Call> {
            self.calls.lock().unwrap().clone()
        }

        pub fn last_snapshot(&self) -> Snapshot {
            self.published
                .lock()
                .unwrap()
                .last()
                .cloned()
                .expect("a snapshot was published")
        }
    }

    impl Shell for RecordingShell {
        fn publish(&self, snapshot: &Snapshot) {
            self.published.lock().unwrap().push(snapshot.clone());
        }
        fn show(&self) {
            self.calls.lock().unwrap().push(Call::Show);
        }
        fn hide(&self, restore_focus: bool) {
            self.calls
                .lock()
                .unwrap()
                .push(Call::Hide { restore_focus });
        }
        fn on_main_thread(&self, work: Box<dyn FnOnce() + Send>) {
            work();
        }
        fn set_tray(&self, tray: &TrayStatus) {
            self.calls.lock().unwrap().push(Call::Tray(tray.clone()));
        }
        fn quit(&self) {
            self.calls.lock().unwrap().push(Call::Quit);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tray_lists_grants_first_and_commands_last() {
        let tray = TrayStatus {
            notices: vec!["Pasting is off, see dango.log".into()],
            config_error: false,
            missing: vec![Permission::InputMonitoring],
        };
        let texts: Vec<String> = tray.items().into_iter().map(|item| item.text).collect();
        assert_eq!(
            texts,
            [
                "Grant Input Monitoring",
                "Pasting is off, see dango.log",
                "Config file loaded",
                "Show or hide Dango",
                "Quit Dango",
            ]
        );
    }

    #[test]
    fn notices_and_status_lines_are_not_commands() {
        let tray = TrayStatus {
            notices: vec!["Database unavailable, see dango.log".into()],
            config_error: true,
            missing: vec![],
        };
        let items = tray.items();
        assert_eq!(items[0].command, None);
        assert_eq!(items[1].text, "Config file has an error, see dango.log");
        assert_eq!(items[1].command, None);
    }

    #[cfg(debug_assertions)]
    #[test]
    fn a_development_build_says_so_in_the_tooltip() {
        assert_eq!(tooltip(), "Dango (development build)");
    }
}
