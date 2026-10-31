//! The one log file, beside the config: `dango.log` in the config directory.
//!
//! Where detail goes that the user must not be shown. A failure on screen says
//! what failed and what to do; the raw cause from a library or the operating
//! system lands here instead.

use std::io::Write;

use rolling_file::{BasicRollingFileAppender, RollingConditionBasic};

/// Past this size the log moves to `dango.log.1`, replacing the one before.
const MAX_BYTES: u64 = 1024 * 1024;

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
    let condition = RollingConditionBasic::new().max_size(MAX_BYTES);
    if let Ok(mut file) = BasicRollingFileAppender::new(&path, condition, 1) {
        // One write per line, so a rollover never splits it across files.
        let _ = file.write_all(format!("{message}\n").as_bytes());
        let _ = file.flush();
    }
}
