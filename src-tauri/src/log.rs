//! The one log file, beside the config: `dango.log` in the config directory.
//!
//! Where detail goes that the user must not be shown. A failure on screen says
//! what failed and what to do; the raw cause from a library or the operating
//! system lands here instead.

pub fn append(message: &str) {
    // A test run shares the developer's config directory, and a test asserting
    // on a failure message should not leave a line in the log they read.
    if cfg!(test) {
        return;
    }
    let path = crate::config::config_dir().join("dango.log");
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        use std::io::Write;
        let _ = writeln!(file, "{message}");
    }
}
