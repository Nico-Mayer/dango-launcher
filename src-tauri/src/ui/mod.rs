//! The launcher's interaction state: query, selection, the view stack, the
//! action panel, Escape stages, and what each key does. Pure, so it is tested
//! without a window, and a renderer only sends intents and draws snapshots.

mod effect;
mod input;
mod snapshot;
mod state;
#[cfg(test)]
mod tests;

pub use effect::{ActionRequest, Effect, Origin};
pub use input::{Chord, Event, Input, Intent, ResultItem, TemplateInspection};
pub use snapshot::Snapshot;
pub use state::Launcher;
