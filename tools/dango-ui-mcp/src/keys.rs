//! Turns readable key steps like `ctrl+k` or `down` into the characters
//! Slint's `dispatch_key_event` expects. Slint encodes special keys as control
//! and private-use characters, which an agent can't send through a tool call.

#[derive(Debug, PartialEq)]
pub enum KeyEvent {
    Press(String),
    Release(String),
    Tap(String),
}

impl KeyEvent {
    pub fn event_type(&self) -> &'static str {
        match self {
            KeyEvent::Press(_) => "Press",
            KeyEvent::Release(_) => "Release",
            KeyEvent::Tap(_) => "PressAndRelease",
        }
    }

    pub fn text(&self) -> &str {
        match self {
            KeyEvent::Press(text) | KeyEvent::Release(text) | KeyEvent::Tap(text) => text,
        }
    }
}

pub fn parse(step: &str) -> Result<Vec<KeyEvent>, String> {
    if let Some(text) = step.strip_prefix("text:") {
        return Ok(text.chars().map(|c| KeyEvent::Tap(c.into())).collect());
    }
    if step.chars().count() == 1 {
        return Ok(vec![KeyEvent::Tap(step.into())]);
    }
    let parts: Vec<&str> = step.split('+').collect();
    let (key, held) = parts.split_last().ok_or("Empty step")?;
    let modifiers = held
        .iter()
        .map(|name| modifier(name).ok_or_else(|| format!("Unknown modifier `{name}` in `{step}`")))
        .collect::<Result<Vec<_>, _>>()?;
    let key = self::key(key).ok_or_else(|| format!("Unknown key `{key}` in `{step}`"))?;

    let mut events: Vec<KeyEvent> = modifiers
        .iter()
        .map(|m| KeyEvent::Press(m.to_string()))
        .collect();
    events.push(KeyEvent::Tap(key));
    events.extend(
        modifiers
            .iter()
            .rev()
            .map(|m| KeyEvent::Release(m.to_string())),
    );
    Ok(events)
}

fn modifier(name: &str) -> Option<char> {
    Some(match name.to_ascii_lowercase().as_str() {
        "shift" => '\u{0010}',
        "ctrl" | "control" => '\u{0011}',
        "alt" | "option" => '\u{0012}',
        "altgr" => '\u{0013}',
        "meta" | "cmd" | "win" | "super" => '\u{0017}',
        _ => return None,
    })
}

fn key(name: &str) -> Option<String> {
    let mut chars = name.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        return Some(c.to_lowercase().collect());
    }
    let lower = name.to_ascii_lowercase();
    if let Some(n) = lower.strip_prefix('f').and_then(|n| n.parse::<u32>().ok()) {
        return (1..=24)
            .contains(&n)
            .then(|| char::from_u32(0xF703 + n).unwrap().into());
    }
    let c = match lower.as_str() {
        "backspace" => '\u{0008}',
        "tab" => '\u{0009}',
        "enter" | "return" => '\u{000a}',
        "escape" | "esc" => '\u{001b}',
        "backtab" => '\u{0019}',
        "delete" | "del" => '\u{007f}',
        "space" => ' ',
        "up" => '\u{F700}',
        "down" => '\u{F701}',
        "left" => '\u{F702}',
        "right" => '\u{F703}',
        "insert" => '\u{F727}',
        "home" => '\u{F729}',
        "end" => '\u{F72B}',
        "pageup" => '\u{F72C}',
        "pagedown" => '\u{F72D}',
        "menu" => '\u{F735}',
        _ => return None,
    };
    Some(c.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use KeyEvent::*;

    #[test]
    fn a_named_key_becomes_slints_code() {
        assert_eq!(parse("down").unwrap(), [Tap("\u{F701}".into())]);
        assert_eq!(parse("f1").unwrap(), [Tap("\u{F704}".into())]);
    }

    #[test]
    fn a_chord_holds_its_modifiers_around_the_key() {
        assert_eq!(
            parse("ctrl+shift+K").unwrap(),
            [
                Press("\u{11}".into()),
                Press("\u{10}".into()),
                Tap("k".into()),
                Release("\u{10}".into()),
                Release("\u{11}".into()),
            ]
        );
    }

    #[test]
    fn text_types_each_character() {
        assert_eq!(
            parse("text:a+b").unwrap(),
            [Tap("a".into()), Tap("+".into()), Tap("b".into())]
        );
    }

    #[test]
    fn a_single_character_is_that_key() {
        assert_eq!(parse("+").unwrap(), [Tap("+".into())]);
    }

    #[test]
    fn unknown_names_are_refused() {
        assert!(parse("hyper+k").is_err());
        assert!(parse("ctrl+nope").is_err());
    }
}
