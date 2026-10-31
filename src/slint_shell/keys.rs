//! Slint's key events in the terms the launcher's bindings use, which are the
//! web's key names, lowercased.

use crate::protocol::{Modifier, Shortcut};
use crate::ui::snapshot::Binding;
use crate::ui::Chord;

const MAC: bool = cfg!(target_os = "macos");

fn command_label() -> &'static str {
    if MAC {
        "⌘"
    } else {
        "Ctrl"
    }
}

/// The confirm key shown in the footer.
pub fn confirm_label(with_command: bool) -> String {
    if with_command {
        format!("{}↵", command_label())
    } else {
        "↵".to_string()
    }
}

/// The chord shown on the Actions button.
pub fn panel_label() -> String {
    format!("{} K", command_label())
}

pub fn shortcut_label(shortcut: &Shortcut) -> String {
    let mut parts: Vec<String> = shortcut
        .modifiers
        .iter()
        .map(|modifier| {
            match modifier {
                Modifier::Cmd | Modifier::Ctrl => command_label(),
                Modifier::Alt if MAC => "⌥",
                Modifier::Alt => "Alt",
                Modifier::Shift if MAC => "⇧",
                Modifier::Shift => "Shift",
            }
            .to_string()
        })
        .collect();
    parts.push(shortcut.key.to_uppercase());
    parts.join(if MAC { "" } else { "+" })
}

/// The modifiers held with a key. `control` and `meta` both count as the
/// command key, as Ctrl on Windows and Cmd on macOS always have.
#[derive(Clone, Copy, Debug, Default)]
pub struct Held {
    pub control: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
}

pub fn chord(text: &str, held: Held) -> Option<Chord> {
    let key = name(text)?;
    Some(Chord {
        key,
        command: held.control || held.meta,
        alt: held.alt,
        shift: held.shift,
    })
}

/// The chord when the snapshot binds it. A multiline field keeps the chords
/// its binding leaves to it, as a template field keeps Enter for a newline.
pub fn bound(bindings: &[Binding], chord: &Chord, in_multiline: bool) -> bool {
    bindings
        .iter()
        .any(|binding| binding.chord == *chord && (!in_multiline || binding.in_multiline))
}

fn name(text: &str) -> Option<String> {
    let mut chars = text.chars();
    let first = chars.next()?;
    if chars.next().is_some() {
        return Some(text.to_lowercase());
    }
    let special = match first {
        '\u{8}' => "backspace",
        '\t' => "tab",
        '\n' | '\r' => "enter",
        '\u{1b}' => "escape",
        '\u{7f}' => "delete",
        '\u{F700}' => "arrowup",
        '\u{F701}' => "arrowdown",
        '\u{F702}' => "arrowleft",
        '\u{F703}' => "arrowright",
        '\u{F729}' => "home",
        '\u{F72B}' => "end",
        '\u{F72C}' => "pageup",
        '\u{F72D}' => "pagedown",
        // Modifiers alone, function keys, and other private-use codes.
        c if c.is_control() || ('\u{F700}'..='\u{F8FF}').contains(&c) => return None,
        c => return Some(c.to_lowercase().collect()),
    };
    Some(special.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_keys_take_their_web_names() {
        assert_eq!(
            chord("\u{1b}", Held::default()),
            Some(Chord::plain("Escape"))
        );
        assert_eq!(chord("\n", Held::default()), Some(Chord::plain("Enter")));
        assert_eq!(
            chord("\u{F700}", Held::default()),
            Some(Chord::plain("ArrowUp"))
        );
        assert_eq!(
            chord("\u{F701}", Held::default()),
            Some(Chord::plain("ArrowDown"))
        );
    }

    #[test]
    fn control_and_meta_both_count_as_the_command_key() {
        let control = Held {
            control: true,
            ..Held::default()
        };
        let meta = Held {
            meta: true,
            ..Held::default()
        };
        assert_eq!(chord("k", control), Some(Chord::command("k")));
        assert_eq!(chord("K", meta), Some(Chord::command("k")));
    }

    #[test]
    fn a_modifier_alone_is_not_a_chord() {
        assert_eq!(chord("\u{10}", Held::default()), None);
        assert_eq!(chord("\u{11}", Held::default()), None);
    }

    #[test]
    fn a_multiline_field_keeps_what_its_binding_leaves_to_it() {
        let bindings = [Binding {
            chord: Chord::plain("Enter"),
            in_multiline: false,
        }];
        assert!(bound(&bindings, &Chord::plain("Enter"), false));
        assert!(!bound(&bindings, &Chord::plain("Enter"), true));
    }
}
