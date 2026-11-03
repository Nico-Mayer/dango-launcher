//! Keeps the Slint MCP server of a Dango debug build reachable from Claude
//! Code. Claude Code gives up on an HTTP server that is not listening when the
//! session starts, and Slint's server only exists while a debug build runs.
//! This stdio server is always up: it forwards Slint's tools once the build
//! listens, tells Claude Code when they appear, starts and restarts the build,
//! and adds tools Slint's set lacks for an agent.

mod dango;
mod keys;
mod snapshot;

use std::io::Write;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, JsonObject,
    ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerConfig, Tool,
};
use rmcp::service::{NotificationContext, RequestContext, RunningService};
use rmcp::transport::{stdio, StreamableHttpClientTransport};
use rmcp::{ErrorData, Peer, RoleClient, RoleServer, ServerHandler, ServiceExt};
use serde_json::{json, Value};
use tokio::process::Child;
use tokio::sync::Mutex;

use dango::Checkout;

const URL: &str = "http://127.0.0.1:9315/mcp";
const LAUNCH: &str = "launch_dango";
const RESTART: &str = "restart_dango";
const PRESS: &str = "press";
const SNAPSHOT: &str = "snapshot";
/// The socket a second Dango uses to ask the running one to show itself.
const INSTANCE_SOCKET: &str = "com.nimayer.dango.sock";
const POLL: Duration = Duration::from_secs(2);
/// A cold debug build of the whole app takes minutes.
const LAUNCH_DEADLINE: Duration = Duration::from_secs(600);
/// A running Dango opens Slint's server on its first show.
const SHOW_DEADLINE: Duration = Duration::from_secs(10);

type Upstream = RunningService<RoleClient, ()>;

struct Proxy {
    upstream: Mutex<Option<Upstream>>,
    client: OnceLock<Peer<RoleServer>>,
    checkout: Checkout,
    dango: Mutex<Option<Child>>,
}

impl Proxy {
    /// Connects when nothing is connected. True when this call connected.
    async fn connect(&self) -> bool {
        let mut upstream = self.upstream.lock().await;
        if upstream.as_ref().is_some_and(|u| !u.is_transport_closed()) {
            return false;
        }
        *upstream = None;
        let Ok(service) = ().serve(StreamableHttpClientTransport::from_uri(URL)).await else {
            return false;
        };
        *upstream = Some(service);
        drop(upstream);
        self.announce().await;
        true
    }

    async fn announce(&self) {
        if let Some(client) = self.client.get() {
            let _ = client.notify_tool_list_changed().await;
        }
    }

    async fn connected(&self) -> bool {
        self.upstream
            .lock()
            .await
            .as_ref()
            .is_some_and(|u| !u.is_transport_closed())
    }

    /// Notices a build that stopped, so its tools go away, and one that
    /// started some other way, so its tools appear.
    async fn watch(self: Arc<Self>) {
        loop {
            tokio::time::sleep(POLL).await;
            if !self.connected().await {
                let was_connected = self.upstream.lock().await.take().is_some();
                if !self.connect().await && was_connected {
                    self.announce().await;
                }
            }
        }
    }

    /// Shows the launcher until Slint's server answers.
    async fn show_until_connected(&self, deadline: Duration) -> bool {
        let started = Instant::now();
        while started.elapsed() < deadline {
            let _ = tokio::task::spawn_blocking(show).await;
            if self.connect().await || self.connected().await {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        false
    }

    async fn launch(&self) -> CallToolResult {
        if self.connected().await {
            return text("Dango is already running with its MCP server.");
        }
        if instance_socket().is_ok() {
            if self.show_until_connected(SHOW_DEADLINE).await {
                return text("Dango was already running; its tools are available.");
            }
            return failure(format!(
                "Another Dango is running without the MCP server. If it is this \
                 checkout's debug build, call `{RESTART}`; otherwise quit it first."
            ));
        }
        if let Err(error) = self.checkout.build().await {
            return failure(error);
        }
        let child = match self.checkout.spawn() {
            Ok(child) => child,
            Err(error) => return failure(format!("Couldn't start Dango: {error}")),
        };
        let mut dango = self.dango.lock().await;
        let child = dango.insert(child);

        let started = Instant::now();
        while started.elapsed() < LAUNCH_DEADLINE {
            tokio::time::sleep(Duration::from_millis(500)).await;
            if let Ok(Some(status)) = child.try_wait() {
                return failure(format!(
                    "Dango stopped ({status}). If another Dango is running, quit it \
                     first: only one runs at a time.\n\n{}",
                    self.checkout.log_tail(20)
                ));
            }
            let _ = tokio::task::spawn_blocking(show).await;
            if self.connect().await || self.connected().await {
                return text(format!(
                    "Dango's debug build is running and its tools are available. Log: {}",
                    self.checkout.log.display()
                ));
            }
        }
        failure(format!(
            "Dango did not open its MCP server in time. Log: {}",
            self.checkout.log.display()
        ))
    }

    async fn restart(&self) -> CallToolResult {
        self.upstream.lock().await.take();
        self.dango.lock().await.take();
        let checkout = self.checkout.clone();
        let stopped = tokio::task::spawn_blocking(move || checkout.stop())
            .await
            .unwrap_or(0);
        dango::wait_until_gone(|| instance_socket().is_ok()).await;
        self.announce().await;
        let result = self.launch().await;
        if stopped == 0 || result.is_error == Some(true) {
            return result;
        }
        text("Stopped the old debug build, rebuilt, and started it again. Its tools are available.")
    }

    async fn call(&self, name: &'static str, arguments: Value) -> Result<Value, String> {
        let upstream = self.upstream.lock().await;
        let upstream = upstream.as_ref().ok_or_else(not_running)?;
        let arguments = match arguments {
            Value::Object(map) => map,
            _ => JsonObject::new(),
        };
        let result = upstream
            .call_tool(CallToolRequestParams::new(name).with_arguments(arguments))
            .await
            .map_err(|error| format!("Dango didn't answer: {error}"))?;
        let body = result
            .content
            .iter()
            .filter_map(ContentBlock::as_text)
            .map(|t| t.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        if result.is_error == Some(true) {
            return Err(format!("`{name}` failed: {body}"));
        }
        Ok(result
            .structured_content
            .unwrap_or_else(|| serde_json::from_str(&body).unwrap_or(Value::Null)))
    }

    async fn window(&self, arguments: &JsonObject) -> Result<Value, String> {
        if let Some(handle) = arguments.get("windowHandle") {
            return Ok(handle.clone());
        }
        // The HUD is a window too, and lists first once it has shown. The
        // launcher is always the taller one.
        let windows = self.call("list_windows", json!({})).await?;
        let mut tallest: Option<(f64, Value)> = None;
        for handle in windows["windowHandles"].as_array().into_iter().flatten() {
            let properties = self
                .call("get_window_properties", json!({ "windowHandle": handle }))
                .await?;
            let height = properties["size"]["height"].as_f64().unwrap_or(0.0);
            if tallest.as_ref().is_none_or(|(best, _)| height > *best) {
                tallest = Some((height, handle.clone()));
            }
        }
        tallest
            .map(|(_, handle)| handle)
            .ok_or_else(|| "Dango has no window open.".into())
    }

    async fn press(&self, arguments: JsonObject) -> Result<String, String> {
        let steps: Vec<String> = arguments
            .get("steps")
            .and_then(|steps| serde_json::from_value(steps.clone()).ok())
            .ok_or("`steps` must be a list of strings.")?;
        let events = steps
            .iter()
            .map(|step| keys::parse(step))
            .collect::<Result<Vec<_>, _>>()?;
        let window = self.window(&arguments).await?;
        for event in events.into_iter().flatten() {
            self.call(
                "dispatch_key_event",
                json!({
                    "windowHandle": window,
                    "text": event.text(),
                    "eventType": event.event_type(),
                }),
            )
            .await?;
        }
        Ok(format!("Pressed {}.", steps.join(", ")))
    }

    async fn snapshot(&self, arguments: JsonObject) -> Result<String, String> {
        let window = self.window(&arguments).await?;
        let properties = self
            .call("get_window_properties", json!({ "windowHandle": window }))
            .await?;
        let scale = properties["scaleFactor"].as_f64().unwrap_or(1.0);
        let height = properties["size"]["height"].as_f64().unwrap_or(0.0) / scale;
        let tree = self
            .call(
                "get_element_tree",
                json!({
                    "elementHandle": properties["rootElementHandle"],
                    "maxElements": 1000,
                }),
            )
            .await?;
        let elements = tree["elements"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default();
        Ok(snapshot::render(
            elements,
            height,
            tree["truncated"].as_bool().unwrap_or(false),
        ))
    }
}

impl ServerHandler for Proxy {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_tool_list_changed()
                .build(),
        )
        .with_instructions(
            "Inspects and drives the Dango launcher UI of a debug build. When only the \
             proxy's own tools are listed, call `launch_dango`; Slint's tools appear once \
             Dango runs. After changing code, call `restart_dango`. Prefer `snapshot` over \
             `get_element_tree` and `press` over `dispatch_key_event`.",
        )
    }

    async fn on_initialized(&self, context: NotificationContext<RoleServer>) {
        let _ = self.client.set(context.peer);
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let mut tools = own_tools();
        if let Some(upstream) = self.upstream.lock().await.as_ref() {
            if let Ok(forwarded) = upstream.list_all_tools().await {
                tools.extend(forwarded);
            }
        }
        Ok(ListToolsResult::with_all_items(tools))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let arguments = request.arguments.clone().unwrap_or_default();
        let result = match request.name.as_ref() {
            LAUNCH => self.launch().await,
            RESTART => self.restart().await,
            PRESS => outcome(self.press(arguments).await),
            SNAPSHOT => outcome(self.snapshot(arguments).await),
            _ => {
                let upstream = self.upstream.lock().await;
                match upstream.as_ref() {
                    None => failure(not_running()),
                    Some(upstream) => match upstream.call_tool(request).await {
                        Ok(result) => result,
                        Err(error) => failure(format!("Dango didn't answer: {error}")),
                    },
                }
            }
        };
        Ok(result.into())
    }
}

fn own_tools() -> Vec<Tool> {
    let window_handle = json!({
        "type": "object",
        "description": "Optional. Defaults to the launcher, the tallest window.",
        "properties": {"index": {"type": "string"}, "generation": {"type": "string"}},
    });
    vec![
        tool(
            LAUNCH,
            "Build and start this checkout's Dango debug build with the Slint MCP \
             server, show the launcher once so the server starts, and wait until its UI \
             tools are available. Fails if another Dango is already running.",
            json!({"type": "object"}),
        ),
        tool(
            RESTART,
            "Stop this checkout's running debug build (never an installed Dango), \
             rebuild, and start it again. Use after changing code.",
            json!({"type": "object"}),
        ),
        tool(
            PRESS,
            "Press keys in the Dango window, in order. Each step is a key (`down`, \
             `up`, `enter`, `escape`, `tab`, `backspace`, `home`, `end`, `pageup`, \
             `f1`, a single character), a chord with `ctrl`, `shift`, `alt`, `meta` \
             (`ctrl+k`, `shift+tab`), or `text:<characters>` to type. Use this instead \
             of dispatch_key_event, which can't carry arrow or modifier keys from an \
             agent.",
            json!({
                "type": "object",
                "properties": {
                    "steps": {"type": "array", "items": {"type": "string"}},
                    "windowHandle": window_handle,
                },
                "required": ["steps"],
            }),
        ),
        tool(
            SNAPSHOT,
            "List the Dango window's elements an agent acts on, one per line: \
             `#index/generation role \"label\" value=\"...\" selected offscreen`. \
             `#23/1` is elementHandle {\"index\":\"23\",\"generation\":\"1\"}. Far smaller \
             than get_element_tree; use that only for layout details.",
            json!({
                "type": "object",
                "properties": {"windowHandle": window_handle},
            }),
        ),
    ]
}

fn tool(name: &'static str, description: &'static str, schema: Value) -> Tool {
    let Value::Object(schema) = schema else {
        unreachable!("tool schemas are objects")
    };
    Tool::new(name, description, schema)
}

fn not_running() -> String {
    format!("Dango isn't running with its MCP server. Call `{LAUNCH}` first.")
}

fn text(message: impl Into<String>) -> CallToolResult {
    CallToolResult::success(vec![ContentBlock::text(message)])
}

fn failure(message: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(message)])
}

fn outcome(result: Result<String, String>) -> CallToolResult {
    match result {
        Ok(message) => text(message),
        Err(message) => failure(message),
    }
}

fn instance_socket() -> std::io::Result<interprocess::local_socket::Stream> {
    use interprocess::local_socket::{prelude::*, GenericNamespaced, Stream};
    Stream::connect(INSTANCE_SOCKET.to_ns_name::<GenericNamespaced>()?)
}

fn show() -> std::io::Result<()> {
    instance_socket()?.write_all(b"show\n")
}

/// Windows can't replace a running exe, so while any session runs this proxy,
/// `cargo run` in the next one fails to rebuild it after a change. Windows
/// can rename a running exe, though: move ours aside and copy it back. The
/// copy keeps the timestamp, so cargo still sees the build as fresh.
#[cfg(windows)]
fn release_own_exe() {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let Some(dir) = exe.parent() else {
        return;
    };
    // cargo hard-links the exe from `deps/`, so both names point at the
    // running file.
    let names = [exe.clone(), dir.join("deps").join("dango_ui_mcp.exe")];
    let pid = std::process::id();
    for (n, name) in names.iter().enumerate() {
        let aside = name.with_extension(format!("exe.old-{pid}-{n}"));
        if std::fs::rename(name, &aside).is_ok() {
            let _ = std::fs::copy(&aside, name);
        }
    }
    for dir in [dir.to_path_buf(), dir.join("deps")] {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy().contains(".exe.old-") {
                // Fails for files other sessions still run, which is fine.
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
}

#[tokio::main]
async fn main() {
    #[cfg(windows)]
    release_own_exe();
    let proxy = Arc::new(Proxy {
        upstream: Mutex::default(),
        client: OnceLock::new(),
        checkout: Checkout::new(),
        dango: Mutex::default(),
    });
    tokio::spawn(proxy.clone().watch());
    proxy.connect().await;
    let Ok(server) = proxy.serve(stdio()).await else {
        return;
    };
    let _ = server.waiting().await;
}
