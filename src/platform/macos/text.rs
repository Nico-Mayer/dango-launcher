//! The macOS half of selection and paste.
//!
//! Three things live here and nothing else does: the keystrokes, the wait for
//! the panel to stop being key, and reading the selection through the
//! accessibility API. Everything about borrowing the clipboard is shared.
//!
//! All of it is gated behind the Accessibility permission. Without it, a posted
//! event is discarded silently by the system, which is the worst failure
//! available, so nothing is sent until `AXIsProcessTrusted` says it is worth
//! sending.

use std::ptr::NonNull;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use enigo::{Direction, Enigo, Key, Keyboard as _, Settings};
use objc2_app_kit::NSWorkspace;
use objc2_application_services::{AXError, AXUIElement};
use objc2_core_foundation::{CFRetained, CFString, CFType};
use objc2_core_graphics::{CGEvent, CGEventFlags, CGEventSource, CGEventSourceStateID};

use super::DANGO_INJECTED;

use crate::text::{
    with_deadline, DirectSelection, Handoff, Keys, Lend, MainThread, Selected, TextError,
};

const FOCUSED_ELEMENT: &str = "AXFocusedUIElement";
const SELECTED_TEXT: &str = "AXSelectedText";
const SELECTED_TEXT_RANGE: &str = "AXSelectedTextRange";

const KEY_COMMAND: u16 = 55;
/// `kVK_ANSI_C`. The C key sits in the same place on the US and German layouts.
const KEY_C: u16 = 8;

/// How long to wait for the target application to take the paste before the
/// clipboard is put back. macOS offers no signal that a paste has been handled,
/// unlike Windows, so this is a delay rather than an answer.
///
/// Measured rather than guessed. In Notes, restoring 80ms after the keystroke
/// loses the race intermittently and 100ms upward never did across repeated
/// trials. This carries headroom over that for applications slower than Notes,
/// and it costs nothing the user sees: the text has already arrived, and only
/// the clipboard going back is waiting.
const PASTE_SETTLE: Duration = Duration::from_millis(300);

/// How long the hide needs before a keystroke can be posted. The panel is
/// dismissed on the main thread and this runs off it, so this is time for that
/// to land rather than a check that it has.
const HIDE_SETTLE: Duration = Duration::from_millis(60);

/// Long enough that a briefly busy main thread still runs the keystroke, short
/// enough that a wedged one fails rather than hanging the action forever.
const MAIN_THREAD_TIMEOUT: Duration = Duration::from_secs(2);

pub struct MacKeys {
    /// Empty until the first keystroke. enigo refuses to start without the
    /// Accessibility permission, so connecting at startup would leave a Dango
    /// launched before the grant unable to type until it restarts.
    enigo: Arc<Mutex<Option<Enigo>>>,
    main: Arc<dyn MainThread>,
}

impl MacKeys {
    pub fn new(main: Arc<dyn MainThread>) -> Self {
        Self {
            enigo: Arc::new(Mutex::new(None)),
            main,
        }
    }

    fn chord(&self, letter: char) -> Result<(), TextError> {
        self.on_main(move |enigo| {
            enigo.key(Key::Meta, Direction::Press)?;
            let pressed = enigo.key(Key::Unicode(letter), Direction::Click);
            enigo.key(Key::Meta, Direction::Release)?;
            pressed
        })
    }

    /// `Key::Unicode` is the one that reaches the keyboard layout, so every
    /// synthesis goes across rather than only the ones known to.
    fn on_main(
        &self,
        work: impl FnOnce(&mut Enigo) -> Result<(), enigo::InputError> + Send + 'static,
    ) -> Result<(), TextError> {
        let enigo = self.enigo.clone();
        let (done, wait) = std::sync::mpsc::channel();
        self.main.run(Box::new(move || {
            let mut slot = enigo.lock().unwrap();
            if slot.is_none() {
                match connect() {
                    Ok(connected) => *slot = Some(connected),
                    Err(error) => {
                        let _ = done.send(Err(error));
                        return;
                    }
                }
            }
            if let Some(enigo) = slot.as_mut() {
                let _ = done.send(work(enigo).map_err(|error| {
                    eprintln!("[dango] keystroke failed: {error}");
                    keystroke_failed()
                }));
            }
        }));
        match wait.recv_timeout(MAIN_THREAD_TIMEOUT) {
            Ok(result) => result,
            Err(_) => {
                eprintln!("[dango] the main thread did not run the keystroke");
                Err(keystroke_failed())
            }
        }
    }
}

fn connect() -> Result<Enigo, TextError> {
    let settings = Settings {
        // The user got here by pressing a hotkey and may still be holding
        // its modifiers. Without this, those modifiers ride along with the
        // injected paste.
        independent_of_keyboard_state: true,
        // Dango asks for the permission deliberately, through an action the
        // user chose. A dialog appearing by itself the first time they
        // paste, from a process they cannot see, is worse.
        open_prompt_to_get_permissions: false,
        ..Default::default()
    };
    Enigo::new(&settings).map_err(|error| match error {
        enigo::NewConError::NoPermission => TextError::PermissionMissing,
        error => {
            eprintln!("[dango] no key injection available: {error}");
            keystroke_failed()
        }
    })
}

fn keystroke_failed() -> TextError {
    TextError::TargetUnavailable(crate::text::KEYSTROKE_FAILED.into())
}

impl Keys for MacKeys {
    /// Posted to the frontmost application's process rather than through
    /// enigo, which posts to the key window: the launcher panel stays key
    /// during a read.
    fn copy(&self) -> Result<(), TextError> {
        let pid = frontmost_pid().ok_or_else(keystroke_failed)?;
        let source =
            CGEventSource::new(CGEventSourceStateID::Private).ok_or_else(keystroke_failed)?;
        CGEventSource::set_user_data(Some(&source), DANGO_INJECTED);
        for (keycode, down, flags) in [
            (KEY_COMMAND, true, CGEventFlags::MaskCommand),
            (KEY_C, true, CGEventFlags::MaskCommand),
            (KEY_C, false, CGEventFlags::MaskCommand),
            (KEY_COMMAND, false, CGEventFlags::empty()),
        ] {
            let event = CGEvent::new_keyboard_event(Some(&source), keycode, down)
                .ok_or_else(keystroke_failed)?;
            CGEvent::set_flags(Some(&event), flags);
            CGEvent::post_to_pid(pid, Some(&event));
        }
        Ok(())
    }

    fn paste(&self) -> Result<(), TextError> {
        self.chord('v')
    }

    fn caret_left(&self, times: usize) -> Result<(), TextError> {
        self.on_main(move |enigo| {
            for _ in 0..times {
                enigo.key(Key::LeftArrow, Direction::Click)?;
            }
            Ok(())
        })
    }

    fn backspace(&self, times: usize) -> Result<(), TextError> {
        self.on_main(move |enigo| {
            for _ in 0..times {
                enigo.key(Key::Backspace, Direction::Click)?;
            }
            Ok(())
        })
    }

    fn permitted(&self) -> bool {
        super::accessibility_trusted()
    }
}

/// The launcher is a non-activating panel, so the application the user was in
/// never stopped being the active one and there is nothing to restore. Before a
/// keystroke the panel has to be off screen and no longer taking keys, which
/// the caller has already asked for by the time `yield_to_previous` runs.
pub struct MacHandoff;

impl Handoff for MacHandoff {
    /// A settle, not a check. Asking AppKit whether the panel is still the key
    /// window only works on the main thread, and this runs off it by design:
    /// the hide that just happened needs the main thread's run loop to take
    /// effect, so blocking there would prevent the very thing being waited for.
    fn yield_to_previous(&self) -> Result<(), TextError> {
        std::thread::sleep(HIDE_SETTLE);
        Ok(())
    }

    /// The panel never took activation, so the application behind it is
    /// already frontmost and the panel stays key.
    fn lend_to_previous(&self) -> Lend {
        Lend::nothing_to_give_back()
    }

    fn settle_after_paste(&self) {
        std::thread::sleep(PASTE_SETTLE);
    }
}

/// The accessibility route: focused element, then selected text. Touches
/// nothing, so it is tried before the clipboard round trip.
///
/// The application's own element is asked first, and the system-wide one only
/// as a fallback. The spike found Ghostty answering through the application
/// element on every selection while the system-wide element returned
/// `CannotComplete` every single time, which is the opposite of what the
/// documentation's usual example suggests.
pub struct MacSelection;

impl DirectSelection for MacSelection {
    /// Bounded by a thread rather than `AXUIElementSetMessagingTimeout`: set
    /// on the system-wide fallback element, that timeout becomes the global
    /// one, window management included.
    fn selected_text(&self) -> Selected {
        let pid = frontmost_pid();
        with_deadline(move || read(pid))
    }
}

fn read(pid: Option<i32>) -> Selected {
    pid.and_then(|pid| answer_from(unsafe { AXUIElement::new_application(pid) }))
        .or_else(|| answer_from(unsafe { AXUIElement::new_system_wide() }))
        .unwrap_or(Selected::Unavailable)
}

fn frontmost_pid() -> Option<i32> {
    NSWorkspace::sharedWorkspace()
        .frontmostApplication()
        .map(|app| app.processIdentifier())
}

fn answer_from(root: CFRetained<AXUIElement>) -> Option<Selected> {
    unsafe {
        let focused = copy_attribute(&root, FOCUSED_ELEMENT)
            .ok()
            .flatten()?
            .downcast::<AXUIElement>()
            .ok()?;
        let text = copy_attribute(&focused, SELECTED_TEXT)
            .ok()
            .flatten()
            .and_then(|value| value.downcast::<CFString>().ok())
            .map(|text| text.to_string());
        let answer = Answer {
            own: element_pid(&focused) == Some(std::process::id() as libc::pid_t),
            text,
            has_range: matches!(copy_attribute(&focused, SELECTED_TEXT_RANGE), Ok(Some(_))),
        };
        answer.selected()
    }
}

fn element_pid(element: &AXUIElement) -> Option<libc::pid_t> {
    let mut pid: libc::pid_t = 0;
    let status = unsafe { element.pid(NonNull::from(&mut pid)) };
    (status == AXError::Success).then_some(pid)
}

struct Answer {
    own: bool,
    text: Option<String>,
    has_range: bool,
}

impl Answer {
    /// GPUI apps answer `""` for an element with no text at all, and the
    /// panel's own search field answers `""` too. Neither is a real "nothing
    /// selected", so both go to the copy fallback.
    fn selected(self) -> Option<Selected> {
        if self.own {
            return None;
        }
        match self.text? {
            text if !text.is_empty() => Some(Selected::Text(text)),
            _ if self.has_range => Some(Selected::Empty),
            _ => None,
        }
    }
}

/// `None` covers both "this element has no such attribute" and "it has no
/// value", which are the same answer here: ask the clipboard instead.
unsafe fn copy_attribute(
    element: &AXUIElement,
    attribute: &str,
) -> Result<Option<CFRetained<CFType>>, AXError> {
    let name = CFString::from_str(attribute);
    let mut value: *const CFType = std::ptr::null();
    let status = unsafe { element.copy_attribute_value(&name, NonNull::from(&mut value)) };
    match status {
        AXError::Success => {
            Ok(NonNull::new(value.cast_mut()).map(|value| unsafe { CFRetained::from_raw(value) }))
        }
        AXError::NoValue | AXError::AttributeUnsupported => Ok(None),
        other => Err(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn answer(own: bool, text: Option<&str>, has_range: bool) -> Option<Selected> {
        Answer {
            own,
            text: text.map(str::to_owned),
            has_range,
        }
        .selected()
    }

    #[test]
    fn selected_text_is_the_answer() {
        assert_eq!(
            answer(false, Some("hello"), true),
            Some(Selected::Text("hello".into()))
        );
    }

    #[test]
    fn an_empty_answer_with_a_range_is_empty() {
        assert_eq!(answer(false, Some(""), true), Some(Selected::Empty));
    }

    #[test]
    fn an_empty_answer_without_a_range_is_no_answer() {
        assert_eq!(answer(false, Some(""), false), None);
    }

    #[test]
    fn dangos_own_element_is_no_answer() {
        assert_eq!(answer(true, Some(""), true), None);
        assert_eq!(answer(true, Some("query"), true), None);
    }

    #[test]
    fn an_element_without_selected_text_is_no_answer() {
        assert_eq!(answer(false, None, true), None);
    }
}
