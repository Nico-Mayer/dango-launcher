//! The `display` built-in: brightness steps for external monitors over DDC/CI.

use std::sync::{Arc, Mutex, PoisonError};

use crate::extension::{CommandDecl, Extension, InvocationMode, Manifest, NAMED_ICON};
use crate::invocation::{Command, InvocationContext, Outcome};
use crate::platform::{BrightnessMonitors, Level, Monitor};

pub const EXTENSION_ID: &str = "dango.display";
pub const COMMAND_INCREASE: &str = "increase-brightness";
pub const COMMAND_DECREASE: &str = "decrease-brightness";

pub const NO_MONITOR: &str = "Turn on DDC/CI in your monitor's menu to change brightness";
pub const REFUSED: &str = "Couldn't change monitor brightness";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
}

/// What every press shares. One step takes about a quarter second over two
/// monitors, so presses made meanwhile add up in `pending` and the next step
/// applies them all at once.
#[derive(Default)]
pub struct Presses {
    /// Two steps talking to one monitor at once interleave their I2C
    /// messages, which monitors answer with garbage or not at all.
    bus: Mutex<()>,
    /// Signed tenths not yet applied.
    pending: Mutex<i32>,
}

/// The value a step of `tenths` tenths of the monitor's own range moves to,
/// clamped. `None` for a monitor that reports no range, which counts as not
/// answering.
pub fn stepped(level: Level, tenths: i32) -> Option<u16> {
    if level.maximum == 0 {
        return None;
    }
    let step = i64::from(level.maximum.div_ceil(10));
    let current = i64::from(level.value.min(level.maximum));
    let target = (current + step * i64::from(tenths)).clamp(0, i64::from(level.maximum));
    u16::try_from(target).ok()
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

pub struct DisplayExtension {
    manifest: Manifest,
    monitors: Arc<dyn BrightnessMonitors>,
    presses: Arc<Presses>,
}

impl DisplayExtension {
    pub fn new(monitors: Arc<dyn BrightnessMonitors>) -> Self {
        Self {
            manifest: manifest(),
            monitors,
            presses: Arc::default(),
        }
    }
}

impl Extension for DisplayExtension {
    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn command(&self, command_id: &str) -> Option<Arc<dyn Command>> {
        let direction = match command_id {
            COMMAND_INCREASE => Direction::Up,
            COMMAND_DECREASE => Direction::Down,
            _ => return None,
        };
        Some(Arc::new(BrightnessStep {
            monitors: self.monitors.clone(),
            presses: self.presses.clone(),
            direction,
        }))
    }
}

pub(crate) fn manifest() -> Manifest {
    Manifest {
        manifest_version: 1,
        id: EXTENSION_ID.into(),
        name: "Display".into(),
        icon: None,
        commands: vec![
            command(
                COMMAND_INCREASE,
                "Increase Brightness",
                "sun",
                &["brightness", "brighter", "monitor", "display", "screen"],
            ),
            command(
                COMMAND_DECREASE,
                "Decrease Brightness",
                "sun-dim",
                &[
                    "brightness",
                    "dim",
                    "dimmer",
                    "monitor",
                    "display",
                    "screen",
                ],
            ),
        ],
        root_items: false,
        services: false,
    }
}

fn command(id: &str, title: &str, icon: &str, keywords: &[&str]) -> CommandDecl {
    CommandDecl {
        id: id.into(),
        title: title.into(),
        mode: InvocationMode::NoView,
        icon: Some(format!("{NAMED_ICON}{icon}")),
        keywords: keywords.iter().map(|k| (*k).to_string()).collect(),
        alias: None,
    }
}

pub struct BrightnessStep {
    monitors: Arc<dyn BrightnessMonitors>,
    presses: Arc<Presses>,
    direction: Direction,
}

impl Command for BrightnessStep {
    /// Never checks `is_cancelled`: a quick second press supersedes this run,
    /// and the press still has to count and show its level.
    fn invoke(&self, ctx: &InvocationContext) {
        let press = match self.direction {
            Direction::Up => 1,
            Direction::Down => -1,
        };
        *lock(&self.presses.pending) += press;
        let _bus = lock(&self.presses.bus);
        // An earlier press may already have applied this one along with its own.
        let tenths = std::mem::take(&mut *lock(&self.presses.pending));
        if tenths == 0 {
            return ctx.report(Outcome::Success(None));
        }
        let outcome = step_all(self.monitors.external(), tenths, &mut |line| {
            crate::log::append(&line)
        });
        ctx.report(match outcome {
            Ok(message) => Outcome::Success(Some(message)),
            Err(message) => Outcome::Failure(message.into()),
        });
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn step_all(
    monitors: Vec<Box<dyn Monitor>>,
    tenths: i32,
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
        let Some(value) = stepped(level, tenths) else {
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
    use crate::invocation::Sink;
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

    /// Superseded like every press but the last; keeps what `report` delivers
    /// and drops what `finish` would.
    #[derive(Default)]
    struct SupersededSink(Mutex<Option<Outcome>>);

    impl Sink for SupersededSink {
        fn push_view(&self, _tree: crate::protocol::ViewTree) {}
        fn finish(&self, _outcome: Outcome) {}
        fn report(&self, outcome: Outcome) {
            *self.0.lock().unwrap() = Some(outcome);
        }
        fn superseded(&self) -> bool {
            true
        }
    }

    fn run(answers: Vec<Answer>, direction: Direction) -> (Outcome, Vec<String>) {
        let tenths = match direction {
            Direction::Up => 1,
            Direction::Down => -1,
        };
        let mut lines = Vec::new();
        let monitors = FakeMonitors(answers).external();
        let outcome = match step_all(monitors, tenths, &mut |line| lines.push(line)) {
            Ok(message) => Outcome::Success(Some(message)),
            Err(message) => Outcome::Failure(message.into()),
        };
        (outcome, lines)
    }

    fn at(value: u16) -> (Answer, Arc<Mutex<u16>>) {
        let level = Arc::new(Mutex::new(value));
        (Answer::Level(level.clone()), level)
    }

    fn press(
        monitors: &Arc<dyn BrightnessMonitors>,
        presses: &Arc<Presses>,
        direction: Direction,
    ) -> std::thread::JoinHandle<Option<Outcome>> {
        let command = BrightnessStep {
            monitors: monitors.clone(),
            presses: presses.clone(),
            direction,
        };
        std::thread::spawn(move || {
            let sink = Arc::new(SupersededSink::default());
            command.invoke(&InvocationContext::for_test("display.x", sink.clone()));
            let outcome = sink.0.lock().unwrap().clone();
            outcome
        })
    }

    #[test]
    fn three_quick_presses_each_count_and_every_step_reports() {
        let (answer, level) = at(50);
        let monitors: Arc<dyn BrightnessMonitors> = Arc::new(FakeMonitors(vec![answer]));
        let presses = Arc::new(Presses::default());
        let outcomes: Vec<_> = (0..3)
            .map(|_| press(&monitors, &presses, Direction::Up))
            .collect::<Vec<_>>()
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect();
        assert_eq!(*level.lock().unwrap(), 80);
        let reported: Vec<_> = outcomes
            .iter()
            .filter(|outcome| matches!(outcome, Some(Outcome::Success(Some(_)))))
            .collect();
        assert!(!reported.is_empty());
        assert!(outcomes.iter().all(Option::is_some), "every press reports");
    }

    #[test]
    fn presses_during_a_step_are_applied_together() {
        let (answer, level) = at(50);
        let monitors: Arc<dyn BrightnessMonitors> = Arc::new(FakeMonitors(vec![answer]));
        let presses = Arc::new(Presses::default());
        let bus = lock(&presses.bus);
        let waiting: Vec<_> = (0..4)
            .map(|_| press(&monitors, &presses, Direction::Up))
            .collect();
        while *lock(&presses.pending) < 4 {
            std::thread::sleep(Duration::from_millis(1));
        }
        drop(bus);
        let outcomes: Vec<_> = waiting
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect();
        assert_eq!(*level.lock().unwrap(), 90);
        let messages: Vec<_> = outcomes
            .iter()
            .filter_map(|outcome| match outcome {
                Some(Outcome::Success(Some(message))) => Some(message.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(
            messages,
            ["Brightness set to 90%"],
            "one step applied all four"
        );
    }

    #[test]
    fn up_and_down_during_a_step_cancel_out() {
        let (answer, level) = at(50);
        let monitors: Arc<dyn BrightnessMonitors> = Arc::new(FakeMonitors(vec![answer]));
        let presses = Arc::new(Presses::default());
        let bus = lock(&presses.bus);
        let up = press(&monitors, &presses, Direction::Up);
        while *lock(&presses.pending) != 1 {
            std::thread::sleep(Duration::from_millis(1));
        }
        let down = press(&monitors, &presses, Direction::Down);
        while *lock(&presses.pending) != 0 {
            std::thread::sleep(Duration::from_millis(1));
        }
        drop(bus);
        up.join().unwrap();
        down.join().unwrap();
        assert_eq!(*level.lock().unwrap(), 50);
    }

    #[test]
    fn two_monitors_report_both_levels() {
        let (low, _) = at(30);
        let (high, _) = at(60);
        let (outcome, _) = run(vec![low, high], Direction::Up);
        assert_eq!(
            outcome,
            Outcome::Success(Some("Brightness set to 40% and 70%".into()))
        );
    }

    #[test]
    fn no_monitor_answering_says_to_turn_on_ddc_ci() {
        let (outcome, lines) = run(vec![Answer::Silent], Direction::Up);
        assert_eq!(outcome, Outcome::Failure(NO_MONITOR.into()));
        assert_eq!(lines, ["brightness: Fake did not answer: timeout"]);
        let (outcome, _) = run(vec![], Direction::Up);
        assert_eq!(outcome, Outcome::Failure(NO_MONITOR.into()));
    }

    #[test]
    fn every_monitor_refusing_fails_and_logs_each() {
        let (outcome, lines) = run(
            vec![Answer::Refuses(50), Answer::Refuses(20)],
            Direction::Down,
        );
        assert_eq!(outcome, Outcome::Failure(REFUSED.into()));
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn one_of_two_refusing_reports_the_one_that_changed() {
        let (answer, level) = at(60);
        let (outcome, lines) = run(vec![answer, Answer::Refuses(10)], Direction::Up);
        assert_eq!(
            outcome,
            Outcome::Success(Some("Brightness set to 70%".into()))
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
        assert_eq!(stepped(level(30), 1), Some(40));
        assert_eq!(stepped(level(60), 1), Some(70));
    }

    #[test]
    fn a_step_stops_at_the_top() {
        let once = stepped(level(95), 1).unwrap();
        assert_eq!(once, 100);
        assert_eq!(stepped(level(once), 1), Some(100));
    }

    #[test]
    fn a_step_stops_at_the_bottom() {
        assert_eq!(stepped(level(0), -1), Some(0));
        assert_eq!(stepped(level(4), -1), Some(0));
    }

    #[test]
    fn a_step_is_a_tenth_of_the_monitors_own_range() {
        let small = Level {
            value: 25,
            maximum: 50,
        };
        assert_eq!(stepped(small, 1), Some(30));
        assert_eq!(percent(30, 50), 60);
        let tiny = Level {
            value: 2,
            maximum: 5,
        };
        assert_eq!(stepped(tiny, -1), Some(1));
    }

    #[test]
    fn several_tenths_apply_at_once_and_clamp() {
        assert_eq!(stepped(level(50), 3), Some(80));
        assert_eq!(stepped(level(50), 7), Some(100));
        assert_eq!(stepped(level(50), -9), Some(0));
    }

    #[test]
    fn a_monitor_without_a_range_does_not_answer() {
        let none = Level {
            value: 0,
            maximum: 0,
        };
        assert_eq!(stepped(none, 1), None);
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
