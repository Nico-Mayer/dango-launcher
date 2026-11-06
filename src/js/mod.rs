//! JavaScript extensions: folders in the config directory run by an embedded
//! QuickJS runtime.

mod session;

pub use session::{JsError, Session};
