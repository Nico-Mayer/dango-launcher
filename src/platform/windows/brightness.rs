use crate::platform::brightness::DdcMonitor;
use crate::platform::{BrightnessMonitors, Monitor};

pub struct WindowsBrightness;

impl BrightnessMonitors for WindowsBrightness {
    fn external(&self) -> Vec<Box<dyn Monitor>> {
        match ddc_winapi::Monitor::enumerate() {
            Ok(monitors) => monitors
                .into_iter()
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
}
