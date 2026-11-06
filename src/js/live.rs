//! A session bound to the launcher: where its views go and when it stops.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::Notify;

use std::path::PathBuf;

use rquickjs::prelude::{Async, Func};
use rquickjs::{Ctx, Exception, Object};

use crate::invocation::InvocationContext;
use crate::protocol::{View, ViewTree, PROTOCOL_VERSION};

use super::host::JsHost;
use super::session::{JsError, Natives, Session, Target};

/// Carries views from extension code to the invocation they belong to.
pub struct Bridge {
    extension_id: String,
    folder: PathBuf,
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

    /// Marks the start of an action: its first push opens a level.
    pub fn begin_action(&self) {
        self.opens_level.store(true, Ordering::SeqCst);
    }

    /// Ends an action and says whether it opened a level of its own.
    pub fn finish_action(&self) -> bool {
        !self.opens_level.swap(false, Ordering::SeqCst)
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
        let id = self.extension_id.clone();
        native.set(
            "log",
            Func::from(move |level: String, text: String| {
                crate::log::append(&console_line(&id, &level, &text));
            }),
        )?;
        let folder = self.folder.clone();
        native.set(
            "exec",
            Func::from(Async(move |ctx: Ctx<'js>, json: String| {
                let folder = folder.clone();
                async move {
                    let request = serde_json::from_str(&json)
                        .map_err(|error| Exception::throw_message(&ctx, &error.to_string()))?;
                    let result = super::exec::exec(request, folder)
                        .await
                        .map_err(|message| Exception::throw_message(&ctx, &message))?;
                    Ok::<_, rquickjs::Error>(serde_json::to_string(&result).unwrap_or_default())
                }
            })),
        )?;
        native.set(
            "fetch",
            Func::from(Async(move |ctx: Ctx<'js>, json: String| async move {
                let request = serde_json::from_str(&json).map_err(|error| throw(&ctx, error))?;
                let response = super::caps::fetch(request)
                    .await
                    .map_err(|message| throw(&ctx, message))?;
                Ok::<_, rquickjs::Error>(serde_json::to_string(&response).unwrap_or_default())
            })),
        )?;
        native.set(
            "fs",
            Func::from(Async(move |ctx: Ctx<'js>, json: String| async move {
                let request = serde_json::from_str(&json).map_err(|error| throw(&ctx, error))?;
                super::caps::fs(request)
                    .await
                    .map_err(|message| throw(&ctx, message))
            })),
        )?;
        for (name, action) in [
            ("open", self.host.open.clone()),
            ("copy", self.host.copy.clone()),
            ("paste", self.host.paste.clone()),
        ] {
            native.set(
                name,
                Func::from(move |ctx: Ctx<'js>, text: String| -> rquickjs::Result<()> {
                    action(&text).map_err(|message| throw(&ctx, message))
                }),
            )?;
        }
        let store = self.host.store.clone();
        let id = self.extension_id.clone();
        native.set(
            "storage",
            Func::from(Async(move |ctx: Ctx<'js>, json: String| {
                let store = store.clone();
                let id = id.clone();
                async move {
                    let request =
                        serde_json::from_str(&json).map_err(|error| throw(&ctx, error))?;
                    let store = store.ok_or_else(|| throw(&ctx, "storage is unavailable"))?;
                    tokio::task::spawn_blocking(move || storage(&store, &id, request))
                        .await
                        .map_err(|error| throw(&ctx, error))?
                        .map_err(|error| throw(&ctx, error))
                }
            })),
        )?;
        let secret = self.host.secret.clone();
        let id = self.extension_id.clone();
        native.set(
            "secret",
            Func::from(
                move |ctx: Ctx<'js>, name: String| -> rquickjs::Result<Option<String>> {
                    secret(&id, &name).map_err(|detail| {
                        crate::log::append(&format!("extension {id}: {detail}"));
                        throw(&ctx, "auth.toml can't be read")
                    })
                },
            ),
        )?;
        let preferences = serde_json::Value::Object((self.host.preferences)(&self.extension_id));
        native.set("preferences", preferences.to_string())?;
        let env = serde_json::json!({
            "id": self.extension_id,
            "folder": self.folder.to_string_lossy(),
        })
        .to_string();
        native.set("env", env)?;
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

#[derive(serde::Deserialize)]
#[serde(tag = "op", rename_all = "camelCase")]
enum StorageRequest {
    Get { key: String },
    Set { key: String, value: String },
    Remove { key: String },
    Keys,
}

/// One storage call, scoped to the calling extension. Values travel as the
/// JSON they were stored as.
fn storage(
    store: &crate::store::Store,
    extension_id: &str,
    request: StorageRequest,
) -> rusqlite::Result<String> {
    Ok(match request {
        StorageRequest::Get { key } => store
            .extension_value(extension_id, &key)?
            .unwrap_or_else(|| "null".into()),
        StorageRequest::Set { key, value } => {
            store.set_extension_value(extension_id, &key, &value)?;
            "null".into()
        }
        StorageRequest::Remove { key } => {
            store.remove_extension_value(extension_id, &key)?;
            "null".into()
        }
        StorageRequest::Keys => serde_json::to_string(&store.extension_keys(extension_id)?)
            .unwrap_or_else(|_| "[]".into()),
    })
}

fn throw(ctx: &Ctx<'_>, message: impl ToString) -> rquickjs::Error {
    Exception::throw_message(ctx, &message.to_string())
}

fn console_line(extension_id: &str, level: &str, text: &str) -> String {
    format!("extension {extension_id} {level}: {text}")
}

/// How often a session looks at whether the launcher has moved on.
const WATCH: Duration = Duration::from_millis(50);

pub struct Live {
    session: Session,
    bridge: Arc<Bridge>,
    cancel: Arc<AtomicBool>,
    interrupt: Notify,
    in_flight: AtomicUsize,
    ended: AtomicBool,
}

impl Live {
    pub async fn start(
        source: &str,
        extension_id: &str,
        folder: PathBuf,
        host: Arc<JsHost>,
        context: Option<InvocationContext>,
    ) -> Result<Self, JsError> {
        let bridge = Arc::new(Bridge {
            extension_id: extension_id.to_owned(),
            folder,
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
            interrupt: Notify::new(),
            in_flight: AtomicUsize::new(0),
            ended: AtomicBool::new(false),
        })
    }

    /// Runs extension code until it returns or the launcher moves on. An
    /// interrupt stops running bytecode; the select drops an await on a host
    /// call, which bytecode checks can't reach.
    pub async fn call(
        &self,
        target: Target<'_>,
        argument: Option<&str>,
    ) -> Result<Option<String>, JsError> {
        self.in_flight.fetch_add(1, Ordering::SeqCst);
        let interrupted = self.interrupt.notified();
        let result = tokio::select! {
            result = self.session.call(target, argument) => result,
            _ = interrupted => Err(JsError::Interrupted),
        };
        if self.in_flight.fetch_sub(1, Ordering::SeqCst) == 1 {
            self.cancel.store(false, Ordering::SeqCst);
        }
        result
    }

    /// Stops the session for good: running code is interrupted and the
    /// watcher lets go of it, which drops its timers and pending work.
    pub fn end(&self) {
        self.ended.store(true, Ordering::SeqCst);
        self.stop_running();
    }

    pub fn is_ended(&self) -> bool {
        self.ended.load(Ordering::SeqCst)
    }

    fn stop_running(&self) {
        self.cancel.store(true, Ordering::SeqCst);
        self.interrupt.notify_waiters();
    }

    /// Follows the launcher until the session's work is over. Running code
    /// stops when its invocation is abandoned; the session itself ends once
    /// nothing runs and none of the extension's views are on screen.
    pub fn watch(self: &Arc<Self>, on_end: impl FnOnce(&Arc<Live>) + Send + 'static) {
        let live = self.clone();
        let runtime = self.bridge.host.runtime.clone();
        runtime.spawn(async move {
            loop {
                tokio::time::sleep(WATCH).await;
                if live.is_ended() {
                    break;
                }
                let abandoned = live
                    .bridge
                    .context()
                    .is_none_or(|context| context.is_cancelled());
                if !abandoned {
                    continue;
                }
                if live.in_flight.load(Ordering::SeqCst) > 0 {
                    live.stop_running();
                } else if !(live.bridge.host.showing)(&live.bridge.extension_id) {
                    live.end();
                    break;
                }
            }
            on_end(&live);
        });
    }

    pub fn bridge(&self) -> &Bridge {
        &self.bridge
    }

    pub fn pushed(&self) -> bool {
        self.bridge.pushed.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_console_line_names_the_extension() {
        assert_eq!(
            console_line("gh-prs", "warn", "hi"),
            "extension gh-prs warn: hi"
        );
    }
}
