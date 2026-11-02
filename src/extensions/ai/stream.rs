//! Turning a running request into what the user sees.
//!
//! The protocol replaces the whole tree, so a fragment per tree would mean a
//! hundred trees for one answer. Fragments are buffered instead and the view is
//! replaced at most once every 50ms, which is well inside what the frontend
//! renders without flicker and keeps a fast local model from flooding the
//! channel.

use std::time::{Duration, Instant};

use super::provider::{AiError, Chunks, Next, Request};

/// Nothing at all within this long means the model is not going to answer.
/// Only the wait for the first fragment is bounded; a long answer is allowed to
/// take as long as it takes.
pub const FIRST_FRAGMENT_TIMEOUT: Duration = Duration::from_secs(30);

/// An agent program prints its answer at the end, after seconds of warm-up and
/// thinking, so it gets longer, and the clock restarts on every output.
pub const AGENT_QUIET_LIMIT: Duration = Duration::from_secs(120);

/// When a running request counts as not answering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Limit {
    /// Nothing at all arrived within this long.
    FirstOutput(Duration),
    /// This long passed since the start or the last fragment.
    Quiet(Duration),
}

pub fn limit_for(request: &Request) -> Limit {
    match request.kind.is_agent() {
        true => Limit::Quiet(AGENT_QUIET_LIMIT),
        false => Limit::FirstOutput(FIRST_FRAGMENT_TIMEOUT),
    }
}

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
    limit: Limit,
    cancelled: &dyn Fn() -> bool,
    on_update: &mut dyn FnMut(&str),
) -> Ended {
    let mut answer = String::new();
    let mut heard = false;
    let mut last_heard = Instant::now();
    let mut last_update: Option<Instant> = None;

    loop {
        if cancelled() {
            return Ended::Cancelled;
        }
        match chunks.next(POLL) {
            Next::Text(fragment) if fragment.is_empty() => {
                heard = true;
                last_heard = Instant::now();
            }
            Next::Text(fragment) => {
                heard = true;
                last_heard = Instant::now();
                answer.push_str(&fragment);
                let due = last_update.is_none_or(|at| at.elapsed() >= COALESCE);
                if due {
                    on_update(&answer);
                    last_update = Some(Instant::now());
                }
            }
            Next::Idle => {
                let expired = match limit {
                    Limit::FirstOutput(within) => !heard && last_heard.elapsed() >= within,
                    Limit::Quiet(within) => last_heard.elapsed() >= within,
                };
                if expired {
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
            model: Some("qwen3:8b".into()),
            path: None,
            thinking: Thinking::Off,
            prompt: "Improve this.".into(),
            key: None,
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
            Limit::FirstOutput(FIRST_FRAGMENT_TIMEOUT),
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
            Limit::FirstOutput(FIRST_FRAGMENT_TIMEOUT),
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
            Limit::FirstOutput(Duration::from_millis(60)),
            &never_cancelled(),
            &mut |_| {},
        );

        let Ended::Failed(error) = ended else {
            panic!("expected a timeout");
        };
        assert_eq!(error.to_string(), "The model didn't answer in time");
    }

    #[test]
    fn a_model_that_is_still_thinking_does_not_time_out() {
        let (sender, receiver) = std::sync::mpsc::channel();
        sender.send(Ok(String::new())).unwrap();
        let chunks = Chunks::new(receiver);
        let started = Instant::now();
        let mut updates = Vec::new();

        let ended = drive(
            &chunks,
            Limit::FirstOutput(Duration::from_millis(60)),
            &|| started.elapsed() >= Duration::from_millis(200),
            &mut |so_far| updates.push(so_far.to_string()),
        );

        assert_eq!(ended, Ended::Cancelled);
        assert!(updates.is_empty(), "thinking was drawn as text");
        drop(sender);
    }

    #[test]
    fn a_cancelled_request_stops_being_read() {
        let client = super::super::provider::testing::Scripted::answering(&["one", "two"]);
        let chunks = client.stream(request());
        let mut updates = Vec::new();
        let ended = drive(
            &chunks,
            Limit::FirstOutput(FIRST_FRAGMENT_TIMEOUT),
            &|| true,
            &mut |so_far| updates.push(so_far.to_string()),
        );
        assert_eq!(ended, Ended::Cancelled);
        assert!(updates.is_empty(), "a cancelled request still drew");
    }

    #[test]
    fn a_failure_ends_the_drive() {
        let client = super::super::provider::testing::Scripted::failing(AiError::Timeout);
        let chunks = client.stream(request());
        let ended = drive(
            &chunks,
            Limit::FirstOutput(FIRST_FRAGMENT_TIMEOUT),
            &never_cancelled(),
            &mut |_| {},
        );
        assert_eq!(ended, Ended::Failed(AiError::Timeout));
    }

    #[test]
    fn an_agent_program_gets_the_quiet_limit_and_a_service_the_first_output_one() {
        let agent = Request {
            kind: ProviderKind::Claude,
            ..request()
        };
        assert_eq!(limit_for(&agent), Limit::Quiet(AGENT_QUIET_LIMIT));
        assert_eq!(AGENT_QUIET_LIMIT, Duration::from_secs(120));
        assert_eq!(
            limit_for(&request()),
            Limit::FirstOutput(Duration::from_secs(30))
        );
    }

    #[test]
    fn a_program_that_goes_quiet_after_output_times_out() {
        let (sender, receiver) = std::sync::mpsc::channel();
        sender.send(Ok("partial".to_string())).unwrap();
        let chunks = Chunks::new(receiver);

        let ended = drive(
            &chunks,
            Limit::Quiet(Duration::from_millis(60)),
            &never_cancelled(),
            &mut |_| {},
        );

        assert_eq!(ended, Ended::Failed(AiError::Timeout));
        drop(sender);
    }

    #[test]
    fn a_program_that_keeps_printing_is_not_cut_off() {
        let (sender, receiver) = std::sync::mpsc::channel();
        let chunks = Chunks::new(receiver);
        let feeder = std::thread::spawn(move || {
            for _ in 0..10 {
                sender.send(Ok("x".to_string())).unwrap();
                std::thread::sleep(Duration::from_millis(20));
            }
        });

        let ended = drive(
            &chunks,
            Limit::Quiet(Duration::from_millis(80)),
            &never_cancelled(),
            &mut |_| {},
        );

        feeder.join().unwrap();
        assert_eq!(ended, Ended::Answer("x".repeat(10)));
    }
}
