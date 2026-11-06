//! JavaScript extensions: folders in the config directory run by an embedded
//! QuickJS runtime.

mod extension;
pub mod load;
pub mod manifest;
mod session;

pub use extension::JsExtension;
pub use session::{JsError, Session};
