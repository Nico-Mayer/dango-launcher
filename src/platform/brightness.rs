use std::fmt::Display;

use super::{Level, Monitor, MonitorError};

const VCP_BRIGHTNESS: u8 = 0x10;

/// One adapter for both platforms' DDC crates, which share the `ddc` traits.
/// Waits out the pause DDC/CI asks for after every command, while the caller
/// still holds the bus: `ddc-macos` records the pause but never waits for it,
/// and the next press builds a fresh monitor that knows nothing of it.
pub(super) struct DdcMonitor<M> {
    name: String,
    inner: M,
}

impl<M> DdcMonitor<M> {
    pub(super) fn new(name: String, inner: M) -> Self {
        Self { name, inner }
    }
}

impl<M> Monitor for DdcMonitor<M>
where
    M: ddc::Ddc,
    M::Error: Display,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn brightness(&mut self) -> Result<Level, MonitorError> {
        let value = self.inner.get_vcp_feature(VCP_BRIGHTNESS);
        self.inner.sleep();
        let value = value.map_err(|error| MonitorError(error.to_string()))?;
        Ok(Level {
            value: value.value(),
            maximum: value.maximum(),
        })
    }

    fn set_brightness(&mut self, value: u16) -> Result<(), MonitorError> {
        let result = self.inner.set_vcp_feature(VCP_BRIGHTNESS, value);
        self.inner.sleep();
        result.map_err(|error| MonitorError(error.to_string()))
    }
}
