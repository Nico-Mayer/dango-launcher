//! The one `Completions` the extension holds, handing each request to the
//! client its kind needs.

use std::sync::Arc;

use super::provider::{Chunks, Completions, Request};

pub struct Clients {
    service: Arc<dyn Completions>,
    agent: Arc<dyn Completions>,
}

impl Clients {
    pub fn new(service: Arc<dyn Completions>, agent: Arc<dyn Completions>) -> Self {
        Self { service, agent }
    }
}

impl Completions for Clients {
    fn stream(&self, request: Request) -> Chunks {
        match request.kind.is_agent() {
            true => self.agent.stream(request),
            false => self.service.stream(request),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::provider::testing::Scripted;
    use super::*;
    use crate::config::{ProviderKind, Thinking};

    fn request(kind: ProviderKind) -> Request {
        Request {
            provider: "test".into(),
            kind,
            base_url: None,
            model: None,
            path: None,
            thinking: Thinking::Off,
            prompt: "hi".into(),
            key: None,
        }
    }

    #[test]
    fn an_agent_kind_goes_to_the_agent_client_and_a_service_to_the_other() {
        let service = Arc::new(Scripted::answering(&["service"]));
        let agent = Arc::new(Scripted::answering(&["agent"]));
        let clients = Clients::new(service.clone(), agent.clone());

        clients.stream(request(ProviderKind::Claude));
        clients.stream(request(ProviderKind::Pi));
        clients.stream(request(ProviderKind::Ollama));

        let kinds = |client: &Scripted| -> Vec<ProviderKind> {
            client.seen.lock().unwrap().iter().map(|r| r.kind).collect()
        };
        assert_eq!(kinds(&agent), [ProviderKind::Claude, ProviderKind::Pi]);
        assert_eq!(kinds(&service), [ProviderKind::Ollama]);
    }
}
