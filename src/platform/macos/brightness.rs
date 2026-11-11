use objc2_core_graphics::{CGDirectDisplayID, CGEvent, CGGetDisplaysWithPoint};

use crate::platform::brightness::DdcMonitor;
use crate::platform::{BrightnessMonitors, Monitor};

pub struct MacBrightness;

impl BrightnessMonitors for MacBrightness {
    fn external(&self) -> Vec<Box<dyn Monitor>> {
        enumerate(|_| true)
    }

    fn under_pointer(&self) -> Vec<Box<dyn Monitor>> {
        let Some(id) = display_under_pointer() else {
            return Vec::new();
        };
        enumerate(|monitor| monitor.handle().id == id)
    }
}

fn enumerate(keep: impl Fn(&ddc_macos::Monitor) -> bool) -> Vec<Box<dyn Monitor>> {
    match ddc_macos::Monitor::enumerate() {
        Ok(monitors) => monitors
            .into_iter()
            .filter(keep)
            .map(|monitor| {
                Box::new(DdcMonitor::new(monitor.description(), monitor)) as Box<dyn Monitor>
            })
            .collect(),
        Err(error) => {
            crate::log::append(&format!("brightness: could not list monitors: {error}"));
            Vec::new()
        }
    }
}

/// CoreGraphics, not `NSScreen`: a brightness step runs off the main thread.
fn display_under_pointer() -> Option<CGDirectDisplayID> {
    let event = CGEvent::new(None);
    let point = CGEvent::location(event.as_deref());
    let mut id: CGDirectDisplayID = 0;
    let mut count = 0u32;
    // SAFETY: room for exactly the one display asked for.
    unsafe { CGGetDisplaysWithPoint(point, 1, &mut id, &mut count) };
    (count > 0).then_some(id)
}
