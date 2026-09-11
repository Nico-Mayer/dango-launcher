use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;

/// Measures hotkey-to-paint against a single monotonic clock on the Rust side,
/// so the number is not distorted by clock skew between Rust and the webview.
#[derive(Default)]
pub struct LatencyProbe {
    next_id: AtomicU64,
    pending: Mutex<HashMap<u64, Instant>>,
}

impl LatencyProbe {
    fn enabled() -> bool {
        cfg!(debug_assertions) || std::env::var_os("DANGO_MEASURE").is_some()
    }

    pub fn start(&self) -> Option<u64> {
        if !Self::enabled() {
            return None;
        }
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.pending.lock().unwrap().insert(id, Instant::now());
        eprintln!("[dango] activation {id} started");
        Some(id)
    }

    pub fn finish(&self, id: u64) {
        let Some(started) = self.pending.lock().unwrap().remove(&id) else {
            return;
        };
        eprintln!(
            "[dango] activation {id} painted in {:.1}ms",
            started.elapsed().as_secs_f64() * 1000.0
        );
    }
}
