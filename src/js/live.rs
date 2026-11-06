//! A session bound to the launcher: where its views go and when it stops.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
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
    /// The command's own invocation, where `dango.push` goes.
    command: Option<InvocationContext>,
    /// Levels actions opened, by the token their `event.push` carries, so a
    /// push from the command's code never lands in one, nor opens one.
    levels: Mutex<HashMap<u32, InvocationContext>>,
    next_level: AtomicU32,
    /// The latest invocation, the one the watcher follows.
    newest: Mutex<Option<InvocationContext>>,
    pushed: AtomicBool,
}

impl Bridge {
    fn tree(json: &str) -> Result<ViewTree, String> {
        let view: View = serde_json::from_str(json).map_err(|error| error.to_string())?;
        Ok(ViewTree {
            protocol_version: PROTOCOL_VERSION,
            view,
        })
    }

    fn push(&self, json: &str) -> Result<(), String> {
        let tree = Self::tree(json)?;
        let context = self
            .command
            .as_ref()
            .ok_or("there is no view to show this in")?;
        context.push_view(tree);
        self.pushed.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// An action's push: the first opens a level of its own and returns its
    /// token; a push carrying that token replaces what the level shows.
    fn push_level(&self, json: &str, token: Option<u32>) -> Result<u32, String> {
        let tree = Self::tree(json)?;
        let mut levels = self.levels.lock().unwrap();
        let token = match token.filter(|token| levels.contains_key(token)) {
            Some(token) => token,
            None => {
                let opened = (self.host.start_view)(&self.extension_id, None)
                    .ok_or("the launcher can't show a new view")?;
                let token = self.next_level.fetch_add(1, Ordering::SeqCst);
                *self.newest.lock().unwrap() = Some((*opened).clone());
                levels.insert(token, (*opened).clone());
                token
            }
        };
        levels[&token].push_view(tree);
        self.pushed.store(true, Ordering::SeqCst);
        Ok(token)
    }

    /// How many levels actions have opened, to tell whether one just did.
    pub fn levels_opened(&self) -> u32 {
        self.next_level.load(Ordering::SeqCst)
    }

    pub fn context(&self) -> Option<InvocationContext> {
        self.newest.lock().unwrap().clone()
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
                Func::from(Async(move |ctx: Ctx<'js>, text: String| {
                    let action = action.clone();
                    async move {
                        tokio::task::spawn_blocking(move || action(&text))
                            .await
                            .map_err(|error| throw(&ctx, error))?
                            .map_err(|message| throw(&ctx, message))
                    }
                })),
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
            Func::from(Async(move |ctx: Ctx<'js>, name: String| {
                let secret = secret.clone();
                let id = id.clone();
                async move {
                    let read = {
                        let id = id.clone();
                        tokio::task::spawn_blocking(move || secret(&id, &name))
                            .await
                            .map_err(|error| throw(&ctx, error))?
                    };
                    read.map_err(|detail| {
                        crate::log::append(&format!("extension {id}: {detail}"));
                        throw(&ctx, "auth.toml can't be read")
                    })
                }
            })),
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
            "pushLevel",
            Func::from(
                move |ctx: Ctx<'js>, json: String, token: Option<u32>| -> rquickjs::Result<u32> {
                    bridge
                        .push_level(&json, token)
                        .map_err(|message| throw(&ctx, message))
                },
            ),
        )?;
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

/// How long `main.js` may take to load, top-level awaits included. Nothing
/// else can stop it then: the watcher only starts once the session exists.
const LOAD_LIMIT: Duration = if cfg!(test) {
    Duration::from_secs(1)
} else {
    Duration::from_secs(10)
};

pub struct Live {
    session: Session,
    bridge: Arc<Bridge>,
    cancel: Arc<AtomicBool>,
    interrupt: Notify,
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
            newest: Mutex::new(context.clone()),
            command: context,
            levels: Mutex::default(),
            next_level: AtomicU32::new(0),
            pushed: AtomicBool::new(false),
        });
        let cancel = Arc::new(AtomicBool::new(false));
        let loaded = Arc::new(AtomicBool::new(false));
        // A top-level loop holds the task it runs in, so the limit is also
        // enforced from a task of its own, through the interrupt flag.
        bridge.host.runtime.spawn({
            let (cancel, loaded) = (cancel.clone(), loaded.clone());
            async move {
                tokio::time::sleep(LOAD_LIMIT).await;
                if !loaded.load(Ordering::SeqCst) {
                    cancel.store(true, Ordering::SeqCst);
                }
            }
        });
        let started = tokio::time::timeout(
            LOAD_LIMIT,
            Session::start(source, cancel.clone(), bridge.clone()),
        )
        .await;
        loaded.store(true, Ordering::SeqCst);
        let session = match started {
            Ok(Ok(session)) => session,
            Ok(Err(JsError::Interrupted)) | Err(_) => {
                return Err(JsError::Engine(
                    "main.js didn't finish loading in time".into(),
                ))
            }
            Ok(Err(error)) => return Err(error),
        };
        Ok(Self {
            session,
            bridge,
            cancel,
            interrupt: Notify::new(),
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
        if self.is_ended() {
            return Err(JsError::Interrupted);
        }
        let interrupted = self.interrupt.notified();
        let result = tokio::select! {
            result = self.session.call(target, argument) => result,
            _ = interrupted => Err(JsError::Interrupted),
        };
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

    /// Follows the launcher until the session's work is over: once its
    /// invocation is abandoned and none of the extension's views are on
    /// screen, running code is interrupted and the session ends. A view below
    /// a left one keeps the session, and its slow actions, going.
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
                if abandoned && !(live.bridge.host.showing)(&live.bridge.extension_id) {
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
