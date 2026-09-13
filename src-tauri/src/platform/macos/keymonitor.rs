//! The macOS key monitor: a listen-only `CGEventTap` reporting the characters
//! each keystroke produced, for keyword expansion.
//!
//! It never swallows a key - the tap is created `ListenOnly`, so it structurally
//! cannot. For each key-down it asks the event which characters it produced,
//! which is the whole reason this is hand-rolled rather than taken from a crate:
//! matching typed text is a question about characters, not key positions.
//!
//! The callback hands off and does nothing else. The spike found the system
//! never disables a slow tap on this path, which means a slow callback silently
//! slows the whole machine instead of failing visibly - handing off immediately
//! is the only defence, not hygiene.
//!
//! Dango's own output is filtered two ways, because it arrives two ways. The
//! hyperkey's tap key carries `DANGO_INJECTED` in the event source's user data
//! and is filtered on the marker. The expansion's backspaces and paste go
//! through enigo, which stamps nothing on macOS, so expansion mutes the monitor
//! for its duration instead. See design.md.

use objc2_core_graphics::{CGEvent, CGEventField, CGEventFlags, CGEventTapOptions, CGEventType};

use super::eventtap::{mask_bit, EventTap, Verdict};
use super::{muted, DANGO_INJECTED};
use crate::platform::{KeyMonitor, KeyStroke};

type Sink = Box<dyn Fn(KeyStroke) + Send>;

pub struct MacKeyMonitor {
    _tap: EventTap,
}

impl MacKeyMonitor {
    pub fn start(sink: Sink) -> Option<Self> {
        let tap = EventTap::start(
            CGEventTapOptions::ListenOnly,
            mask_bit(CGEventType::KeyDown),
            Box::new(move |_kind, event| {
                if !muted() && !is_injected(event) {
                    if let Some(stroke) = translate(event) {
                        sink(stroke);
                    }
                }
                Verdict::Pass
            }),
        );

        match tap {
            Some(tap) => Some(Self { _tap: tap }),
            None => {
                eprintln!("[dango] keyword expansion needs the Accessibility permission");
                None
            }
        }
    }
}

impl KeyMonitor for MacKeyMonitor {}

fn is_injected(event: &CGEvent) -> bool {
    CGEvent::integer_value_field(Some(event), CGEventField::EventSourceUserData) == DANGO_INJECTED
}

/// A chord with anything but Shift is not text, so it clears the buffer.
/// Modifier keys themselves never reach here: macOS reports them as
/// `FlagsChanged`, which this tap does not ask for.
fn translate(event: &CGEvent) -> Option<KeyStroke> {
    let flags = CGEvent::flags(Some(event));
    if flags.intersects(
        CGEventFlags::MaskControl | CGEventFlags::MaskAlternate | CGEventFlags::MaskCommand,
    ) {
        return Some(KeyStroke::Clear);
    }

    let mut buffer = [0u16; 8];
    let mut length: u64 = 0;
    unsafe {
        CGEvent::keyboard_get_unicode_string(
            Some(event),
            buffer.len() as u64,
            &mut length,
            buffer.as_mut_ptr(),
        );
    }

    Some(characters(&buffer[..(length as usize).min(buffer.len())]))
}

/// Exactly one unit, a non-control character, is text. Anything else - a
/// function key producing nothing, a control character, or a surrogate pair -
/// clears. Pure, so the decision is tested without a tap.
fn characters(units: &[u16]) -> KeyStroke {
    match units {
        [unit] => match char::from_u32(*unit as u32) {
            Some(c) if !c.is_control() => KeyStroke::Char(c),
            _ => KeyStroke::Clear,
        },
        _ => KeyStroke::Clear,
    }
}

#[cfg(test)]
mod tests {
    use super::characters;
    use crate::platform::KeyStroke;

    #[test]
    fn one_printable_unit_is_a_character() {
        assert_eq!(characters(&['a' as u16]), KeyStroke::Char('a'));
        assert_eq!(characters(&['Ä' as u16]), KeyStroke::Char('Ä'));
    }

    #[test]
    fn a_control_character_clears() {
        assert_eq!(characters(&[0x0D]), KeyStroke::Clear);
        assert_eq!(characters(&[0x1B]), KeyStroke::Clear);
    }

    #[test]
    fn nothing_and_more_than_one_unit_clear() {
        assert_eq!(characters(&[]), KeyStroke::Clear);
        assert_eq!(characters(&['a' as u16, 'b' as u16]), KeyStroke::Clear);
    }
}
