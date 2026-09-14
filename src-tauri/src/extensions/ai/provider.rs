//! The seam between a command and whatever answers it.
//!
//! A command builds a `Request` and reads `Chunks`. Everything about the model
//! service lives on the other side of `Completions`, so the client can be
//! swapped and a test can answer without a network.

use std::time::Duration;

pub use crate::config::ProviderKind;

/// How hard the model should think before answering. A model that cannot think
/// ignores it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Thinking {
    #[default]
    Off,
    Low,
    Medium,
    High,
}

impl Thinking {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "off" => Some(Thinking::Off),
            "low" => Some(Thinking::Low),
            "medium" => Some(Thinking::Medium),
            "high" => Some(Thinking::High),
            _ => None,
        }
    }
}

/// The program behind a provider of the `cli` kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliSpec {
    pub command: String,
    pub args: Vec<String>,
}

/// One resolved request: which service, which model, what to ask, and the key
/// to ask with. Built fresh per invocation, so no key is held between them.
#[derive(Debug, Clone)]
pub struct Request {
    /// The name the user gave this provider. Only for the messages they read.
    pub provider: String,
    pub kind: ProviderKind,
    pub base_url: Option<String>,
    pub model: String,
    pub thinking: Thinking,
    pub prompt: String,
    pub key: Option<String>,
    /// Present only for a command provider.
    pub cli: Option<CliSpec>,
}

impl Request {
    /// Whether the service runs on the user's own machine, which changes the
    /// message when it cannot be reached and means no key is needed.
    pub fn is_local(&self) -> bool {
        matches!(self.kind, ProviderKind::Ollama | ProviderKind::Cli)
    }
}

/// What a failed request tells the user. Every variant is a complete message:
/// what did not happen, and what to do about it. The raw cause never gets here,
/// it goes to the log.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AiError {
    #[error("{provider} needs a key. Add one to auth.json in your config folder.")]
    MissingKey { provider: String },
    #[error("{provider} didn't accept your key. Check it in auth.json.")]
    KeyRejected { provider: String },
    #[error("{provider} doesn't have that model. Check the model name in your config file.")]
    UnknownModel { provider: String },
    #[error("{provider} couldn't be reached. Check your connection and try again.")]
    Unreachable { provider: String },
    #[error("The local model couldn't be reached. Check that your model server is running.")]
    LocalUnreachable,
    #[error("{provider} is busy right now. Try again in a moment.")]
    RateLimited { provider: String },
    #[error("The model didn't answer in time. Try again.")]
    Timeout,
    #[error("auth.json couldn't be read. Check the file in your config folder.")]
    KeyFileUnreadable,
    #[error("Dango couldn't run {command}. Check that it's installed and on your PATH.")]
    CommandMissing { command: String },
    #[error("{provider} gave no answer. See dango.log for what it printed.")]
    CommandFailed { provider: String },
}

/// What the next poll of a running request produced.
#[derive(Debug, PartialEq, Eq)]
pub enum Next {
    Text(String),
    /// The answer is complete.
    Done,
    Failed(AiError),
    /// Nothing arrived within the time asked for. The caller decides whether
    /// that is a timeout or just a slow model.
    Idle,
}

/// A running request, read one fragment at a time. Dropping it abandons the
/// request.
pub struct Chunks {
    receiver: std::sync::mpsc::Receiver<Result<String, AiError>>,
    /// Run when the answer is let go of. An HTTP request ends by itself once
    /// nothing is reading it; a child process has to be told.
    cleanup: Option<Box<dyn FnOnce() + Send>>,
}

impl Chunks {
    pub fn new(receiver: std::sync::mpsc::Receiver<Result<String, AiError>>) -> Self {
        Self {
            receiver,
            cleanup: None,
        }
    }

    pub fn with_cleanup(
        receiver: std::sync::mpsc::Receiver<Result<String, AiError>>,
        cleanup: impl FnOnce() + Send + 'static,
    ) -> Self {
        Self {
            receiver,
            cleanup: Some(Box::new(cleanup)),
        }
    }

    pub fn next(&self, within: Duration) -> Next {
        use std::sync::mpsc::RecvTimeoutError;

        match self.receiver.recv_timeout(within) {
            Ok(Ok(text)) => Next::Text(text),
            Ok(Err(error)) => Next::Failed(error),
            Err(RecvTimeoutError::Timeout) => Next::Idle,
            Err(RecvTimeoutError::Disconnected) => Next::Done,
        }
    }
}

impl Drop for Chunks {
    fn drop(&mut self) {
        if let Some(cleanup) = self.cleanup.take() {
            cleanup();
        }
    }
}

/// Whatever answers a request. The `genai` client is one implementation; the
/// tests use a scripted one.
pub trait Completions: Send + Sync {
    fn stream(&self, request: Request) -> Chunks;
}

#[cfg(test)]
pub mod testing {
    use super::*;
    use std::sync::Mutex;

    /// A `Completions` that replays what it was given, and remembers the
    /// request it was asked for.
    pub struct Scripted {
        script: Mutex<Vec<Result<String, AiError>>>,
        pub seen: Mutex<Vec<Request>>,
    }

    impl Scripted {
        pub fn answering(fragments: &[&str]) -> Self {
            Self {
                script: Mutex::new(fragments.iter().map(|text| Ok(text.to_string())).collect()),
                seen: Mutex::new(Vec::new()),
            }
        }

        pub fn failing(error: AiError) -> Self {
            Self {
                script: Mutex::new(vec![Err(error)]),
                seen: Mutex::new(Vec::new()),
            }
        }

        pub fn last_request(&self) -> Option<Request> {
            self.seen.lock().unwrap().last().cloned()
        }
    }

    impl Completions for Scripted {
        fn stream(&self, request: Request) -> Chunks {
            self.seen.lock().unwrap().push(request);
            let (tx, rx) = std::sync::mpsc::channel();
            for item in self.script.lock().unwrap().iter() {
                let _ = tx.send(item.clone());
            }
            Chunks::new(rx)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::testing::Scripted;
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

    #[test]
    fn a_scripted_answer_arrives_fragment_by_fragment_and_then_ends() {
        let client = Scripted::answering(&["Hello", " there"]);
        let chunks = client.stream(request());
        let wait = Duration::from_millis(50);
        assert_eq!(chunks.next(wait), Next::Text("Hello".into()));
        assert_eq!(chunks.next(wait), Next::Text(" there".into()));
        assert_eq!(chunks.next(wait), Next::Done);
    }

    #[test]
    fn a_failure_arrives_as_the_message_the_user_reads() {
        let client = Scripted::failing(AiError::RateLimited {
            provider: "anthropic".into(),
        });
        let chunks = client.stream(request());
        let Next::Failed(error) = chunks.next(Duration::from_millis(50)) else {
            panic!("expected a failure");
        };
        assert_eq!(
            error.to_string(),
            "anthropic is busy right now. Try again in a moment."
        );
    }

    #[test]
    fn nothing_yet_is_idle_rather_than_done() {
        let (_tx, rx) = std::sync::mpsc::channel::<Result<String, AiError>>();
        let chunks = Chunks::new(rx);
        assert_eq!(chunks.next(Duration::from_millis(10)), Next::Idle);
    }

    #[test]
    fn a_thinking_level_parses_and_an_unknown_one_does_not() {
        assert_eq!(Thinking::parse("high"), Some(Thinking::High));
        assert_eq!(Thinking::parse("off"), Some(Thinking::Off));
        assert_eq!(Thinking::parse("deep"), None);
    }
}
