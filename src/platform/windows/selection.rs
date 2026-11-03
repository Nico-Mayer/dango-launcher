//! Reading the selection without touching the keyboard.
//!
//! Windows exposes the focused element's selected text through UI Automation,
//! the same API a screen reader uses, so anything a screen reader can read this
//! can read, without the clipboard round trip of the Ctrl+Insert fallback.
//!
//! An application that exposes no text pattern still answers `Unavailable` and
//! still gets the fallback. The probe that informed this found Zed to be one:
//! its focused element is a bare window.

use uiautomation::patterns::UITextPattern;
use uiautomation::UIAutomation;

use crate::text::{with_deadline, DirectSelection, Selected};

/// How much of the selection to take. Far past anything a transform should be
/// pointed at, and it stops a runaway document range being copied whole.
const MAX_CHARACTERS: i32 = 100_000;

pub struct WindowsSelection;

impl DirectSelection for WindowsSelection {
    fn selected_text(&self) -> Selected {
        with_deadline(read)
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
