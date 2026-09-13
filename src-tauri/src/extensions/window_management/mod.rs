//! The `window-management` built-in: tiling, maximising, centring, and moving
//! the focused window across displays. Its whole contribution is no-view
//! commands, so it is `system` without the confirmations: each command computes
//! a target rectangle from shared geometry and applies it through the platform
//! `WindowManager`.

use std::sync::Arc;

use crate::extension::{Extension, InvocationMode, Manifest};
use crate::invocation::{Command, InvocationContext};
use crate::platform::WindowManager;

pub mod geometry;

use geometry::Region;

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

/// What a command does to the target window.
#[derive(Clone, Copy)]
enum Arrangement {
    Region(Region),
    Center,
    NextDisplay,
}

fn arrangement(command_id: &str) -> Option<Arrangement> {
    let region = |r| Some(Arrangement::Region(r));
    match command_id {
        COMMAND_LEFT_HALF => region(Region::LeftHalf),
        COMMAND_RIGHT_HALF => region(Region::RightHalf),
        COMMAND_TOP_HALF => region(Region::TopHalf),
        COMMAND_BOTTOM_HALF => region(Region::BottomHalf),
        COMMAND_TOP_LEFT => region(Region::TopLeftQuarter),
        COMMAND_TOP_RIGHT => region(Region::TopRightQuarter),
        COMMAND_BOTTOM_LEFT => region(Region::BottomLeftQuarter),
        COMMAND_BOTTOM_RIGHT => region(Region::BottomRightQuarter),
        COMMAND_LEFT_THIRD => region(Region::LeftThird),
        COMMAND_CENTER_THIRD => region(Region::CenterThird),
        COMMAND_RIGHT_THIRD => region(Region::RightThird),
        COMMAND_MAXIMIZE => region(Region::Maximize),
        COMMAND_CENTER => Some(Arrangement::Center),
        COMMAND_NEXT_DISPLAY => Some(Arrangement::NextDisplay),
        _ => None,
    }
}

pub struct WindowManagementExtension {
    manifest: Manifest,
    manager: Arc<dyn WindowManager>,
}

impl WindowManagementExtension {
    pub fn new(manager: Arc<dyn WindowManager>) -> Self {
        Self {
            manifest: manifest(),
            manager,
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
            arrangement,
        }))
    }
}

/// One arrangement of the target window. Holds the manager and what to do; the
/// geometry decides the rectangle and the manager applies it.
struct Arrange {
    manager: Arc<dyn WindowManager>,
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

        let frame = match self.arrangement {
            Arrangement::Region(region) => region.rect(placement.work_area),
            Arrangement::Center => geometry::center(
                placement.work_area,
                (placement.frame.width, placement.frame.height),
            ),
            Arrangement::NextDisplay => {
                match geometry::next_display(
                    placement.frame,
                    placement.work_area,
                    &self.manager.displays(),
                ) {
                    Some(frame) => frame,
                    // One display is not a failure; there is simply nowhere to
                    // send the window, so the command is a no-op.
                    None => {
                        ctx.succeed();
                        return;
                    }
                }
            }
        };

        match self.manager.place(frame) {
            Ok(()) => ctx.succeed(),
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
            command(COMMAND_LEFT_HALF, "Left Half", "panel-left", &["left", "half", "tile", "snap"]),
            command(COMMAND_RIGHT_HALF, "Right Half", "panel-right", &["right", "half", "tile", "snap"]),
            command(COMMAND_TOP_HALF, "Top Half", "panel-top", &["top", "half", "tile", "snap"]),
            command(COMMAND_BOTTOM_HALF, "Bottom Half", "panel-bottom", &["bottom", "half", "tile", "snap"]),
            command(COMMAND_TOP_LEFT, "Top Left Quarter", "grid-2x2", &["top", "left", "quarter", "corner"]),
            command(COMMAND_TOP_RIGHT, "Top Right Quarter", "grid-2x2", &["top", "right", "quarter", "corner"]),
            command(COMMAND_BOTTOM_LEFT, "Bottom Left Quarter", "grid-2x2", &["bottom", "left", "quarter", "corner"]),
            command(COMMAND_BOTTOM_RIGHT, "Bottom Right Quarter", "grid-2x2", &["bottom", "right", "quarter", "corner"]),
            command(COMMAND_LEFT_THIRD, "Left Third", "columns-3", &["left", "third", "column"]),
            command(COMMAND_CENTER_THIRD, "Center Third", "columns-3", &["center", "middle", "third", "column"]),
            command(COMMAND_RIGHT_THIRD, "Right Third", "columns-3", &["right", "third", "column"]),
            command(COMMAND_MAXIMIZE, "Maximize", "maximize", &["maximize", "fill", "full", "fullscreen"]),
            command(COMMAND_CENTER, "Center", "square", &["center", "middle"]),
            command(COMMAND_NEXT_DISPLAY, "Move to Next Display", "monitor", &["display", "monitor", "screen", "next", "move"]),
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

    const AREA: Rect = Rect { x: 0, y: 0, width: 1920, height: 1080 };
    const FRAME: Rect = Rect { x: 100, y: 100, width: 800, height: 600 };

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
        command.invoke(&InvocationContext::for_test(COMMAND_LEFT_HALF, sink.clone()));
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
        assert_eq!((placed[0].width, placed[0].height), (FRAME.width, FRAME.height));
    }

    #[test]
    fn next_display_with_one_display_is_a_no_op_success() {
        let manager = FakeManager::holding(FRAME, AREA);
        let extension = WindowManagementExtension::new(manager.clone());
        let sink = Arc::new(RecordingSink::default());
        extension
            .command(COMMAND_NEXT_DISPLAY)
            .unwrap()
            .invoke(&InvocationContext::for_test(COMMAND_NEXT_DISPLAY, sink.clone()));
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
            .invoke(&InvocationContext::for_test(COMMAND_LEFT_HALF, sink.clone()));
        assert!(matches!(
            sink.outcome.lock().unwrap().clone(),
            Some(Outcome::Failure(_))
        ));
    }
}
