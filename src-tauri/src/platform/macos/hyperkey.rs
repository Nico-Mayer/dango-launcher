//! The macOS hyperkey: an HID-level remap, then an event tap that turns the
//! remapped key into the hyper modifier.
//!
//! Two layers, for the reason design.md records. A session tap cannot own
//! CapsLock: the lock state, its LED, and the key's debounce all live in the HID
//! system below every tap location, so swallowing the event at the tap does not
//! stop the toggle. `hidutil` remaps CapsLock to F18 first, a key no Apple
//! keyboard has, and the tap then owns an ordinary key with no lock state and no
//! debounce.
//!
//! While the key is held the tap adds the configured flags to each event passing
//! through, rather than synthesizing modifier presses the way Windows must. No
//! modifier is ever pressed, so none can be left stuck down; the flags exist
//! only on events seen while the key is held.

use std::cell::Cell;
use std::time::{Duration, Instant};

use objc2_core_graphics::{
    CGEvent, CGEventField, CGEventFlags, CGEventSource, CGEventSourceStateID, CGEventTapLocation,
    CGEventTapOptions, CGEventType,
};

use super::eventtap::{mask_bit, EventTap, Verdict};
use super::{keymap, DANGO_INJECTED};
use crate::platform::{Hyperkey, HyperkeySpec, HyperkeyTrigger};

/// F18. Nothing on an Apple keyboard sends it, so the remap cannot collide with
/// a key the user still needs.
const REMAPPED_KEYCODE: i64 = 79;

/// The HID usage pages `hidutil` speaks, for the two ends of the remap.
const HID_CAPSLOCK: u64 = 0x700000039;
const HID_F18: u64 = 0x70000006D;

/// Longer than this held is a hold, not a tap. Matches the Windows threshold.
const TAP_THRESHOLD: Duration = Duration::from_millis(200);

pub struct MacHyperkey {
    _tap: EventTap,
    _remap: Remap,
}

impl MacHyperkey {
    pub fn start(spec: HyperkeySpec) -> Option<Self> {
        match spec.trigger {
            HyperkeyTrigger::CapsLock => {}
        }
        let remap = Remap::apply()?;

        let emit = flags_for(&spec);
        let tap_key = spec.tap.as_deref().and_then(keymap::tap_key);
        let held = Cell::new(false);
        let pressed_at = Cell::new(None::<Instant>);
        let other_key = Cell::new(false);

        let tap = EventTap::start(
            CGEventTapOptions::Default,
            mask_bit(CGEventType::KeyDown)
                | mask_bit(CGEventType::KeyUp)
                | mask_bit(CGEventType::FlagsChanged),
            Box::new(move |kind, event| {
                let keycode =
                    CGEvent::integer_value_field(Some(event), CGEventField::KeyboardEventKeycode);

                if keycode == REMAPPED_KEYCODE
                    && matches!(kind, CGEventType::KeyDown | CGEventType::KeyUp)
                {
                    match kind {
                        CGEventType::KeyDown => {
                            // Only the first down starts the clock: an
                            // auto-repeat down must not restart it, or a long
                            // hold would end up looking like a tap.
                            if !held.replace(true) {
                                pressed_at.set(Some(Instant::now()));
                                other_key.set(false);
                            }
                        }
                        _ => {
                            held.set(false);
                            let elapsed = pressed_at
                                .take()
                                .map(|at| at.elapsed())
                                .unwrap_or(TAP_THRESHOLD);
                            if let Some(key) = tap_key {
                                if should_tap(elapsed, other_key.get()) {
                                    send_tap(key);
                                }
                            }
                        }
                    }
                    return Verdict::Swallow;
                }

                if held.get() {
                    if kind == CGEventType::KeyDown {
                        other_key.set(true);
                    }
                    CGEvent::set_flags(Some(event), CGEvent::flags(Some(event)) | emit);
                }
                Verdict::Pass
            }),
        );

        match tap {
            Some(tap) => Some(Self {
                _tap: tap,
                _remap: remap,
            }),
            None => {
                eprintln!("[dango] the hyperkey needs the Accessibility permission");
                None
            }
        }
    }
}

impl Hyperkey for MacHyperkey {}

/// Whether a just-ended press was a tap: solitary and shorter than the
/// threshold. Pure, so the decision is tested without a tap.
fn should_tap(elapsed: Duration, other_key: bool) -> bool {
    !other_key && elapsed < TAP_THRESHOLD
}

/// SUPER is Command on macOS, which is the one mapping that does not read
/// across from the Windows side unchanged.
fn flags_for(spec: &HyperkeySpec) -> CGEventFlags {
    let mut flags = CGEventFlags::empty();
    if spec.emit.ctrl {
        flags |= CGEventFlags::MaskControl;
    }
    if spec.emit.alt {
        flags |= CGEventFlags::MaskAlternate;
    }
    if spec.emit.shift {
        flags |= CGEventFlags::MaskShift;
    }
    if spec.emit.meta {
        flags |= CGEventFlags::MaskCommand;
    }
    flags
}

/// Sends the tap key, stamped so the key monitor can tell it from the user's
/// own typing. This is the one key path Dango builds its own events for, so it
/// can carry the marker that the enigo-driven paste cannot.
fn send_tap(key: keymap::TapKey) {
    let Some(source) = CGEventSource::new(CGEventSourceStateID::HIDSystemState) else {
        return;
    };
    CGEventSource::set_user_data(Some(&source), DANGO_INJECTED);
    for down in [true, false] {
        let Some(event) = CGEvent::new_keyboard_event(Some(&source), key.keycode, down) else {
            return;
        };
        if let Some(character) = key.character {
            let mut units = [0u16; 2];
            let units = character.encode_utf16(&mut units);
            unsafe {
                CGEvent::keyboard_set_unicode_string(
                    Some(&event),
                    units.len() as u64,
                    units.as_ptr(),
                );
            }
        }
        CGEvent::post(CGEventTapLocation::HIDEventTap, Some(&event));
    }
}

/// What `UserKeyMapping` currently holds. It is one machine-wide list, so who
/// owns it decides whether the hyperkey may take it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Existing {
    Empty,
    /// Exactly Dango's own remap, left behind by a previous run that did not
    /// get to clean up. Reclaimable.
    Ours,
    /// Someone else's, such as Karabiner or a hand-written `hidutil` line.
    Foreign,
}

/// The `hidutil` remap, held for as long as the hyperkey runs.
///
/// A user who already has a remap installed keeps it, and the hyperkey reports
/// unavailable rather than silently replacing something they set up. Dango's own
/// leftover is a different thing entirely: `Drop` does not run when the process
/// is signalled or killed, so a force-quit always leaves the remap behind, and
/// treating that as a foreign mapping would disable the hyperkey for good.
struct Remap;

impl Remap {
    fn apply() -> Option<Self> {
        match existing() {
            Existing::Foreign => {
                eprintln!(
                    "[dango] the hyperkey needs hidutil's UserKeyMapping, which another tool already set"
                );
                return None;
            }
            Existing::Ours | Existing::Empty => {}
        }
        let mapping = format!(
            r#"{{"UserKeyMapping":[{{"HIDKeyboardModifierMappingSrc":{HID_CAPSLOCK},"HIDKeyboardModifierMappingDst":{HID_F18}}}]}}"#
        );
        set_mapping(&mapping).then_some(Self)
    }
}

impl Drop for Remap {
    fn drop(&mut self) {
        set_mapping(r#"{"UserKeyMapping":[]}"#);
    }
}

fn set_mapping(json: &str) -> bool {
    std::process::Command::new("hidutil")
        .args(["property", "--set", json])
        .output()
        .is_ok_and(|output| output.status.success())
}

fn existing() -> Existing {
    std::process::Command::new("hidutil")
        .args(["property", "--get", "UserKeyMapping"])
        .output()
        .map(|output| classify(&String::from_utf8_lossy(&output.stdout)))
        .unwrap_or(Existing::Foreign)
}

/// Reads `hidutil`'s printed mapping. An unset mapping prints `(null)` and an
/// empty one prints an empty list; anything else is one entry per `Src`. Pure,
/// so the three cases are tested without touching the machine's real mapping.
fn classify(text: &str) -> Existing {
    let entries = text.matches("HIDKeyboardModifierMappingSrc").count();
    if entries == 0 {
        return Existing::Empty;
    }
    let ours = entries == 1
        && text.contains(&HID_CAPSLOCK.to_string())
        && text.contains(&HID_F18.to_string());
    if ours {
        Existing::Ours
    } else {
        Existing::Foreign
    }
}

#[cfg(test)]
mod tests {
    use super::{flags_for, should_tap, TAP_THRESHOLD};
    use crate::platform::{HyperModifiers, HyperkeySpec, HyperkeyTrigger};
    use objc2_core_graphics::CGEventFlags;
    use std::time::Duration;

    fn spec(shift: bool) -> HyperkeySpec {
        HyperkeySpec {
            trigger: HyperkeyTrigger::CapsLock,
            emit: HyperModifiers {
                ctrl: true,
                alt: true,
                shift,
                meta: true,
            },
            tap: None,
        }
    }

    #[test]
    fn a_short_solitary_press_is_a_tap() {
        assert!(should_tap(Duration::from_millis(80), false));
    }

    #[test]
    fn a_long_press_is_not_a_tap() {
        assert!(!should_tap(TAP_THRESHOLD + Duration::from_millis(1), false));
    }

    #[test]
    fn a_press_with_another_key_is_not_a_tap() {
        assert!(!should_tap(Duration::from_millis(80), true));
    }

    #[test]
    fn an_unset_or_empty_mapping_is_free_to_take() {
        use super::{classify, Existing};
        assert_eq!(classify("(null)\n"), Existing::Empty);
        assert_eq!(classify("(\n)\n"), Existing::Empty);
    }

    #[test]
    fn dangos_own_leftover_mapping_is_reclaimed() {
        use super::{classify, Existing};
        // What a previous run leaves behind when it is killed before Drop runs.
        let left_behind = "(\n  {\n    HIDKeyboardModifierMappingDst = 30064771181;\n    \
                           HIDKeyboardModifierMappingSrc = 30064771129;\n  }\n)";
        assert_eq!(classify(left_behind), Existing::Ours);
    }

    #[test]
    fn another_tools_mapping_is_left_alone() {
        use super::{classify, Existing};
        let karabiner = "(\n  {\n    HIDKeyboardModifierMappingDst = 30064771110;\n    \
                         HIDKeyboardModifierMappingSrc = 30064771129;\n  }\n)";
        assert_eq!(classify(karabiner), Existing::Foreign);

        let two_entries = "(\n  {\n    HIDKeyboardModifierMappingDst = 30064771181;\n    \
                           HIDKeyboardModifierMappingSrc = 30064771129;\n  },\n  {\n    \
                           HIDKeyboardModifierMappingDst = 30064771111;\n    \
                           HIDKeyboardModifierMappingSrc = 30064771112;\n  }\n)";
        assert_eq!(classify(two_entries), Existing::Foreign);
    }

    #[test]
    fn super_becomes_command_and_shift_can_be_dropped() {
        let all = flags_for(&spec(true));
        assert!(all.contains(CGEventFlags::MaskCommand), "SUPER is Command");
        assert!(all.contains(CGEventFlags::MaskControl));
        assert!(all.contains(CGEventFlags::MaskAlternate));
        assert!(all.contains(CGEventFlags::MaskShift));

        let without = flags_for(&spec(false));
        assert!(!without.contains(CGEventFlags::MaskShift));
        assert!(without.contains(CGEventFlags::MaskCommand));
    }
}
