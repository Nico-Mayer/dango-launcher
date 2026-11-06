use std::sync::Arc;

use tokio::runtime::Handle;

use crate::invocation::InvocationContext;
use crate::store::Store;

/// Opens a new invocation for an extension, titled when given, so an action
/// can push a view level of its own.
pub type StartView = dyn Fn(&str, Option<&str>) -> Option<Arc<InvocationContext>> + Send + Sync;

/// Whether the launcher still shows a view owned by the extension.
pub type Showing = dyn Fn(&str) -> bool + Send + Sync;

/// Hands text to the system: opening a URL or file, copying, pasting.
pub type TextAction = dyn Fn(&str) -> Result<(), String> + Send + Sync;

/// An extension's preferences as the config file holds them right now.
pub type Preferences = dyn Fn(&str) -> serde_json::Map<String, serde_json::Value> + Send + Sync;

/// A secret of an extension by name. `Err` carries what may be logged: the
/// file and line, never its text.
pub type Secret = dyn Fn(&str, &str) -> Result<Option<String>, String> + Send + Sync;

/// What every JavaScript extension shares: the runtime sessions run on, the
/// launcher hooks they report through, and the built-ins' system paths.
pub struct JsHost {
    pub runtime: Handle,
    pub start_view: Arc<StartView>,
    pub showing: Arc<Showing>,
    pub open: Arc<TextAction>,
    pub copy: Arc<TextAction>,
    pub paste: Arc<TextAction>,
    /// Where `dango.storage` keeps values; absent when the database is.
    pub store: Option<Arc<Store>>,
    pub preferences: Arc<Preferences>,
    pub secret: Arc<Secret>,
}
