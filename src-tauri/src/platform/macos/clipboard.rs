//! Which application did the copying.
//!
//! The clipboard itself is `clipboard-rs`. This is the one question it cannot
//! answer, and the spike showed why it needs answering carefully: Proton Pass
//! sets no privacy marker, and asking which application is frontmost at the
//! moment a change arrives blamed a password on the terminal the user had
//! already switched back to. So activations are recorded as they happen, and a
//! change is attributed to everything that was frontmost around it.

use std::ptr::NonNull;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use block2::RcBlock;
use objc2_app_kit::{NSWorkspace, NSWorkspaceDidActivateApplicationNotification};
use objc2_foundation::NSNotification;

use crate::extensions::clipboard::Attribution;

/// How far back an activation still counts as a candidate for a change arriving
/// now. Generous, because the switch away can precede the change being
/// delivered and the cost of being wrong in this direction is only a lost
/// entry.
const TRAIL_WINDOW: Duration = Duration::from_millis(1_500);

/// How many activations to remember. A handful covers any plausible flurry of
/// switching.
const TRAIL_LENGTH: usize = 8;

/// Shared with the activation observer, which outlives any particular caller.
type Trail = Arc<Mutex<Vec<(String, Instant)>>>;

pub struct MacAttribution {
    trail: Trail,
}

impl Default for MacAttribution {
    fn default() -> Self {
        Self::new()
    }
}

impl MacAttribution {
    pub fn new() -> Self {
        Self {
            trail: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Starts recording activations. Separate from construction because it
    /// needs the application's run loop, which is running by the time an
    /// extension activates but not necessarily when it is built.
    ///
    /// Must be called from the main thread: workspace notifications arrive only
    /// while a run loop is pumping, and a trail that quietly stops being fed
    /// stops excluding anything.
    pub fn watch_activations(&self) {
        let trail = self.trail.clone();
        let handler = RcBlock::new(move |_: NonNull<NSNotification>| {
            if let Some(name) = frontmost_name() {
                remember(&trail, name);
            }
        });
        let observer = unsafe {
            NSWorkspace::sharedWorkspace()
                .notificationCenter()
                .addObserverForName_object_queue_usingBlock(
                    Some(NSWorkspaceDidActivateApplicationNotification),
                    None,
                    None,
                    &handler,
                )
        };
        // Deliberately leaked: dropping the observer unregisters it, and one
        // object for the life of the process is the cheaper mistake.
        std::mem::forget(observer);
    }
}

impl Attribution for MacAttribution {
    fn candidate_applications(&self) -> Vec<String> {
        let mut candidates: Vec<String> = self
            .trail
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, at)| at.elapsed() <= TRAIL_WINDOW)
            .map(|(name, _)| name.clone())
            .collect();
        // Whoever is frontmost now is always a candidate, which is what makes
        // this work before any activation has been seen.
        if let Some(name) = frontmost_name() {
            if !candidates.contains(&name) {
                candidates.push(name);
            }
        }
        candidates
    }
}

fn remember(trail: &Mutex<Vec<(String, Instant)>>, name: String) {
    let mut trail = trail.lock().unwrap();
    if trail.last().is_some_and(|(last, _)| *last == name) {
        return;
    }
    trail.push((name, Instant::now()));
    let excess = trail.len().saturating_sub(TRAIL_LENGTH);
    trail.drain(..excess);
}

fn frontmost_name() -> Option<String> {
    NSWorkspace::sharedWorkspace()
        .frontmostApplication()
        .and_then(|app| app.localizedName())
        .map(|name| name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_trail_keeps_the_most_recent_activations() {
        let trail: Trail = Arc::new(Mutex::new(Vec::new()));
        for index in 0..TRAIL_LENGTH + 4 {
            remember(&trail, format!("app {index}"));
        }
        let kept = trail.lock().unwrap();
        assert_eq!(kept.len(), TRAIL_LENGTH);
        assert_eq!(kept.last().unwrap().0, format!("app {}", TRAIL_LENGTH + 3));
    }

    #[test]
    fn reactivating_the_same_application_does_not_fill_the_trail() {
        let trail: Trail = Arc::new(Mutex::new(Vec::new()));
        remember(&trail, "Ghostty".into());
        remember(&trail, "Ghostty".into());
        remember(&trail, "Ghostty".into());
        assert_eq!(trail.lock().unwrap().len(), 1);
    }

    /// Needs a desktop session: run with `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn whoever_is_frontmost_is_always_a_candidate() {
        assert!(!MacAttribution::new().candidate_applications().is_empty());
    }
}
