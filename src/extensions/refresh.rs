//! The wait between two reads of something installed on disk, shared by the
//! extensions that keep such a list fresh in the background.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// The longest a list may go without a read, for platforms that cannot
/// signal a change and as a backstop for those that can.
const REFRESH_INTERVAL: Duration = Duration::from_secs(180);

/// How often the loop wakes to check whether it should stop or read again.
const TICK: Duration = Duration::from_millis(200);

/// An install writes many files; waiting a moment after the first sign of
/// change avoids reading a half-written one.
const SETTLE: Duration = Duration::from_secs(1);

/// Waits until `fingerprint` changes or the sweep is due, whichever comes
/// first. A `None` fingerprint waits for the sweep. Sleeps in short slices so
/// disabling the extension stops the loop promptly.
pub fn wait_for_change(fingerprint: impl Fn() -> Option<u64>, running: &AtomicBool) {
    let baseline = fingerprint();
    let started = Instant::now();
    while running.load(Ordering::SeqCst) && started.elapsed() < REFRESH_INTERVAL {
        std::thread::sleep(TICK);
        if baseline.is_some() && fingerprint() != baseline {
            std::thread::sleep(SETTLE);
            return;
        }
    }
}
