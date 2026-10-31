//! The AI extension: one-shot text transforms, each one a prompt the user can
//! read and change.

mod auth;
mod client;
pub(crate) mod commands;
mod extension;
mod provider;
mod stream;

pub use auth::{AuthError, AuthFile, Keys};
pub use client::GenAiClient;
pub use commands::EXTENSION_ID;
pub(crate) use extension::manifest;
pub use extension::AiExtension;
pub use provider::{AiError, Chunks, Completions, Next, ProviderKind, Request, Thinking};
