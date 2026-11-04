//! The `display` built-in: brightness steps for external monitors over DDC/CI.

use std::sync::{Arc, Mutex, PoisonError};

use crate::invocation::{Command, InvocationContext};
use crate::platform::{BrightnessMonitors, Level, Monitor};

pub const NO_MONITOR: &str = "Turn on DDC/CI in your monitor's menu to change brightness";
pub const REFUSED: &str = "Couldn't change monitor brightness";

/// Two steps talking to one monitor at once interleave their I2C messages,
/// which monitors answer with garbage or not at all.
static BUS: Mutex<()> = Mutex::new(());

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

pub struct BrightnessStep {
    pub monitors: Arc<dyn BrightnessMonitors>,
    pub direction: Direction,
}

impl Command for BrightnessStep {
    /// Never checks `is_cancelled`: a quick second press supersedes this run,
    /// and the press still has to count.
    fn invoke(&self, ctx: &InvocationContext) {
        let _bus = BUS.lock().unwrap_or_else(PoisonError::into_inner);
        let outcome = step_all(self.monitors.external(), self.direction, &mut |line| {
            crate::log::append(&line)
        });
        match outcome {
            Ok(message) => ctx.succeed_with(message),
            Err(message) => ctx.fail(message),
        }
    }
}

fn step_all(
    monitors: Vec<Box<dyn Monitor>>,
    direction: Direction,
    log: &mut dyn FnMut(String),
) -> Result<String, &'static str> {
    let mut answered = false;
    let mut percents = Vec::new();
    for mut monitor in monitors {
        let level = match monitor.brightness() {
            Ok(level) => level,
            Err(error) => {
                log(format!(
                    "brightness: {} did not answer: {error}",
                    monitor.name()
                ));
                continue;
            }
        };
        let Some(value) = stepped(level, direction) else {
            log(format!("brightness: {} reports no range", monitor.name()));
            continue;
        };
        answered = true;
        match monitor.set_brightness(value) {
            Ok(()) => percents.push(percent(value, level.maximum)),
            Err(error) => log(format!("brightness: {} refused: {error}", monitor.name())),
        }
    }
    match (percents.is_empty(), answered) {
        (false, _) => Ok(message(&percents)),
        (true, true) => Err(REFUSED),
        (true, false) => Err(NO_MONITOR),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::invocation::{Outcome, Sink};
    use crate::platform::MonitorError;
    use std::time::Duration;

    #[derive(Clone)]
    enum Answer {
        Level(Arc<Mutex<u16>>),
        Silent,
        Refuses(u16),
    }

    struct FakeMonitor(Answer);

    impl Monitor for FakeMonitor {
        fn name(&self) -> &str {
            "Fake"
        }
        fn brightness(&mut self) -> Result<Level, MonitorError> {
            let value = match &self.0 {
                Answer::Level(level) => *level.lock().unwrap(),
                Answer::Silent => return Err(MonitorError("timeout".into())),
                Answer::Refuses(value) => *value,
            };
            // Room for an overlapping step to read the same level.
            std::thread::sleep(Duration::from_millis(5));
            Ok(Level {
                value,
                maximum: 100,
            })
        }
        fn set_brightness(&mut self, value: u16) -> Result<(), MonitorError> {
            match &self.0 {
                Answer::Level(level) => {
                    *level.lock().unwrap() = value;
                    Ok(())
                }
                _ => Err(MonitorError("rejected".into())),
            }
        }
    }

    struct FakeMonitors(Vec<Answer>);

    impl BrightnessMonitors for FakeMonitors {
        fn external(&self) -> Vec<Box<dyn Monitor>> {
            self.0
                .iter()
                .map(|answer| Box::new(FakeMonitor(answer.clone())) as Box<dyn Monitor>)
                .collect()
        }
    }

    #[derive(Default)]
    struct SupersededSink(Mutex<Option<Outcome>>);

    impl Sink for SupersededSink {
        fn push_view(&self, _tree: crate::protocol::ViewTree) {}
        fn finish(&self, outcome: Outcome) {
            *self.0.lock().unwrap() = Some(outcome);
        }
        fn superseded(&self) -> bool {
            true
        }
    }

    fn run(answers: Vec<Answer>, direction: Direction) -> (Option<Outcome>, Vec<String>) {
        let mut lines = Vec::new();
        let monitors = FakeMonitors(answers).external();
        let outcome = match step_all(monitors, direction, &mut |line| lines.push(line)) {
            Ok(message) => Outcome::Success(Some(message)),
            Err(message) => Outcome::Failure(message.into()),
        };
        (Some(outcome), lines)
    }

    fn at(value: u16) -> (Answer, Arc<Mutex<u16>>) {
        let level = Arc::new(Mutex::new(value));
        (Answer::Level(level.clone()), level)
    }

    #[test]
    fn three_quick_presses_each_count() {
        let (answer, level) = at(50);
        let monitors: Arc<dyn BrightnessMonitors> = Arc::new(FakeMonitors(vec![answer]));
        let presses: Vec<_> = (0..3)
            .map(|_| {
                let command = BrightnessStep {
                    monitors: monitors.clone(),
                    direction: Direction::Up,
                };
                std::thread::spawn(move || {
                    let sink = Arc::new(SupersededSink::default());
                    command.invoke(&InvocationContext::for_test("display.x", sink));
                })
            })
            .collect();
        for press in presses {
            press.join().unwrap();
        }
        assert_eq!(*level.lock().unwrap(), 80);
    }

    #[test]
    fn two_monitors_report_both_levels() {
        let (low, _) = at(30);
        let (high, _) = at(60);
        let (outcome, _) = run(vec![low, high], Direction::Up);
        assert_eq!(
            outcome,
            Some(Outcome::Success(Some(
                "Brightness set to 40% and 70%".into()
            )))
        );
    }

    #[test]
    fn no_monitor_answering_says_to_turn_on_ddc_ci() {
        let (outcome, lines) = run(vec![Answer::Silent], Direction::Up);
        assert_eq!(outcome, Some(Outcome::Failure(NO_MONITOR.into())));
        assert_eq!(lines, ["brightness: Fake did not answer: timeout"]);
        let (outcome, _) = run(vec![], Direction::Up);
        assert_eq!(outcome, Some(Outcome::Failure(NO_MONITOR.into())));
    }

    #[test]
    fn every_monitor_refusing_fails_and_logs_each() {
        let (outcome, lines) = run(
            vec![Answer::Refuses(50), Answer::Refuses(20)],
            Direction::Down,
        );
        assert_eq!(outcome, Some(Outcome::Failure(REFUSED.into())));
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn one_of_two_refusing_reports_the_one_that_changed() {
        let (answer, level) = at(60);
        let (outcome, lines) = run(vec![answer, Answer::Refuses(10)], Direction::Up);
        assert_eq!(
            outcome,
            Some(Outcome::Success(Some("Brightness set to 70%".into())))
        );
        assert_eq!(*level.lock().unwrap(), 70);
        assert_eq!(lines, ["brightness: Fake refused: rejected"]);
    }

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
