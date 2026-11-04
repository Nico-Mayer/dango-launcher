//! The seam between a command and whatever answers it.
//!
//! A command builds a `Request` and reads `Chunks`. Everything about the model
//! service lives on the other side of `Completions`, so the client can be
//! swapped and a test can answer without a network.

use std::time::Duration;

pub use crate::config::ProviderKind;

pub use crate::config::Thinking;

/// One resolved request: which service, which model, what to ask, and the key
/// to ask with. Built fresh per invocation, so no key is held between them.
#[derive(Clone)]
pub struct Request {
    /// The name the user gave this provider. Only for the messages they read.
    pub provider: String,
    pub kind: ProviderKind,
    pub base_url: Option<String>,
    /// Always set for a service. An agent program without one uses its own
    /// default.
    pub model: Option<String>,
    /// The program to run for an agent kind, when the user named one.
    pub path: Option<std::path::PathBuf>,
    pub thinking: Thinking,
    pub prompt: String,
    pub key: Option<String>,
}

impl std::fmt::Debug for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Request")
            .field("provider", &self.provider)
            .field("kind", &self.kind)
            .field("base_url", &self.base_url)
            .field("model", &self.model)
            .field("path", &self.path)
            .field("thinking", &self.thinking)
            .field("prompt", &self.prompt)
            .field("key", &self.key.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

impl Request {
    /// Whether the service runs on the user's own machine, which changes the
    /// message when it cannot be reached and means no key is needed.
    pub fn is_local(&self) -> bool {
        self.kind == ProviderKind::Ollama || self.base_url.as_deref().is_some_and(is_loopback)
    }

    /// An agent program brings its own sign-in, and a local server takes none.
    pub fn needs_key(&self) -> bool {
        !self.kind.is_agent() && !self.is_local()
    }
}

fn is_loopback(base_url: &str) -> bool {
    let Ok(url) = url::Url::parse(base_url.trim()) else {
        return false;
    };
    match url.host() {
        Some(url::Host::Domain(domain)) => domain.eq_ignore_ascii_case("localhost"),
        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        None => false,
    }
}

/// What a failed request tells the user. Every variant is a complete message:
/// what did not happen, and what to do about it. The raw cause never gets here,
/// it goes to the log.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AiError {
    #[error("{provider} needs a key. Add one to auth.toml in your config folder.")]
    MissingKey { provider: String },
    #[error("{provider} didn't accept your key. Check it in auth.toml.")]
    KeyRejected { provider: String },
    #[error("{provider} doesn't have that model. Check the model name in your config file.")]
    UnknownModel { provider: String },
    #[error("{provider} couldn't be reached. Check your connection and try again.")]
    Unreachable { provider: String },
    #[error("{provider} refused this request. See dango.log for the reason.")]
    Refused { provider: String },
    #[error("{provider} had a problem on its side. Try again later.")]
    ServerError { provider: String },
    #[error("Couldn't reach your local model server")]
    LocalUnreachable,
    #[error("{provider} is busy right now. Try again in a moment.")]
    RateLimited { provider: String },
    #[error("The model didn't answer in time")]
    Timeout,
    #[error("auth.toml couldn't be read. Check the file in your config folder.")]
    KeyFileUnreadable,
    #[error("Couldn't find {program}. Install it or set its path in your config file.")]
    ProgramMissing { program: String },
    #[error("Couldn't find {path}. Check the path in your config file.")]
    PathMissing { path: String },
    #[error("{provider} stopped with an error. See dango.log for the reason.")]
    ProgramFailed { provider: String },
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
    /// An HTTP request ends by itself once nothing reads it; a child process
    /// has to be told.
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

    /// Stops whatever is still running, because Dango is quitting. A service
    /// request ends with the process; a program has to be told.
    fn stop_all(&self) {}
}

#[cfg(test)]
pub mod testing {
    use super::*;
    use std::sync::Mutex;

    type Fragments = std::sync::mpsc::Sender<Result<String, AiError>>;

    /// A `Completions` that replays what it was given, and remembers the
    /// request it was asked for.
    pub struct Scripted {
        script: Mutex<Vec<Result<String, AiError>>>,
        pub seen: Mutex<Vec<Request>>,
        /// Streams that stay open until `release`, for a run caught mid-way.
        held: Option<Mutex<Vec<Fragments>>>,
    }

    impl Scripted {
        pub fn answering(fragments: &[&str]) -> Self {
            Self {
                script: Mutex::new(fragments.iter().map(|text| Ok(text.to_string())).collect()),
                seen: Mutex::new(Vec::new()),
                held: None,
            }
        }

        pub fn failing(error: AiError) -> Self {
            Self {
                script: Mutex::new(vec![Err(error)]),
                seen: Mutex::new(Vec::new()),
                held: None,
            }
        }

        /// Sends the fragments, then keeps every stream open until `release`.
        pub fn holding(fragments: &[&str]) -> Self {
            Self {
                held: Some(Mutex::new(Vec::new())),
                ..Self::answering(fragments)
            }
        }

        pub fn release(&self) {
            if let Some(held) = &self.held {
                held.lock().unwrap().clear();
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
            if let Some(held) = &self.held {
                held.lock().unwrap().push(tx);
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
            model: Some("qwen3:8b".into()),
            path: None,
            thinking: Thinking::Off,
            prompt: "Improve this.".into(),
            key: None,
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
    fn an_endpoint_on_this_machine_is_local() {
        let openai = |base: &str| Request {
            kind: ProviderKind::Openai,
            base_url: Some(base.into()),
            ..request()
        };
        assert!(openai("http://localhost:1234/v1").is_local());
        assert!(openai("http://127.0.0.1:8080/v1").is_local());
        assert!(openai("http://[::1]:8080/v1").is_local());
        assert!(!openai("https://openrouter.ai/api/v1").is_local());
        assert!(!openai("not a url").is_local());
    }

    #[test]
    fn debug_output_never_shows_the_key() {
        let shown = format!(
            "{:?}",
            Request {
                key: Some("sk-secret".into()),
                ..request()
            }
        );
        assert!(!shown.contains("sk-secret"), "{shown}");
        assert!(shown.contains("<redacted>"), "{shown}");
    }
}
