use std::sync::Arc;

use tokio::runtime::Handle;

use crate::invocation::InvocationContext;

/// Opens a new invocation for an extension, titled when given, so an action
/// can push a view level of its own.
pub type StartView = dyn Fn(&str, Option<&str>) -> Option<Arc<InvocationContext>> + Send + Sync;

/// Whether the launcher still shows a view owned by the extension.
pub type Showing = dyn Fn(&str) -> bool + Send + Sync;

/// What every JavaScript extension shares: the runtime sessions run on and
/// the launcher hooks they report through.
pub struct JsHost {
    pub runtime: Handle,
    pub start_view: Arc<StartView>,
    pub showing: Arc<Showing>,
}
