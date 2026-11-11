use windows_sys::Win32::Foundation::POINT;
use windows_sys::Win32::Graphics::Gdi::{MonitorFromPoint, MONITOR_DEFAULTTONULL};
use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

use crate::platform::brightness::DdcMonitor;
use crate::platform::{BrightnessMonitors, Monitor};

pub struct WindowsBrightness;

impl BrightnessMonitors for WindowsBrightness {
    fn external(&self) -> Vec<Box<dyn Monitor>> {
        match ddc_winapi::Monitor::enumerate() {
            Ok(monitors) => monitors.into_iter().map(boxed).collect(),
            Err(error) => {
                crate::log::append(&format!("brightness: could not list monitors: {error}"));
                Vec::new()
            }
        }
    }

    fn under_pointer(&self) -> Vec<Box<dyn Monitor>> {
        let mut point = POINT { x: 0, y: 0 };
        // SAFETY: plain Win32 calls; `point` outlives the call that fills it.
        let hmonitor = unsafe {
            if GetCursorPos(&mut point) == 0 {
                return Vec::new();
            }
            MonitorFromPoint(point, MONITOR_DEFAULTTONULL)
        };
        if hmonitor.is_null() {
            return Vec::new();
        }
        // `ddc-winapi` takes `winapi`'s `HMONITOR`, the same handle under another type.
        match ddc_winapi::get_physical_monitors_from_hmonitor(hmonitor.cast()) {
            Ok(physical) => physical
                .into_iter()
                // SAFETY: each handle was just returned by the OS and goes into
                // exactly one `Monitor`, whose `Drop` destroys it.
                .map(|monitor| boxed(unsafe { ddc_winapi::Monitor::new(monitor) }))
                .collect(),
            Err(error) => {
                crate::log::append(&format!(
                    "brightness: could not open the monitor under the pointer: {error}"
                ));
                Vec::new()
            }
        }
    }
}

fn boxed(monitor: ddc_winapi::Monitor) -> Box<dyn Monitor> {
    Box::new(DdcMonitor::new(monitor.description(), monitor))
}
