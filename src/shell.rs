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
}

/// Runs work on the main thread, inline when already there.
///
/// AppKit traps rather than misbehaving: ordering a window from another thread
/// aborts the process. Queuing from the main thread would deadlock a caller
/// that waits for the result.
pub fn on_main_thread(shell: &dyn Shell, work: impl FnOnce() + Send + 'static) {
    if crate::platform::runs_inline_on_main_thread() {
        work();
        return;
    }
    shell.on_main_thread(Box::new(work));
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

impl TrayStatus {
    /// The disabled status lines. The grants and commands around them are in
    /// the Slint menu.
    pub fn lines(&self) -> Vec<String> {
        let config = config_status_text(self.config_error).to_string();
        self.notices.iter().cloned().chain([config]).collect()
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

    type Work = Box<dyn FnOnce() + Send>;

    /// Runs main-thread work inline, or queued until `run_main_thread` once
    /// `queue_main_thread` is called, and records everything else.
    #[derive(Default)]
    pub struct RecordingShell {
        pub calls: Mutex<Vec<Call>>,
        pub published: Mutex<Vec<Snapshot>>,
        queued: Mutex<Option<Vec<Work>>>,
    }

    impl RecordingShell {
        pub fn queue_main_thread(&self) {
            *self.queued.lock().unwrap() = Some(Vec::new());
        }

        pub fn run_main_thread(&self) {
            loop {
                let work: Vec<Work> = match self.queued.lock().unwrap().as_mut() {
                    Some(queue) => std::mem::take(queue),
                    None => return,
                };
                if work.is_empty() {
                    return;
                }
                for item in work {
                    item();
                }
            }
        }

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
            let mut queued = self.queued.lock().unwrap();
            match queued.as_mut() {
                Some(queue) => queue.push(work),
                None => {
                    drop(queued);
                    work();
                }
            }
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
    fn notices_come_before_the_config_status() {
        let tray = TrayStatus {
            notices: vec!["Database unavailable, see dango.log".into()],
            config_error: true,
            missing: vec![],
        };
        assert_eq!(
            tray.lines(),
            [
                "Database unavailable, see dango.log",
                "Config file has an error, see dango.log",
            ]
        );
    }
}
