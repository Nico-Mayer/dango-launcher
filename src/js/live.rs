//! A session bound to the launcher: where its views go and when it stops.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use rquickjs::prelude::Func;
use rquickjs::{Ctx, Exception, Object};

use crate::invocation::InvocationContext;
use crate::protocol::{View, ViewTree, PROTOCOL_VERSION};

use super::host::JsHost;
use super::session::{JsError, Natives, Session, Target};

/// Carries views from extension code to the invocation they belong to.
pub struct Bridge {
    extension_id: String,
    host: Arc<JsHost>,
    context: Mutex<Option<InvocationContext>>,
    /// Set while an action runs: its first push opens a level of its own.
    opens_level: AtomicBool,
    pushed: AtomicBool,
}

impl Bridge {
    fn push(&self, json: &str) -> Result<(), String> {
        let view: View = serde_json::from_str(json).map_err(|error| error.to_string())?;
        let tree = ViewTree {
            protocol_version: PROTOCOL_VERSION,
            view,
        };
        let mut context = self.context.lock().unwrap();
        if self.opens_level.swap(false, Ordering::SeqCst) {
            let opened = (self.host.start_view)(&self.extension_id, None)
                .ok_or("the launcher can't show a new view")?;
            *context = Some((*opened).clone());
        }
        let context = context.as_ref().ok_or("there is no view to show this in")?;
        context.push_view(tree);
        self.pushed.store(true, Ordering::SeqCst);
        Ok(())
    }

    pub fn context(&self) -> Option<InvocationContext> {
        self.context.lock().unwrap().clone()
    }
}

impl Natives for Bridge {
    fn install<'js>(
        self: Arc<Self>,
        _ctx: &Ctx<'js>,
        native: &Object<'js>,
    ) -> rquickjs::Result<()> {
        let bridge = self.clone();
        native.set(
            "push",
            Func::from(move |ctx: Ctx<'js>, json: String| -> rquickjs::Result<()> {
                bridge
                    .push(&json)
                    .map_err(|message| Exception::throw_message(&ctx, &message))
            }),
        )
    }
}

pub struct Live {
    pub session: Session,
    pub bridge: Arc<Bridge>,
    pub cancel: Arc<AtomicBool>,
}

impl Live {
    pub async fn start(
        source: &str,
        extension_id: &str,
        host: Arc<JsHost>,
        context: Option<InvocationContext>,
    ) -> Result<Self, JsError> {
        let bridge = Arc::new(Bridge {
            extension_id: extension_id.to_owned(),
            host,
            context: Mutex::new(context),
            opens_level: AtomicBool::new(false),
            pushed: AtomicBool::new(false),
        });
        let cancel = Arc::new(AtomicBool::new(false));
        let session = Session::start(source, cancel.clone(), bridge.clone()).await?;
        Ok(Self {
            session,
            bridge,
            cancel,
        })
    }

    pub async fn call(
        &self,
        target: Target<'_>,
        argument: Option<&str>,
    ) -> Result<Option<String>, JsError> {
        self.session.call(target, argument).await
    }

    pub fn pushed(&self) -> bool {
        self.bridge.pushed.load(Ordering::SeqCst)
    }
}
