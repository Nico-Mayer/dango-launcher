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

use crate::text::{DirectSelection, Handoff, Keys, MainThread, Selected, TextError};

const FOCUSED_ELEMENT: &str = "AXFocusedUIElement";
const SELECTED_TEXT: &str = "AXSelectedText";

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
    fn copy(&self) -> Result<(), TextError> {
        self.chord('c')
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

    fn request_permission(&self) {
        super::prompt_for_accessibility();
    }
}

/// The launcher is a non-activating panel, so the application the user was in
/// never stopped being the active one and there is nothing to restore. What has
/// to be true is that the panel is off screen and no longer taking keys, which
/// the caller has already asked for by the time this runs.
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
    fn selected_text(&self) -> Selected {
        frontmost_pid()
            .and_then(|pid| selected_text_from(unsafe { AXUIElement::new_application(pid) }))
            .or_else(|| selected_text_from(unsafe { AXUIElement::new_system_wide() }))
            .map_or(Selected::Unavailable, Selected::Text)
    }
}

fn frontmost_pid() -> Option<i32> {
    NSWorkspace::sharedWorkspace()
        .frontmostApplication()
        .map(|app| app.processIdentifier())
}

fn selected_text_from(root: CFRetained<AXUIElement>) -> Option<String> {
    unsafe {
        let focused = copy_attribute(&root, FOCUSED_ELEMENT)
            .ok()
            .flatten()?
            .downcast::<AXUIElement>()
            .ok()?;
        let selected = copy_attribute(&focused, SELECTED_TEXT).ok().flatten()?;
        let text = selected.downcast::<CFString>().ok()?.to_string();
        (!text.is_empty()).then_some(text)
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
