use std::fmt::Display;

use super::{Level, Monitor, MonitorError};

const VCP_BRIGHTNESS: u8 = 0x10;

/// One adapter for both platforms' DDC crates, which share the `ddc` traits.
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
        let value = self
            .inner
            .get_vcp_feature(VCP_BRIGHTNESS)
            .map_err(|error| MonitorError(error.to_string()))?;
        Ok(Level {
            value: value.value(),
            maximum: value.maximum(),
        })
    }

    fn set_brightness(&mut self, value: u16) -> Result<(), MonitorError> {
        self.inner
            .set_vcp_feature(VCP_BRIGHTNESS, value)
            .map_err(|error| MonitorError(error.to_string()))
    }
}
