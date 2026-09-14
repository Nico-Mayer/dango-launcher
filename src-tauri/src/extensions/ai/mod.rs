//! The AI extension: one-shot text transforms, each one a prompt the user can
//! read and change.

mod auth;
mod cli;
mod client;
pub(crate) mod commands;
mod extension;
mod provider;
mod stream;

pub use auth::{AuthError, AuthFile, Keys};
pub use cli::CliClient;
pub use client::GenAiClient;
pub use commands::EXTENSION_ID;
#[cfg(test)]
pub(crate) use extension::manifest;
pub use extension::{AiExtension, Runner};
pub use provider::{AiError, Chunks, Completions, Next, ProviderKind, Request, Thinking};

use std::sync::Arc;

/// Sends a request to whichever client answers that kind of provider. One more
/// implementation of the same trait, so nothing above it knows there are two.
pub struct Clients {
    pub service: Arc<dyn Completions>,
    pub command: Arc<dyn Completions>,
}

impl Completions for Clients {
    fn stream(&self, request: Request) -> Chunks {
        match request.kind {
            ProviderKind::Cli => self.command.stream(request),
            _ => self.service.stream(request),
        }
    }
}
