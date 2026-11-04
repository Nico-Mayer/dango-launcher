//! The `display` built-in: brightness steps for external monitors over DDC/CI.

use crate::platform::Level;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
}

/// The value one step moves to: a tenth of the monitor's own range, clamped.
/// `None` for a monitor that reports no range, which counts as not answering.
pub fn stepped(level: Level, direction: Direction) -> Option<u16> {
    if level.maximum == 0 {
        return None;
    }
    let step = level.maximum.div_ceil(10);
    let current = level.value.min(level.maximum);
    Some(match direction {
        Direction::Up => current.saturating_add(step).min(level.maximum),
        Direction::Down => current.saturating_sub(step),
    })
}

pub fn percent(value: u16, maximum: u16) -> u32 {
    let (value, maximum) = (u32::from(value), u32::from(maximum));
    (value * 100 + maximum / 2) / maximum
}

/// "Brightness set to 60%", listing each distinct level, lowest first.
pub fn message(percents: &[u32]) -> String {
    let mut levels = percents.to_vec();
    levels.sort_unstable();
    levels.dedup();
    let words: Vec<String> = levels.iter().map(|level| format!("{level}%")).collect();
    let list = match words.as_slice() {
        [] => String::new(),
        [one] => one.clone(),
        [first, second] => format!("{first} and {second}"),
        [rest @ .., last] => format!("{}, and {last}", rest.join(", ")),
    };
    format!("Brightness set to {list}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level(value: u16) -> Level {
        Level {
            value,
            maximum: 100,
        }
    }

    #[test]
    fn two_monitors_each_step_from_their_own_level() {
        assert_eq!(stepped(level(30), Direction::Up), Some(40));
        assert_eq!(stepped(level(60), Direction::Up), Some(70));
    }

    #[test]
    fn a_step_stops_at_the_top() {
        let once = stepped(level(95), Direction::Up).unwrap();
        assert_eq!(once, 100);
        assert_eq!(stepped(level(once), Direction::Up), Some(100));
    }

    #[test]
    fn a_step_stops_at_the_bottom() {
        assert_eq!(stepped(level(0), Direction::Down), Some(0));
        assert_eq!(stepped(level(4), Direction::Down), Some(0));
    }

    #[test]
    fn a_step_is_a_tenth_of_the_monitors_own_range() {
        let small = Level {
            value: 25,
            maximum: 50,
        };
        assert_eq!(stepped(small, Direction::Up), Some(30));
        assert_eq!(percent(30, 50), 60);
        let tiny = Level {
            value: 2,
            maximum: 5,
        };
        assert_eq!(stepped(tiny, Direction::Down), Some(1));
    }

    #[test]
    fn a_monitor_without_a_range_does_not_answer() {
        let none = Level {
            value: 0,
            maximum: 0,
        };
        assert_eq!(stepped(none, Direction::Up), None);
    }

    #[test]
    fn one_level_is_named_once() {
        assert_eq!(message(&[60, 60]), "Brightness set to 60%");
    }

    #[test]
    fn different_levels_are_listed_lowest_first() {
        assert_eq!(message(&[70, 40]), "Brightness set to 40% and 70%");
        assert_eq!(
            message(&[70, 30, 50]),
            "Brightness set to 30%, 50%, and 70%"
        );
    }
}
