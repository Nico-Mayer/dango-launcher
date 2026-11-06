use std::sync::Arc;

use tokio::runtime::Handle;

use crate::invocation::InvocationContext;

/// Opens a new invocation for an extension, titled when given, so an action
/// can push a view level of its own.
pub type StartView = dyn Fn(&str, Option<&str>) -> Option<Arc<InvocationContext>> + Send + Sync;

/// Whether the launcher still shows a view owned by the extension.
pub type Showing = dyn Fn(&str) -> bool + Send + Sync;

/// Hands text to the system: opening a URL or file, copying, pasting.
pub type TextAction = dyn Fn(&str) -> Result<(), String> + Send + Sync;

/// What every JavaScript extension shares: the runtime sessions run on, the
/// launcher hooks they report through, and the built-ins' system paths.
pub struct JsHost {
    pub runtime: Handle,
    pub start_view: Arc<StartView>,
    pub showing: Arc<Showing>,
    pub open: Arc<TextAction>,
    pub copy: Arc<TextAction>,
    pub paste: Arc<TextAction>,
}
