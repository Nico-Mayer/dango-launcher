//! Exercises the selection and paste path against a real application, driving
//! `TextExchange` directly rather than the launcher's interface. Each platform
//! has its own harness, because what a target application is and how it is
//! read back differ entirely.
//!
//! Usage: `cargo run --example walkthrough`

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

fn main() {
    #[cfg(target_os = "macos")]
    macos::run();
    #[cfg(target_os = "windows")]
    windows::run();
}
