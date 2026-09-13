//! Resolving a key name in the launcher's grammar to something macOS can post.

/// A key to synthesize. Named keys resolve to their virtual keycode; a key named
/// by a single character rides on a keycode of zero with the character attached
/// to the event, which delivers it without a reverse layout lookup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct TapKey {
    pub keycode: u16,
    pub character: Option<char>,
}

const ESCAPE: u16 = 53;
const TAB: u16 = 48;
const RETURN: u16 = 36;
const SPACE: u16 = 49;
const BACKSPACE: u16 = 51;
const FORWARD_DELETE: u16 = 117;

/// The same small allowlist the Windows side resolves, so one config means the
/// same key on both. An unknown name returns `None`, which disables the tap.
pub(super) fn tap_key(name: &str) -> Option<TapKey> {
    let name = name.trim().to_ascii_lowercase();
    let named = |keycode| {
        Some(TapKey {
            keycode,
            character: None,
        })
    };
    match name.as_str() {
        "escape" | "esc" => named(ESCAPE),
        "tab" => named(TAB),
        "enter" | "return" => named(RETURN),
        "space" => named(SPACE),
        "backspace" => named(BACKSPACE),
        "delete" | "del" => named(FORWARD_DELETE),
        _ => {
            let mut chars = name.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => Some(TapKey {
                    keycode: 0,
                    character: Some(c),
                }),
                _ => None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{tap_key, ESCAPE};

    #[test]
    fn named_keys_resolve_to_a_keycode() {
        assert_eq!(tap_key("escape"), tap_key("esc"));
        assert_eq!(tap_key("Escape").unwrap().keycode, ESCAPE);
        assert!(tap_key("tab").unwrap().character.is_none());
    }

    #[test]
    fn a_single_character_rides_on_the_event() {
        let key = tap_key("x").unwrap();
        assert_eq!(key.keycode, 0);
        assert_eq!(key.character, Some('x'));
    }

    #[test]
    fn an_unknown_name_disables_the_tap() {
        assert!(tap_key("not-a-key").is_none());
        assert!(tap_key("").is_none());
    }
}
