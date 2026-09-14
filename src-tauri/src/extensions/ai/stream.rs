//! Turning a running request into what the user sees.
//!
//! The protocol replaces the whole tree, so a fragment per tree would mean a
//! hundred trees for one answer. Fragments are buffered instead and the view is
//! replaced at most once every 50ms, which is well inside what the frontend
//! renders without flicker and keeps a fast local model from flooding the
//! channel.

use std::time::{Duration, Instant};

use super::provider::{AiError, Chunks, Next};

/// Nothing at all within this long means the model is not going to answer.
/// Only the wait for the first fragment is bounded; a long answer is allowed to
/// take as long as it takes.
pub const FIRST_FRAGMENT_TIMEOUT: Duration = Duration::from_secs(30);

/// The floor between two view replacements.
pub const COALESCE: Duration = Duration::from_millis(50);

/// How long a single poll waits before the loop looks at cancellation again.
const POLL: Duration = Duration::from_millis(20);

#[derive(Debug, PartialEq, Eq)]
pub enum Ended {
    Answer(String),
    Cancelled,
    Failed(AiError),
}

/// Reads a request to its end, calling `on_update` with everything received so
/// far, no more often than `COALESCE`. The caller shows the final answer itself,
/// so the last fragment never needs a wait of its own.
pub fn drive(
    chunks: &Chunks,
    first_fragment_timeout: Duration,
    cancelled: &dyn Fn() -> bool,
    on_update: &mut dyn FnMut(&str),
) -> Ended {
    let started = Instant::now();
    let mut answer = String::new();
    let mut last_update: Option<Instant> = None;

    loop {
        if cancelled() {
            return Ended::Cancelled;
        }
        match chunks.next(POLL) {
            Next::Text(fragment) => {
                answer.push_str(&fragment);
                let due = last_update.is_none_or(|at| at.elapsed() >= COALESCE);
                if due {
                    on_update(&answer);
                    last_update = Some(Instant::now());
                }
            }
            Next::Idle => {
                if answer.is_empty() && started.elapsed() >= first_fragment_timeout {
                    return Ended::Failed(AiError::Timeout);
                }
            }
            Next::Done => return Ended::Answer(answer),
            Next::Failed(error) => return Ended::Failed(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::provider::{Completions, ProviderKind, Request, Thinking};
    use super::*;

    fn request() -> Request {
        Request {
            provider: "local".into(),
            kind: ProviderKind::Ollama,
            base_url: None,
            model: "qwen3:8b".into(),
            thinking: Thinking::Off,
            prompt: "Improve this.".into(),
            key: None,
            cli: None,
        }
    }

    fn never_cancelled() -> impl Fn() -> bool {
        || false
    }

    #[test]
    fn many_fragments_become_far_fewer_view_updates() {
        let fragments: Vec<String> = (0..30).map(|n| format!("{n} ")).collect();
        let borrowed: Vec<&str> = fragments.iter().map(String::as_str).collect();
        let client = super::super::provider::testing::Scripted::answering(&borrowed);
        let chunks = client.stream(request());

        let mut updates = Vec::new();
        let ended = drive(
            &chunks,
            FIRST_FRAGMENT_TIMEOUT,
            &never_cancelled(),
            &mut |so_far| updates.push(so_far.to_string()),
        );

        let expected: String = fragments.concat();
        assert_eq!(ended, Ended::Answer(expected.clone()));
        assert!(
            updates.len() < 30,
            "one tree per fragment: {} updates",
            updates.len()
        );
        assert!(!updates.is_empty(), "nothing was shown while streaming");
        assert!(
            expected.starts_with(updates.last().unwrap()),
            "an update showed text the answer does not contain"
        );
    }

    #[test]
    fn the_first_text_is_shown_without_waiting() {
        let client = super::super::provider::testing::Scripted::answering(&["Hello"]);
        let chunks = client.stream(request());
        let mut updates = Vec::new();
        drive(
            &chunks,
            FIRST_FRAGMENT_TIMEOUT,
            &never_cancelled(),
            &mut |so_far| updates.push(so_far.to_string()),
        );
        assert_eq!(updates.first().map(String::as_str), Some("Hello"));
    }

    #[test]
    fn a_model_that_says_nothing_times_out_with_a_message() {
        // The sender is held, so the request is open but silent.
        let (_sender, receiver) = std::sync::mpsc::channel();
        let chunks = Chunks::new(receiver);

        let ended = drive(
            &chunks,
            Duration::from_millis(60),
            &never_cancelled(),
            &mut |_| {},
        );

        let Ended::Failed(error) = ended else {
            panic!("expected a timeout");
        };
        assert_eq!(error.to_string(), "The model didn't answer in time. Try again.");
    }

    #[test]
    fn a_cancelled_request_stops_being_read() {
        let client = super::super::provider::testing::Scripted::answering(&["one", "two"]);
        let chunks = client.stream(request());
        let mut updates = Vec::new();
        let ended = drive(&chunks, FIRST_FRAGMENT_TIMEOUT, &|| true, &mut |so_far| {
            updates.push(so_far.to_string())
        });
        assert_eq!(ended, Ended::Cancelled);
        assert!(updates.is_empty(), "a cancelled request still drew");
    }

    #[test]
    fn a_failure_ends_the_drive() {
        let client = super::super::provider::testing::Scripted::failing(AiError::Timeout);
        let chunks = client.stream(request());
        let ended = drive(
            &chunks,
            FIRST_FRAGMENT_TIMEOUT,
            &never_cancelled(),
            &mut |_| {},
        );
        assert_eq!(ended, Ended::Failed(AiError::Timeout));
    }
}
