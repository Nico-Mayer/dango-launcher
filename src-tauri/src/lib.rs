pub mod app;
mod autostart;
pub mod config;
pub mod extension;
pub mod extensions;
pub mod hotkeys;
pub mod invocation;
mod latency;
pub mod log;
mod opener;
pub mod platform;
pub mod protocol;
pub mod ranking;
pub mod search;
pub mod shell;
mod slint_shell;
pub mod store;
pub mod templates;
pub mod text;
pub mod ui;

use std::sync::Arc;

/// The platform's answer to which application did the copying, for spikes
/// that need to exercise the real one outside the application.
pub fn platform_attribution() -> Arc<dyn extensions::clipboard::Attribution> {
    platform::attribution()
}

pub fn run() {
    slint_shell::run()
}
