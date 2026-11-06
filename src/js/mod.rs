//! JavaScript extensions: folders in the config directory run by an embedded
//! QuickJS runtime.

pub mod manifest;
mod session;

pub use session::{JsError, Session};
