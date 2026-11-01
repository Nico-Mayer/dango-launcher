//! Keeps the Slint MCP server of a Dango debug build reachable from Claude
//! Code. Claude Code gives up on an HTTP server that is not listening when the
//! session starts, and Slint's server only exists while a debug build runs.
//! This stdio server is always up: it forwards Slint's tools once the build
//! listens, tells Claude Code when they appear, and can start the build.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, ListToolsResult,
    PaginatedRequestParams, ServerCapabilities, ServerConfig, Tool,
};
use rmcp::service::{NotificationContext, RequestContext, RunningService};
use rmcp::transport::{stdio, StreamableHttpClientTransport};
use rmcp::{ErrorData, Peer, RoleClient, RoleServer, ServerHandler, ServiceExt};
use tokio::sync::Mutex;

const PORT: &str = "9315";
const URL: &str = "http://127.0.0.1:9315/mcp";
const LAUNCH: &str = "launch_dango";
/// The socket a second Dango uses to ask the running one to show itself.
const INSTANCE_SOCKET: &str = "com.nimayer.dango.sock";
const POLL: Duration = Duration::from_secs(2);
/// A cold debug build of the whole app takes minutes.
const LAUNCH_DEADLINE: Duration = Duration::from_secs(600);

type Upstream = RunningService<RoleClient, ()>;

#[derive(Default)]
struct Proxy {
    upstream: Mutex<Option<Upstream>>,
    client: OnceLock<Peer<RoleServer>>,
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

    async fn launch(&self) -> CallToolResult {
        if self.connected().await {
            return text("Dango is already running with its MCP server.");
        }
        let root = repo_root();
        let log_path = root.join("target").join("dango-ui-mcp.log");
        let child = std::fs::create_dir_all(root.join("target"))
            .and_then(|()| std::fs::File::create(&log_path))
            .and_then(|log| Ok((log.try_clone()?, log)))
            .and_then(|(out, err)| {
                let mut command = tokio::process::Command::new("cargo");
                command
                    .args(["run", "--features", "slint/mcp"])
                    .env("SLINT_MCP_PORT", PORT)
                    .current_dir(&root)
                    .stdin(Stdio::null())
                    .stdout(out)
                    .stderr(err);
                #[cfg(windows)]
                command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
                command.spawn()
            });
        let mut child = match child {
            Ok(child) => child,
            Err(error) => return failure(format!("Couldn't start `cargo run`: {error}")),
        };

        let started = Instant::now();
        while started.elapsed() < LAUNCH_DEADLINE {
            tokio::time::sleep(Duration::from_millis(500)).await;
            if let Ok(Some(status)) = child.try_wait() {
                return failure(format!(
                    "The debug build stopped ({status}). If another Dango is running, \
                     quit it first: only one runs at a time.\n\nEnd of {}:\n{}",
                    log_path.display(),
                    tail(&log_path, 20)
                ));
            }
            // Slint's server starts listening on the first show.
            let _ = tokio::task::spawn_blocking(show).await;
            if self.connect().await || self.connected().await {
                return text(format!(
                    "Dango's debug build is running and its tools are available. Log: {}",
                    log_path.display()
                ));
            }
        }
        failure(format!(
            "The debug build did not open its MCP server in time. Log: {}",
            log_path.display()
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
            "Inspects and drives the Dango launcher UI of a debug build. When only \
             `launch_dango` is listed, call it to build and start Dango; the UI tools \
             appear once it runs.",
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
        let mut tools = vec![launch_tool()];
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
        if request.name == LAUNCH {
            return Ok(self.launch().await.into());
        }
        let upstream = self.upstream.lock().await;
        let Some(upstream) = upstream.as_ref() else {
            return Ok(failure(format!(
                "Dango isn't running with its MCP server. Call `{LAUNCH}` first."
            ))
            .into());
        };
        Ok(match upstream.call_tool(request).await {
            Ok(result) => result.into(),
            Err(error) => failure(format!("Dango didn't answer: {error}")).into(),
        })
    }
}

fn launch_tool() -> Tool {
    let mut schema = rmcp::model::JsonObject::new();
    schema.insert("type".into(), "object".into());
    Tool::new(
        LAUNCH,
        "Build and start a Dango debug build with the Slint MCP server, show the \
         launcher once so the server starts, and wait until its UI tools are \
         available. Fails if another Dango is already running.",
        schema,
    )
}

fn text(message: impl Into<String>) -> CallToolResult {
    CallToolResult::success(vec![ContentBlock::text(message)])
}

fn failure(message: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(message)])
}

fn show() -> std::io::Result<()> {
    use interprocess::local_socket::{prelude::*, GenericNamespaced, Stream};
    let mut stream = Stream::connect(INSTANCE_SOCKET.to_ns_name::<GenericNamespaced>()?)?;
    stream.write_all(b"show\n")
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn tail(path: &Path, lines: usize) -> String {
    let log = std::fs::read_to_string(path).unwrap_or_default();
    let all: Vec<&str> = log.lines().collect();
    all[all.len().saturating_sub(lines)..].join("\n")
}

#[tokio::main]
async fn main() {
    let proxy = Arc::new(Proxy::default());
    tokio::spawn(proxy.clone().watch());
    proxy.connect().await;
    let Ok(server) = proxy.serve(stdio()).await else {
        return;
    };
    let _ = server.waiting().await;
}
