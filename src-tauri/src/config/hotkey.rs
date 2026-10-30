//! Parsing a hotkey from the config file into a registrable chord.
//!
//! A hotkey is written as portable modifier tokens plus one key, `mod+shift+k`,
//! so one dotfile works on both machines: `mod` is Cmd on macOS and Ctrl on
//! Windows. The key is a physical position, not a character, so a non-US layout
//! does not shift the binding. When a chord must differ per platform, a hotkey
//! may be an object with `macos` and `windows` strings instead of a string.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tauri_plugin_global_shortcut::{Code, Modifiers};

/// A hotkey: one chord that works on both platforms, such as `mod+shift+k`
/// where `mod` is Cmd on macOS and Ctrl on Windows, or a chord per platform.
/// The key is a physical position, so a non-US layout does not move it.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Hotkey {
    Portable(String),
    PerPlatform(PerPlatform),
}

/// One value per platform. A missing side means no value there.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct PerPlatform<T = String> {
    /// The value on macOS.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub macos: Option<T>,
    /// The value on Windows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub windows: Option<T>,
}

impl<T> PerPlatform<T> {
    pub fn for_this_platform(&self) -> Option<&T> {
        #[cfg(target_os = "macos")]
        {
            self.macos.as_ref()
        }
        #[cfg(not(target_os = "macos"))]
        {
            self.windows.as_ref()
        }
    }
}

impl PerPlatform {
    pub fn for_platform(&self) -> Option<&str> {
        self.for_this_platform().map(String::as_str)
    }
}

impl Hotkey {
    /// The chord string for the platform this build runs on, if the file gave
    /// one for it.
    pub fn for_platform(&self) -> Option<&str> {
        match self {
            Hotkey::Portable(spec) => Some(spec),
            Hotkey::PerPlatform(per) => per.for_platform(),
        }
    }

    /// Parses this hotkey for the current platform, expanding `hyper` to the
    /// full four modifiers.
    pub fn parse(&self) -> Result<ParsedHotkey, HotkeyError> {
        self.parse_with_hyper(full_hyper())
    }

    /// Parses this hotkey for the current platform, expanding `hyper` to the
    /// given set so a chord matches whatever the configured hyperkey emits.
    pub fn parse_with_hyper(&self, hyper: Modifiers) -> Result<ParsedHotkey, HotkeyError> {
        let spec = self.for_platform().ok_or(HotkeyError::NoValueForPlatform)?;
        parse_with_hyper(spec, hyper)
    }
}

/// The full hyper combination, the default meaning of `hyper` when no hyperkey
/// narrows it.
pub fn full_hyper() -> Modifiers {
    Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT | Modifiers::SUPER
}

/// A hotkey ready to hand to the global shortcut registrar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParsedHotkey {
    pub modifiers: Modifiers,
    pub code: Code,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum HotkeyError {
    #[error("the hotkey is empty")]
    Empty,
    #[error("the hotkey has no key, only modifiers")]
    NoKey,
    #[error("'{0}' is not a modifier")]
    UnknownModifier(String),
    #[error("'{0}' is not a key")]
    UnknownKey(String),
    #[error("no hotkey is set for this platform")]
    NoValueForPlatform,
}

/// Parses a `mod+shift+k` style chord into modifiers and a physical key,
/// expanding `hyper` to the full four modifiers.
pub fn parse(spec: &str) -> Result<ParsedHotkey, HotkeyError> {
    parse_with_hyper(spec, full_hyper())
}

/// Parses a chord, expanding `hyper` to the given set. The launcher and command
/// hotkeys pass the configured hyperkey's set so a `hyper` chord matches what
/// the key produces.
pub fn parse_with_hyper(spec: &str, hyper: Modifiers) -> Result<ParsedHotkey, HotkeyError> {
    let parts: Vec<&str> = spec
        .split('+')
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect();
    let Some((key, mods)) = parts.split_last() else {
        return Err(HotkeyError::Empty);
    };

    let mut modifiers = Modifiers::empty();
    for token in mods {
        modifiers |= modifier(&token.to_ascii_lowercase(), hyper)?;
    }

    let code = key_code(&key.to_ascii_lowercase())?;
    Ok(ParsedHotkey { modifiers, code })
}

/// What a modifier token stands for. `Primary` is Cmd on macOS and Ctrl on
/// Windows; `Hyper` is whatever set the configured hyperkey emits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModifierToken {
    Primary,
    Control,
    Alt,
    Shift,
    Super,
    Hyper,
}

/// Every modifier token the grammar accepts. Several spell one modifier so a
/// chord reads naturally on either platform. The schema's hotkey pattern is
/// built from this same table.
pub const MODIFIER_TOKENS: &[(&str, ModifierToken)] = &[
    ("mod", ModifierToken::Primary),
    ("ctrl", ModifierToken::Control),
    ("control", ModifierToken::Control),
    ("alt", ModifierToken::Alt),
    ("opt", ModifierToken::Alt),
    ("option", ModifierToken::Alt),
    ("shift", ModifierToken::Shift),
    ("meta", ModifierToken::Super),
    ("cmd", ModifierToken::Super),
    ("command", ModifierToken::Super),
    ("super", ModifierToken::Super),
    ("win", ModifierToken::Super),
    ("windows", ModifierToken::Super),
    ("hyper", ModifierToken::Hyper),
];

/// Every named key token the grammar accepts. A single lowercase letter or
/// digit is a key too and is not listed here.
pub const KEY_TOKENS: &[(&str, Code)] = &[
    ("space", Code::Space),
    ("enter", Code::Enter),
    ("return", Code::Enter),
    ("tab", Code::Tab),
    ("escape", Code::Escape),
    ("esc", Code::Escape),
    ("backspace", Code::Backspace),
    ("delete", Code::Delete),
    ("del", Code::Delete),
    ("up", Code::ArrowUp),
    ("down", Code::ArrowDown),
    ("left", Code::ArrowLeft),
    ("right", Code::ArrowRight),
    ("home", Code::Home),
    ("end", Code::End),
    ("pageup", Code::PageUp),
    ("pagedown", Code::PageDown),
    ("comma", Code::Comma),
    ("period", Code::Period),
    ("dot", Code::Period),
    ("slash", Code::Slash),
    ("backslash", Code::Backslash),
    ("semicolon", Code::Semicolon),
    ("quote", Code::Quote),
    ("backquote", Code::Backquote),
    ("grave", Code::Backquote),
    ("minus", Code::Minus),
    ("equal", Code::Equal),
    ("leftbracket", Code::BracketLeft),
    ("rightbracket", Code::BracketRight),
    ("f1", Code::F1),
    ("f2", Code::F2),
    ("f3", Code::F3),
    ("f4", Code::F4),
    ("f5", Code::F5),
    ("f6", Code::F6),
    ("f7", Code::F7),
    ("f8", Code::F8),
    ("f9", Code::F9),
    ("f10", Code::F10),
    ("f11", Code::F11),
    ("f12", Code::F12),
];

fn modifier(token: &str, hyper: Modifiers) -> Result<Modifiers, HotkeyError> {
    let Some((_, kind)) = MODIFIER_TOKENS.iter().find(|(name, _)| *name == token) else {
        return Err(HotkeyError::UnknownModifier(token.to_string()));
    };
    Ok(match kind {
        ModifierToken::Primary => {
            #[cfg(target_os = "macos")]
            {
                Modifiers::SUPER
            }
            #[cfg(not(target_os = "macos"))]
            {
                Modifiers::CONTROL
            }
        }
        ModifierToken::Control => Modifiers::CONTROL,
        ModifierToken::Alt => Modifiers::ALT,
        ModifierToken::Shift => Modifiers::SHIFT,
        ModifierToken::Super => Modifiers::SUPER,
        ModifierToken::Hyper => hyper,
    })
}

fn key_code(token: &str) -> Result<Code, HotkeyError> {
    // Single letters and digits, the common case.
    if token.len() == 1 {
        let ch = token.chars().next().unwrap();
        if ch.is_ascii_lowercase() {
            return Ok(letter_code(ch));
        }
        if ch.is_ascii_digit() {
            return Ok(digit_code(ch));
        }
    }
    if let Some((_, code)) = KEY_TOKENS.iter().find(|(name, _)| *name == token) {
        return Ok(*code);
    }
    if is_modifier(token) {
        return Err(HotkeyError::NoKey);
    }
    Err(HotkeyError::UnknownKey(token.to_string()))
}

fn is_modifier(token: &str) -> bool {
    modifier(token, full_hyper()).is_ok()
}

fn letter_code(ch: char) -> Code {
    match ch {
        'a' => Code::KeyA,
        'b' => Code::KeyB,
        'c' => Code::KeyC,
        'd' => Code::KeyD,
        'e' => Code::KeyE,
        'f' => Code::KeyF,
        'g' => Code::KeyG,
        'h' => Code::KeyH,
        'i' => Code::KeyI,
        'j' => Code::KeyJ,
        'k' => Code::KeyK,
        'l' => Code::KeyL,
        'm' => Code::KeyM,
        'n' => Code::KeyN,
        'o' => Code::KeyO,
        'p' => Code::KeyP,
        'q' => Code::KeyQ,
        'r' => Code::KeyR,
        's' => Code::KeyS,
        't' => Code::KeyT,
        'u' => Code::KeyU,
        'v' => Code::KeyV,
        'w' => Code::KeyW,
        'x' => Code::KeyX,
        'y' => Code::KeyY,
        _ => Code::KeyZ,
    }
}

fn digit_code(ch: char) -> Code {
    match ch {
        '0' => Code::Digit0,
        '1' => Code::Digit1,
        '2' => Code::Digit2,
        '3' => Code::Digit3,
        '4' => Code::Digit4,
        '5' => Code::Digit5,
        '6' => Code::Digit6,
        '7' => Code::Digit7,
        '8' => Code::Digit8,
        _ => Code::Digit9,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_token_in_the_tables_parses() {
        for (name, _) in MODIFIER_TOKENS {
            assert!(parse(&format!("{name}+a")).is_ok(), "{name}");
        }
        for (name, code) in KEY_TOKENS {
            assert_eq!(
                parse(&format!("ctrl+{name}")).unwrap().code,
                *code,
                "{name}"
            );
        }
    }

    #[test]
    fn single_modifier_and_letter() {
        let parsed = parse("alt+space").unwrap();
        assert_eq!(parsed.modifiers, Modifiers::ALT);
        assert_eq!(parsed.code, Code::Space);
    }

    #[test]
    fn multiple_modifiers() {
        let parsed = parse("ctrl+shift+k").unwrap();
        assert_eq!(parsed.modifiers, Modifiers::CONTROL | Modifiers::SHIFT);
        assert_eq!(parsed.code, Code::KeyK);
    }

    #[test]
    fn mod_is_platform_primary() {
        let parsed = parse("mod+p").unwrap();
        #[cfg(target_os = "macos")]
        assert_eq!(parsed.modifiers, Modifiers::SUPER);
        #[cfg(not(target_os = "macos"))]
        assert_eq!(parsed.modifiers, Modifiers::CONTROL);
    }

    #[test]
    fn hyper_expands_to_four_modifiers() {
        let parsed = parse("hyper+left").unwrap();
        assert_eq!(
            parsed.modifiers,
            Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT | Modifiers::SUPER
        );
        assert_eq!(parsed.code, Code::ArrowLeft);
    }

    #[test]
    fn hyper_expands_to_a_given_set() {
        let hyper = Modifiers::CONTROL | Modifiers::ALT | Modifiers::SUPER;
        let parsed = parse_with_hyper("hyper+left", hyper).unwrap();
        assert_eq!(parsed.modifiers, hyper, "shift excluded from hyper");
        assert!(!parsed.modifiers.contains(Modifiers::SHIFT));
        assert_eq!(parsed.code, Code::ArrowLeft);
    }

    #[test]
    fn unknown_modifier_is_reported() {
        assert!(matches!(
            parse("hyprr+k"),
            Err(HotkeyError::UnknownModifier(_))
        ));
    }

    #[test]
    fn only_modifiers_is_no_key() {
        assert!(matches!(parse("ctrl+shift"), Err(HotkeyError::NoKey)));
    }

    #[test]
    fn unknown_key_is_reported() {
        assert!(matches!(
            parse("ctrl+nope"),
            Err(HotkeyError::UnknownKey(_))
        ));
    }

    #[test]
    fn per_platform_object_picks_this_platform() {
        let hk = Hotkey::PerPlatform(PerPlatform {
            macos: Some("cmd+k".into()),
            windows: Some("ctrl+alt+k".into()),
        });
        let parsed = hk.parse().unwrap();
        #[cfg(target_os = "macos")]
        assert_eq!(parsed.modifiers, Modifiers::SUPER);
        #[cfg(not(target_os = "macos"))]
        assert_eq!(parsed.modifiers, Modifiers::CONTROL | Modifiers::ALT);
        assert_eq!(parsed.code, Code::KeyK);
    }

    #[test]
    fn a_portable_string_parses() {
        let hk = Hotkey::Portable("alt+space".into());
        assert_eq!(hk.parse().unwrap().code, Code::Space);
    }
}
