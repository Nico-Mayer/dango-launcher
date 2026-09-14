//! Reading the selection without touching the keyboard.
//!
//! Windows exposes the focused element's selected text through UI Automation,
//! the same API a screen reader uses, so anything a screen reader can read this
//! can read. That matters beyond speed: the fallback synthesizes Ctrl+C, and a
//! modal editor is free to bind that chord to a command of its own. In a Helix
//! keymap it toggles comments, so the fallback edits the document it was meant
//! to read.
//!
//! An application that exposes no text pattern still answers `Unavailable` and
//! still gets the fallback. The probe that informed this found Zed to be one:
//! its focused element is a bare window.

use std::sync::mpsc;
use std::time::Duration;

use uiautomation::patterns::UITextPattern;
use uiautomation::UIAutomation;

use crate::text::{DirectSelection, Selected};

/// Long enough for a busy application to answer across a process boundary,
/// short enough to leave the fallback room inside the 300ms the read is given.
/// The probe measured real applications at under 10ms.
const DEADLINE: Duration = Duration::from_millis(200);

/// How much of the selection to take. Far past anything a transform should be
/// pointed at, and it stops a runaway document range being copied whole.
const MAX_CHARACTERS: i32 = 100_000;

pub struct WindowsSelection;

impl DirectSelection for WindowsSelection {
    /// The launcher holds the foreground here, so the focused element is its
    /// own search field until it gets out of the way.
    fn needs_foreground(&self) -> bool {
        true
    }

    fn selected_text(&self) -> Selected {
        let (sender, receiver) = mpsc::channel();
        // On its own thread with a deadline: this is a cross-process COM call,
        // and an application that is busy or hung must not hold the launcher.
        // A call that never returns leaks this one thread, which is the price
        // of not blocking.
        std::thread::spawn(move || {
            let _ = sender.send(read());
        });
        receiver.recv_timeout(DEADLINE).unwrap_or(Selected::Unavailable)
    }
}

fn read() -> Selected {
    let Ok(automation) = UIAutomation::new() else {
        return Selected::Unavailable;
    };
    let Ok(element) = automation.get_focused_element() else {
        return Selected::Unavailable;
    };
    let Ok(pattern) = element.get_pattern::<UITextPattern>() else {
        return Selected::Unavailable;
    };
    let Ok(ranges) = pattern.get_selection() else {
        return Selected::Unavailable;
    };

    // A column selection has a range per line. Everything downstream takes one
    // string, and the first is the one the caret is in.
    let Some(first) = ranges.first() else {
        return Selected::Empty;
    };
    match first.get_text(MAX_CHARACTERS) {
        Ok(text) if text.is_empty() => Selected::Empty,
        Ok(text) => Selected::Text(text),
        Err(_) => Selected::Unavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    /// Whatever has focus while the suite runs, the call must answer inside the
    /// deadline rather than blocking the caller.
    #[test]
    fn reading_answers_within_the_deadline() {
        let started = Instant::now();
        let _ = WindowsSelection.selected_text();
        assert!(
            started.elapsed() < DEADLINE + Duration::from_millis(100),
            "took {:?}",
            started.elapsed()
        );
    }
}
