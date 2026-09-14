//! The `genai` client behind the provider seam.
//!
//! One crate covers the three kinds Dango supports: Anthropic's own protocol,
//! any OpenAI-compatible endpoint, and Ollama on the machine. What is left here
//! is the mapping: a configured provider to a service target, a thinking level
//! to a reasoning effort, and a library error to a sentence the user can act on.
//!
//! The client is built per request because the target it resolves to differs per
//! request, and a launcher makes one request at a time. Fragments travel back on
//! a channel, so dropping the `Chunks` ends the request: the send fails, the
//! task returns, and the HTTP stream is dropped with it.

use std::sync::mpsc;

use futures::StreamExt;
use genai::adapter::AdapterKind;
use genai::chat::{ChatMessage, ChatOptions, ChatRequest, ChatStreamEvent, ReasoningEffort};
use genai::resolver::{AuthData, Endpoint, ServiceTargetResolver};
use genai::{Client, ModelIden, ServiceTarget};

use super::provider::{AiError, Chunks, Completions, ProviderKind, Request, Thinking};

const ANTHROPIC_BASE_URL: &str = "https://api.anthropic.com/v1/";
const OPENAI_BASE_URL: &str = "https://api.openai.com/v1/";
const OLLAMA_BASE_URL: &str = "http://localhost:11434/";

pub struct GenAiClient {
    runtime: tokio::runtime::Handle,
}

impl GenAiClient {
    pub fn new(runtime: tokio::runtime::Handle) -> Self {
        Self { runtime }
    }
}

impl Completions for GenAiClient {
    fn stream(&self, request: Request) -> Chunks {
        let (sender, receiver) = mpsc::channel();
        self.runtime.spawn(run(request, sender));
        Chunks::new(receiver)
    }
}

async fn run(request: Request, sender: mpsc::Sender<Result<String, AiError>>) {
    let client = match build(&request) {
        Some(client) => client,
        None => {
            let _ = sender.send(Err(AiError::Unreachable {
                provider: request.provider.clone(),
            }));
            return;
        }
    };

    let chat = ChatRequest::default().append_message(ChatMessage::user(request.prompt.clone()));
    let options = options(&request);

    let started = match client
        .exec_chat_stream(&request.model, chat, Some(&options))
        .await
    {
        Ok(started) => started,
        Err(error) => {
            let _ = sender.send(Err(translate(&request, &error)));
            return;
        }
    };

    let mut stream = started.stream;
    while let Some(event) = stream.next().await {
        match event {
            Ok(ChatStreamEvent::Chunk(chunk)) => {
                if sender.send(Ok(chunk.content)).is_err() {
                    return;
                }
            }
            Ok(_) => {}
            Err(error) => {
                let _ = sender.send(Err(translate(&request, &error)));
                return;
            }
        }
    }
}

fn build(request: &Request) -> Option<Client> {
    let kind = adapter_kind(&request.kind)?;
    let endpoint = Endpoint::from_owned(base_url(request));
    let auth = match &request.key {
        Some(key) => AuthData::from_single(key.clone()),
        // Ollama's adapter wants a value it never sends.
        None => AuthData::from_single("dango"),
    };

    let resolver = ServiceTargetResolver::from_resolver_fn(
        move |target: ServiceTarget| -> Result<ServiceTarget, genai::resolver::Error> {
            Ok(ServiceTarget {
                endpoint: endpoint.clone(),
                auth: auth.clone(),
                model: ModelIden::new(kind, target.model.model_name),
            })
        },
    );

    Some(
        Client::builder()
            .with_service_target_resolver(resolver)
            .build(),
    )
}

fn adapter_kind(kind: &ProviderKind) -> Option<AdapterKind> {
    match kind {
        ProviderKind::Anthropic => Some(AdapterKind::Anthropic),
        ProviderKind::Openai => Some(AdapterKind::OpenAI),
        ProviderKind::Ollama => Some(AdapterKind::Ollama),
        // Answered by the command client, which never asks this.
        ProviderKind::Cli | ProviderKind::Unknown(_) => None,
    }
}

/// The adapters build their URLs by appending a path, so the base has to end in
/// a slash whatever the user wrote.
fn base_url(request: &Request) -> String {
    let configured = request.base_url.as_deref().map(str::trim).unwrap_or("");
    let base = if configured.is_empty() {
        match request.kind {
            ProviderKind::Anthropic => ANTHROPIC_BASE_URL,
            ProviderKind::Ollama => OLLAMA_BASE_URL,
            _ => OPENAI_BASE_URL,
        }
    } else {
        configured
    };
    match base.ends_with('/') {
        true => base.to_string(),
        false => format!("{base}/"),
    }
}

fn options(request: &Request) -> ChatOptions {
    let options = ChatOptions::default();
    match effort(request.thinking) {
        Some(effort) => options.with_reasoning_effort(effort),
        None => options,
    }
}

fn effort(thinking: Thinking) -> Option<ReasoningEffort> {
    match thinking {
        Thinking::Off => None,
        Thinking::Low => Some(ReasoningEffort::Low),
        Thinking::Medium => Some(ReasoningEffort::Medium),
        Thinking::High => Some(ReasoningEffort::High),
    }
}

/// Turns a library error into the sentence the user reads, and puts the cause
/// where only the author will see it.
fn translate(request: &Request, error: &genai::Error) -> AiError {
    crate::log::append(&format!("ai: request to {} failed: {error}", request.provider));
    match status_of(error) {
        Some(401 | 403) => AiError::KeyRejected {
            provider: request.provider.clone(),
        },
        Some(404) => AiError::UnknownModel {
            provider: request.provider.clone(),
        },
        Some(429) => AiError::RateLimited {
            provider: request.provider.clone(),
        },
        _ if needs_key(error) => AiError::MissingKey {
            provider: request.provider.clone(),
        },
        _ if timed_out(error) => AiError::Timeout,
        _ if request.is_local() => AiError::LocalUnreachable,
        _ => AiError::Unreachable {
            provider: request.provider.clone(),
        },
    }
}

fn status_of(error: &genai::Error) -> Option<u16> {
    use genai::webc::Error as WebError;

    let webc = match error {
        genai::Error::WebModelCall { webc_error, .. } => webc_error,
        genai::Error::WebAdapterCall { webc_error, .. } => webc_error,
        genai::Error::HttpError { status, .. } => return Some(status.as_u16()),
        // A failure that arrives once the answer is already streaming wraps the
        // real one. Unwrapping it is what tells a model that does not exist
        // from a server that is not there: both surface as this.
        // A transport failure carries no status and falls through to
        // unreachable, which is what it is.
        genai::Error::WebStream { error, .. } => {
            return error.downcast_ref::<genai::Error>().and_then(status_of)
        }
        _ => return None,
    };
    match webc {
        WebError::ResponseFailedStatus { status, .. } => Some(status.as_u16()),
        WebError::Reqwest(error) => error.status().map(|status| status.as_u16()),
        _ => None,
    }
}

fn needs_key(error: &genai::Error) -> bool {
    matches!(
        error,
        genai::Error::RequiresApiKey { .. }
            | genai::Error::NoAuthData { .. }
            | genai::Error::NoAuthResolver { .. }
    )
}

fn timed_out(error: &genai::Error) -> bool {
    use genai::webc::Error as WebError;

    match error {
        genai::Error::WebModelCall { webc_error, .. }
        | genai::Error::WebAdapterCall { webc_error, .. } => match webc_error {
            WebError::Reqwest(error) => error.is_timeout(),
            _ => false,
        },
        genai::Error::WebStream { error, .. } => error
            .downcast_ref::<genai::Error>()
            .is_some_and(timed_out),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(kind: ProviderKind, base: Option<&str>) -> Request {
        Request {
            provider: "test".into(),
            kind,
            base_url: base.map(str::to_string),
            model: "some-model".into(),
            thinking: Thinking::Off,
            prompt: "Improve this.".into(),
            key: None,
            cli: None,
        }
    }

    #[test]
    fn each_kind_maps_to_its_adapter() {
        assert_eq!(
            adapter_kind(&ProviderKind::Anthropic),
            Some(AdapterKind::Anthropic)
        );
        assert_eq!(adapter_kind(&ProviderKind::Openai), Some(AdapterKind::OpenAI));
        assert_eq!(adapter_kind(&ProviderKind::Ollama), Some(AdapterKind::Ollama));
        assert_eq!(adapter_kind(&ProviderKind::Unknown("mistral".into())), None);
    }

    #[test]
    fn a_kind_with_no_endpoint_falls_back_to_its_own() {
        assert_eq!(
            base_url(&request(ProviderKind::Anthropic, None)),
            ANTHROPIC_BASE_URL
        );
        assert_eq!(base_url(&request(ProviderKind::Ollama, None)), OLLAMA_BASE_URL);
        assert_eq!(base_url(&request(ProviderKind::Openai, None)), OPENAI_BASE_URL);
    }

    #[test]
    fn a_configured_endpoint_is_used_and_always_ends_in_a_slash() {
        assert_eq!(
            base_url(&request(
                ProviderKind::Openai,
                Some("https://openrouter.ai/api/v1")
            )),
            "https://openrouter.ai/api/v1/"
        );
        assert_eq!(
            base_url(&request(ProviderKind::Ollama, Some("http://box:11434/"))),
            "http://box:11434/"
        );
    }

    #[test]
    fn a_thinking_level_maps_to_a_reasoning_effort() {
        assert!(matches!(effort(Thinking::Low), Some(ReasoningEffort::Low)));
        assert!(matches!(effort(Thinking::Medium), Some(ReasoningEffort::Medium)));
        assert!(matches!(effort(Thinking::High), Some(ReasoningEffort::High)));
        assert!(effort(Thinking::Off).is_none(), "off should ask for nothing");
    }

    fn http(status: u16) -> genai::Error {
        genai::Error::HttpError {
            status: reqwest::StatusCode::from_u16(status).unwrap(),
            canonical_reason: String::new(),
            body: String::new(),
        }
    }

    #[test]
    fn a_refused_key_says_to_check_the_key_file() {
        let error = translate(&request(ProviderKind::Anthropic, None), &http(401));
        assert_eq!(
            error.to_string(),
            "test didn't accept your key. Check it in auth.json."
        );
    }

    #[test]
    fn an_unknown_model_says_to_check_the_model_name() {
        let error = translate(&request(ProviderKind::Anthropic, None), &http(404));
        assert_eq!(
            error.to_string(),
            "test doesn't have that model. Check the model name in your config file."
        );
    }

    #[test]
    fn too_many_requests_says_the_provider_is_busy() {
        let error = translate(&request(ProviderKind::Anthropic, None), &http(429));
        assert_eq!(error.to_string(), "test is busy right now. Try again in a moment.");
    }

    #[test]
    fn a_hosted_provider_that_cannot_be_reached_says_so() {
        let error = translate(&request(ProviderKind::Anthropic, None), &http(503));
        assert_eq!(
            error.to_string(),
            "test couldn't be reached. Check your connection and try again."
        );
    }

    #[test]
    fn a_local_provider_that_cannot_be_reached_names_the_model_server() {
        let error = translate(&request(ProviderKind::Ollama, None), &http(503));
        assert_eq!(
            error.to_string(),
            "The local model couldn't be reached. Check that your model server is running."
        );
    }

    fn streaming(inner: genai::Error) -> genai::Error {
        genai::Error::WebStream {
            model_iden: ModelIden::new(AdapterKind::Ollama, "some-model"),
            cause: inner.to_string(),
            error: Box::new(inner),
        }
    }

    #[test]
    fn a_failure_once_the_answer_is_streaming_is_read_through() {
        // What Ollama does for a model it has not pulled: the request is
        // accepted, then the stream fails with a 404.
        let error = translate(&request(ProviderKind::Ollama, None), &streaming(http(404)));
        assert_eq!(
            error.to_string(),
            "test doesn't have that model. Check the model name in your config file."
        );
    }

    #[test]
    fn a_stream_that_never_started_still_reads_as_unreachable() {
        let error = translate(
            &request(ProviderKind::Ollama, None),
            &genai::Error::WebStream {
                model_iden: ModelIden::new(AdapterKind::Ollama, "some-model"),
                cause: "error sending request".into(),
                error: "error sending request".into(),
            },
        );
        assert_eq!(
            error.to_string(),
            "The local model couldn't be reached. Check that your model server is running."
        );
    }

    #[test]
    fn a_missing_key_is_reported_as_one() {
        let model = ModelIden::new(AdapterKind::Anthropic, "some-model");
        let error = translate(
            &request(ProviderKind::Anthropic, None),
            &genai::Error::RequiresApiKey { model_iden: model },
        );
        assert_eq!(
            error.to_string(),
            "test needs a key. Add one to auth.json in your config folder."
        );
    }

    /// Against a model on this machine, so it is never part of a normal run:
    /// `cargo test a_local_ollama_answers -- --ignored --nocapture`.
    /// `DANGO_TEST_MODEL` picks the model; it must be one `ollama list` shows.
    #[test]
    #[ignore]
    fn a_local_ollama_answers_through_the_client() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let client = GenAiClient::new(runtime.handle().clone());
        let model = std::env::var("DANGO_TEST_MODEL").unwrap_or("qwen3:8b".to_string());
        let chunks = client.stream(Request {
            provider: "local".into(),
            kind: ProviderKind::Ollama,
            base_url: None,
            model,
            thinking: Thinking::Off,
            prompt: "Reply with exactly: pong".into(),
            key: None,
            cli: None,
        });

        let mut answer = String::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        loop {
            match chunks.next(std::time::Duration::from_millis(200)) {
                super::super::provider::Next::Text(text) => answer.push_str(&text),
                super::super::provider::Next::Done => break,
                super::super::provider::Next::Failed(error) => panic!("{error}"),
                super::super::provider::Next::Idle if std::time::Instant::now() > deadline => {
                    panic!("nothing arrived in 60s")
                }
                super::super::provider::Next::Idle => {}
            }
        }
        println!("--- answer ---
{answer}
--- end ---");
        assert!(!answer.trim().is_empty(), "the model answered with nothing");
    }

    #[test]
    fn no_message_carries_the_library_error() {
        let error = translate(&request(ProviderKind::Anthropic, None), &http(500));
        assert!(!error.to_string().contains("500"), "a code reached the user");
    }
}
