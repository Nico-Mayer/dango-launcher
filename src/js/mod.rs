//! JavaScript extensions: folders in the config directory run by an embedded
//! QuickJS runtime.

mod caps;
mod exec;
mod extension;
pub(crate) mod host;
mod live;
pub mod load;
pub mod manifest;
mod root;
mod session;

pub use extension::JsExtension;
pub use host::JsHost;
pub use session::{JsError, Natives, Session, Target};
