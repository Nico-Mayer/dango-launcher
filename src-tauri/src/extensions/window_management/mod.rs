//! The `window-management` built-in: tiling, maximising, centring, and moving
//! the focused window across displays. Its whole contribution is no-view
//! commands, so it is `system` without the confirmations: each command computes
//! a target rectangle from shared geometry and applies it through the platform
//! `WindowManager`.

use std::sync::{Arc, Mutex};

use crate::extension::{Extension, InvocationMode, Manifest};
use crate::invocation::{Command, InvocationContext};
use crate::platform::{Rect, WindowManager};

pub mod geometry;

use geometry::{Cycle, Region, Step};

pub const EXTENSION_ID: &str = "dango.window-management";

pub const COMMAND_LEFT_HALF: &str = "left-half";
pub const COMMAND_RIGHT_HALF: &str = "right-half";
pub const COMMAND_TOP_HALF: &str = "top-half";
pub const COMMAND_BOTTOM_HALF: &str = "bottom-half";
pub const COMMAND_TOP_LEFT: &str = "top-left-quarter";
pub const COMMAND_TOP_RIGHT: &str = "top-right-quarter";
pub const COMMAND_BOTTOM_LEFT: &str = "bottom-left-quarter";
pub const COMMAND_BOTTOM_RIGHT: &str = "bottom-right-quarter";
pub const COMMAND_LEFT_THIRD: &str = "left-third";
pub const COMMAND_CENTER_THIRD: &str = "center-third";
pub const COMMAND_RIGHT_THIRD: &str = "right-third";
pub const COMMAND_MAXIMIZE: &str = "maximize";
pub const COMMAND_CENTER: &str = "center";
pub const COMMAND_NEXT_DISPLAY: &str = "next-display";
pub const COMMAND_ALMOST_MAXIMIZE: &str = "almost-maximize";
pub const COMMAND_REASONABLE_SIZE: &str = "reasonable-size";
pub const COMMAND_MAKE_LARGER: &str = "make-larger";
pub const COMMAND_MAKE_SMALLER: &str = "make-smaller";
pub const COMMAND_CENTER_HALF: &str = "center-half";

/// How close two frames must be to count as "the window has not moved since",
/// which is what lets a repeat advance a cycle. A couple of pixels absorbs the
/// border rounding the platforms already produce.
const CYCLE_TOLERANCE: i32 = 2;

/// What a command does to the target window.
#[derive(Clone, Copy)]
enum Arrangement {
    Region(Region),
    Cycle(Cycle),
    Center,
    CenterHalf,
    AlmostMaximize,
    ReasonableSize,
    Step(Step),
    NextDisplay,
}

fn arrangement(command_id: &str) -> Option<Arrangement> {
    Some(match command_id {
        COMMAND_LEFT_HALF => Arrangement::Cycle(Cycle::LeftHalf),
        COMMAND_RIGHT_HALF => Arrangement::Cycle(Cycle::RightHalf),
        COMMAND_TOP_HALF => Arrangement::Cycle(Cycle::TopHalf),
        COMMAND_BOTTOM_HALF => Arrangement::Cycle(Cycle::BottomHalf),
        COMMAND_TOP_LEFT => Arrangement::Region(Region::TopLeftQuarter),
        COMMAND_TOP_RIGHT => Arrangement::Region(Region::TopRightQuarter),
        COMMAND_BOTTOM_LEFT => Arrangement::Region(Region::BottomLeftQuarter),
        COMMAND_BOTTOM_RIGHT => Arrangement::Region(Region::BottomRightQuarter),
        COMMAND_LEFT_THIRD => Arrangement::Region(Region::LeftThird),
        COMMAND_CENTER_THIRD => Arrangement::Region(Region::CenterThird),
        COMMAND_RIGHT_THIRD => Arrangement::Region(Region::RightThird),
        COMMAND_MAXIMIZE => Arrangement::Region(Region::Maximize),
        COMMAND_CENTER => Arrangement::Center,
        COMMAND_CENTER_HALF => Arrangement::CenterHalf,
        COMMAND_ALMOST_MAXIMIZE => Arrangement::AlmostMaximize,
        COMMAND_REASONABLE_SIZE => Arrangement::ReasonableSize,
        COMMAND_MAKE_LARGER => Arrangement::Step(Step::Larger),
        COMMAND_MAKE_SMALLER => Arrangement::Step(Step::Smaller),
        COMMAND_NEXT_DISPLAY => Arrangement::NextDisplay,
        _ => return None,
    })
}

/// What the last cycling command left behind, so a repeat can advance instead of
/// restarting. Shared across every command through the extension.
#[derive(Default)]
struct CycleState {
    last_command: Option<String>,
    last_frame: Option<Rect>,
    step: usize,
}

fn frames_match(a: Rect, b: Rect) -> bool {
    (a.x - b.x).abs() <= CYCLE_TOLERANCE
        && (a.y - b.y).abs() <= CYCLE_TOLERANCE
        && (a.width - b.width).abs() <= CYCLE_TOLERANCE
        && (a.height - b.height).abs() <= CYCLE_TOLERANCE
}

pub struct WindowManagementExtension {
    manifest: Manifest,
    manager: Arc<dyn WindowManager>,
    cycle: Arc<Mutex<CycleState>>,
}

impl WindowManagementExtension {
    pub fn new(manager: Arc<dyn WindowManager>) -> Self {
        Self {
            manifest: manifest(),
            manager,
            cycle: Arc::new(Mutex::new(CycleState::default())),
        }
    }
}

impl Extension for WindowManagementExtension {
    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn command(&self, command_id: &str) -> Option<Arc<dyn Command>> {
        let arrangement = arrangement(command_id)?;
        Some(Arc::new(Arrange {
            manager: self.manager.clone(),
            cycle: self.cycle.clone(),
            command_id: command_id.to_string(),
            arrangement,
        }))
    }
}

/// One arrangement of the target window. Holds the manager, the shared cycle
/// state, and what to do; the geometry decides the rectangle and the manager
/// applies it.
struct Arrange {
    manager: Arc<dyn WindowManager>,
    cycle: Arc<Mutex<CycleState>>,
    command_id: String,
    arrangement: Arrangement,
}

impl Command for Arrange {
    fn invoke(&self, ctx: &InvocationContext) {
        let placement = match self.manager.target() {
            Ok(placement) => placement,
            Err(error) => {
                ctx.fail(error.to_string());
                return;
            }
        };
        let area = placement.work_area;

        let mut cycle = self.cycle.lock().unwrap();
        let repeated = cycle.last_command.as_deref() == Some(self.command_id.as_str())
            && cycle
                .last_frame
                .is_some_and(|last| frames_match(placement.frame, last));

        // `None` means a deliberate no-op that still counts as success, which is
        // only move-to-next-display on a single display.
        let plan: Option<(Rect, usize)> = match self.arrangement {
            Arrangement::Region(region) => Some((region.rect(area), 0)),
            Arrangement::CenterHalf => Some((geometry::center_half(area), 0)),
            Arrangement::AlmostMaximize => Some((geometry::almost_maximize(area), 0)),
            Arrangement::ReasonableSize => Some((geometry::reasonable_size(area), 0)),
            Arrangement::Step(step) => Some((geometry::step(placement.frame, area, step), 0)),
            Arrangement::Cycle(cycle_kind) => {
                let index = if repeated { cycle.step + 1 } else { 0 };
                Some((geometry::cycle_rect(cycle_kind, index, area), index))
            }
            Arrangement::Center => {
                // A fresh centre keeps the window's size, unchanged from M4; only
                // a repeat on the unmoved window enters the size cycle.
                let next = if repeated { cycle.step + 1 } else { 0 };
                let frame = if next == 0 {
                    geometry::center(area, (placement.frame.width, placement.frame.height))
                } else {
                    geometry::cycle_rect(Cycle::Center, next - 1, area)
                };
                Some((frame, next))
            }
            Arrangement::NextDisplay => {
                geometry::next_display(placement.frame, area, &self.manager.displays())
                    .map(|frame| (frame, 0))
            }
        };

        let Some((frame, step)) = plan else {
            ctx.succeed();
            return;
        };

        match self.manager.place(frame) {
            Ok(()) => {
                cycle.last_command = Some(self.command_id.clone());
                cycle.last_frame = Some(frame);
                cycle.step = step;
                ctx.succeed();
            }
            Err(error) => ctx.fail(error.to_string()),
        }
    }
}

fn manifest() -> Manifest {
    Manifest {
        manifest_version: 1,
        id: EXTENSION_ID.into(),
        name: "Window Management".into(),
        icon: None,
        commands: vec![
            command(
                COMMAND_LEFT_HALF,
                "Left Half",
                "panel-left",
                &["left", "half", "tile", "snap"],
            ),
            command(
                COMMAND_RIGHT_HALF,
                "Right Half",
                "panel-right",
                &["right", "half", "tile", "snap"],
            ),
            command(
                COMMAND_TOP_HALF,
                "Top Half",
                "panel-top",
                &["top", "half", "tile", "snap"],
            ),
            command(
                COMMAND_BOTTOM_HALF,
                "Bottom Half",
                "panel-bottom",
                &["bottom", "half", "tile", "snap"],
            ),
            command(
                COMMAND_TOP_LEFT,
                "Top Left Quarter",
                "grid-2x2",
                &["top", "left", "quarter", "corner"],
            ),
            command(
                COMMAND_TOP_RIGHT,
                "Top Right Quarter",
                "grid-2x2",
                &["top", "right", "quarter", "corner"],
            ),
            command(
                COMMAND_BOTTOM_LEFT,
                "Bottom Left Quarter",
                "grid-2x2",
                &["bottom", "left", "quarter", "corner"],
            ),
            command(
                COMMAND_BOTTOM_RIGHT,
                "Bottom Right Quarter",
                "grid-2x2",
                &["bottom", "right", "quarter", "corner"],
            ),
            command(
                COMMAND_LEFT_THIRD,
                "Left Third",
                "columns-3",
                &["left", "third", "column"],
            ),
            command(
                COMMAND_CENTER_THIRD,
                "Center Third",
                "columns-3",
                &["center", "middle", "third", "column"],
            ),
            command(
                COMMAND_RIGHT_THIRD,
                "Right Third",
                "columns-3",
                &["right", "third", "column"],
            ),
            command(
                COMMAND_MAXIMIZE,
                "Maximize",
                "maximize",
                &["maximize", "fill", "full", "fullscreen"],
            ),
            command(
                COMMAND_ALMOST_MAXIMIZE,
                "Almost Maximize",
                "maximize",
                &["almost", "maximize", "large", "margin"],
            ),
            command(
                COMMAND_REASONABLE_SIZE,
                "Reasonable Size",
                "app-window",
                &["reasonable", "default", "comfortable", "size"],
            ),
            command(COMMAND_CENTER, "Center", "square", &["center", "middle"]),
            command(
                COMMAND_CENTER_HALF,
                "Center Half",
                "square",
                &["center", "middle", "half", "column"],
            ),
            command(
                COMMAND_MAKE_LARGER,
                "Make Larger",
                "maximize-2",
                &["larger", "bigger", "grow", "expand"],
            ),
            command(
                COMMAND_MAKE_SMALLER,
                "Make Smaller",
                "minimize-2",
                &["smaller", "shrink", "contract"],
            ),
            command(
                COMMAND_NEXT_DISPLAY,
                "Move to Next Display",
                "monitor",
                &["display", "monitor", "screen", "next", "move"],
            ),
        ],
        preferences: vec![],
        root_items: false,
        services: false,
    }
}

fn command(id: &str, title: &str, icon: &str, keywords: &[&str]) -> crate::extension::CommandDecl {
    crate::extension::CommandDecl {
        id: id.into(),
        title: title.into(),
        mode: InvocationMode::NoView,
        subtitle: Some("Window Management".into()),
        icon: Some(format!("{}{icon}", crate::extension::NAMED_ICON)),
        keywords: keywords.iter().map(|k| (*k).to_string()).collect(),
        alias: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::invocation::{Outcome, Sink};
    use crate::platform::{Placement, Rect, WindowError};
    use std::sync::Mutex;

    struct FakeManager {
        target: Result<Placement, WindowError>,
        displays: Vec<Rect>,
        placed: Mutex<Vec<Rect>>,
        place_result: Result<(), WindowError>,
    }

    impl FakeManager {
        fn holding(frame: Rect, work_area: Rect) -> Arc<Self> {
            Arc::new(Self {
                target: Ok(Placement { frame, work_area }),
                displays: vec![],
                placed: Mutex::new(vec![]),
                place_result: Ok(()),
            })
        }
        fn failing_target() -> Arc<Self> {
            Arc::new(Self {
                target: Err(WindowError::NoTarget),
                displays: vec![],
                placed: Mutex::new(vec![]),
                place_result: Ok(()),
            })
        }
        fn refusing_place(frame: Rect, work_area: Rect) -> Arc<Self> {
            Arc::new(Self {
                target: Ok(Placement { frame, work_area }),
                displays: vec![],
                placed: Mutex::new(vec![]),
                place_result: Err(WindowError::Unreachable("elevated".into())),
            })
        }
        fn placed(&self) -> Vec<Rect> {
            self.placed.lock().unwrap().clone()
        }
    }

    impl WindowManager for FakeManager {
        fn target(&self) -> Result<Placement, WindowError> {
            self.target.clone()
        }
        fn displays(&self) -> Vec<Rect> {
            self.displays.clone()
        }
        fn place(&self, frame: Rect) -> Result<(), WindowError> {
            if self.place_result.is_ok() {
                self.placed.lock().unwrap().push(frame);
            }
            self.place_result.clone()
        }
    }

    #[derive(Default)]
    struct RecordingSink {
        outcome: Mutex<Option<Outcome>>,
    }

    impl Sink for RecordingSink {
        fn push_view(&self, _tree: crate::protocol::ViewTree) {}
        fn finish(&self, outcome: Outcome) {
            *self.outcome.lock().unwrap() = Some(outcome);
        }
    }

    const AREA: Rect = Rect {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    };
    const FRAME: Rect = Rect {
        x: 100,
        y: 100,
        width: 800,
        height: 600,
    };

    #[test]
    fn manifest_validates() {
        let extension = WindowManagementExtension::new(FakeManager::holding(FRAME, AREA));
        assert!(extension.manifest().validate().is_ok());
    }

    #[test]
    fn every_declared_command_is_invocable() {
        let extension = WindowManagementExtension::new(FakeManager::holding(FRAME, AREA));
        for decl in &extension.manifest().commands {
            assert!(
                extension.command(&decl.id).is_some(),
                "no implementation for {}",
                decl.id
            );
        }
    }

    #[test]
    fn left_half_places_the_left_half() {
        let manager = FakeManager::holding(FRAME, AREA);
        let extension = WindowManagementExtension::new(manager.clone());
        let command = extension.command(COMMAND_LEFT_HALF).unwrap();
        let sink = Arc::new(RecordingSink::default());
        command.invoke(&InvocationContext::for_test(
            COMMAND_LEFT_HALF,
            sink.clone(),
        ));
        assert_eq!(manager.placed(), vec![Region::LeftHalf.rect(AREA)]);
        assert_eq!(sink.outcome.lock().unwrap().clone(), Some(Outcome::Success));
    }

    #[test]
    fn center_keeps_the_frame_size() {
        let manager = FakeManager::holding(FRAME, AREA);
        let extension = WindowManagementExtension::new(manager.clone());
        extension
            .command(COMMAND_CENTER)
            .unwrap()
            .invoke(&InvocationContext::for_test(
                COMMAND_CENTER,
                Arc::new(RecordingSink::default()),
            ));
        let placed = manager.placed();
        assert_eq!(placed.len(), 1);
        assert_eq!(
            (placed[0].width, placed[0].height),
            (FRAME.width, FRAME.height)
        );
    }

    #[test]
    fn next_display_with_one_display_is_a_no_op_success() {
        let manager = FakeManager::holding(FRAME, AREA);
        let extension = WindowManagementExtension::new(manager.clone());
        let sink = Arc::new(RecordingSink::default());
        extension
            .command(COMMAND_NEXT_DISPLAY)
            .unwrap()
            .invoke(&InvocationContext::for_test(
                COMMAND_NEXT_DISPLAY,
                sink.clone(),
            ));
        assert!(manager.placed().is_empty());
        assert_eq!(sink.outcome.lock().unwrap().clone(), Some(Outcome::Success));
    }

    #[test]
    fn no_target_fails_without_placing() {
        let manager = FakeManager::failing_target();
        let extension = WindowManagementExtension::new(manager.clone());
        let sink = Arc::new(RecordingSink::default());
        extension
            .command(COMMAND_MAXIMIZE)
            .unwrap()
            .invoke(&InvocationContext::for_test(COMMAND_MAXIMIZE, sink.clone()));
        assert!(manager.placed().is_empty());
        assert!(matches!(
            sink.outcome.lock().unwrap().clone(),
            Some(Outcome::Failure(_))
        ));
    }

    #[test]
    fn a_refused_place_is_reported() {
        let manager = FakeManager::refusing_place(FRAME, AREA);
        let extension = WindowManagementExtension::new(manager.clone());
        let sink = Arc::new(RecordingSink::default());
        extension
            .command(COMMAND_LEFT_HALF)
            .unwrap()
            .invoke(&InvocationContext::for_test(
                COMMAND_LEFT_HALF,
                sink.clone(),
            ));
        assert!(matches!(
            sink.outcome.lock().unwrap().clone(),
            Some(Outcome::Failure(_))
        ));
    }

    /// A window that stays exactly where it is placed, so a repeat is seen as
    /// unmoved and advances the cycle.
    struct WindowSim {
        frame: Mutex<Rect>,
        area: Rect,
        placed: Mutex<Vec<Rect>>,
    }

    impl WindowSim {
        fn new(frame: Rect, area: Rect) -> Arc<Self> {
            Arc::new(Self {
                frame: Mutex::new(frame),
                area,
                placed: Mutex::new(vec![]),
            })
        }
        fn placed(&self) -> Vec<Rect> {
            self.placed.lock().unwrap().clone()
        }
        fn set_frame(&self, frame: Rect) {
            *self.frame.lock().unwrap() = frame;
        }
    }

    impl WindowManager for WindowSim {
        fn target(&self) -> Result<Placement, WindowError> {
            Ok(Placement {
                frame: *self.frame.lock().unwrap(),
                work_area: self.area,
            })
        }
        fn displays(&self) -> Vec<Rect> {
            vec![self.area]
        }
        fn place(&self, frame: Rect) -> Result<(), WindowError> {
            *self.frame.lock().unwrap() = frame;
            self.placed.lock().unwrap().push(frame);
            Ok(())
        }
    }

    fn invoke(extension: &WindowManagementExtension, command_id: &str) {
        extension
            .command(command_id)
            .expect("command exists")
            .invoke(&InvocationContext::for_test(
                command_id,
                Arc::new(RecordingSink::default()),
            ));
    }

    #[test]
    fn repeating_left_half_cycles_the_fractions() {
        let sim = WindowSim::new(FRAME, AREA);
        let extension = WindowManagementExtension::new(sim.clone());
        for _ in 0..4 {
            invoke(&extension, COMMAND_LEFT_HALF);
        }
        assert_eq!(
            sim.placed(),
            vec![
                geometry::cycle_rect(Cycle::LeftHalf, 0, AREA),
                geometry::cycle_rect(Cycle::LeftHalf, 1, AREA),
                geometry::cycle_rect(Cycle::LeftHalf, 2, AREA),
                geometry::cycle_rect(Cycle::LeftHalf, 0, AREA),
            ]
        );
    }

    #[test]
    fn a_different_command_resets_the_cycle() {
        let sim = WindowSim::new(FRAME, AREA);
        let extension = WindowManagementExtension::new(sim.clone());
        invoke(&extension, COMMAND_LEFT_HALF);
        invoke(&extension, COMMAND_RIGHT_HALF);
        invoke(&extension, COMMAND_RIGHT_HALF);
        let placed = sim.placed();
        assert_eq!(placed[1], geometry::cycle_rect(Cycle::RightHalf, 0, AREA));
        assert_eq!(placed[2], geometry::cycle_rect(Cycle::RightHalf, 1, AREA));
    }

    #[test]
    fn a_moved_window_resets_the_cycle() {
        let sim = WindowSim::new(FRAME, AREA);
        let extension = WindowManagementExtension::new(sim.clone());
        invoke(&extension, COMMAND_LEFT_HALF);
        // The user drags the window somewhere else.
        sim.set_frame(Rect {
            x: 500,
            y: 500,
            width: 300,
            height: 300,
        });
        invoke(&extension, COMMAND_LEFT_HALF);
        let placed = sim.placed();
        assert_eq!(placed[1], geometry::cycle_rect(Cycle::LeftHalf, 0, AREA));
    }

    #[test]
    fn center_keeps_size_then_cycles_on_repeat() {
        let start = Rect {
            x: 200,
            y: 150,
            width: 800,
            height: 600,
        };
        let sim = WindowSim::new(start, AREA);
        let extension = WindowManagementExtension::new(sim.clone());
        invoke(&extension, COMMAND_CENTER);
        invoke(&extension, COMMAND_CENTER);
        invoke(&extension, COMMAND_CENTER);
        let placed = sim.placed();
        // Fresh centre keeps the 800x600 size.
        assert_eq!((placed[0].width, placed[0].height), (800, 600));
        // Then it cycles: 1/2, then 2/3.
        assert_eq!(placed[1], geometry::cycle_rect(Cycle::Center, 0, AREA));
        assert_eq!(placed[2], geometry::cycle_rect(Cycle::Center, 1, AREA));
    }

    #[test]
    fn make_larger_accumulates_to_the_work_area() {
        let sim = WindowSim::new(
            Rect {
                x: 800,
                y: 400,
                width: 200,
                height: 200,
            },
            AREA,
        );
        let extension = WindowManagementExtension::new(sim.clone());
        for _ in 0..40 {
            invoke(&extension, COMMAND_MAKE_LARGER);
        }
        let frame = *sim.frame.lock().unwrap();
        assert_eq!((frame.width, frame.height), (AREA.width, AREA.height));
    }

    #[test]
    fn make_smaller_stops_at_the_minimum() {
        let sim = WindowSim::new(AREA, AREA);
        let extension = WindowManagementExtension::new(sim.clone());
        for _ in 0..40 {
            invoke(&extension, COMMAND_MAKE_SMALLER);
        }
        let frame = *sim.frame.lock().unwrap();
        assert_eq!((frame.width, frame.height), (400, 300));
    }

    #[test]
    fn the_new_commands_are_declared_and_invocable() {
        let extension = WindowManagementExtension::new(FakeManager::holding(FRAME, AREA));
        for id in [
            COMMAND_ALMOST_MAXIMIZE,
            COMMAND_REASONABLE_SIZE,
            COMMAND_MAKE_LARGER,
            COMMAND_MAKE_SMALLER,
            COMMAND_CENTER_HALF,
        ] {
            assert!(
                extension.manifest().commands.iter().any(|c| c.id == id),
                "{id} not declared"
            );
            assert!(extension.command(id).is_some(), "{id} not invocable");
        }
    }
}
