use crate::platform::brightness::DdcMonitor;
use crate::platform::{BrightnessMonitors, Monitor};

pub struct MacBrightness;

impl BrightnessMonitors for MacBrightness {
    fn external(&self) -> Vec<Box<dyn Monitor>> {
        match ddc_macos::Monitor::enumerate() {
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
