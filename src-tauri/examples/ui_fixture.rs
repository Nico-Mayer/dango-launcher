//! Runs the launcher state machine for the browser fixtures in
//! `tests/fixtures`, so their keys reach the same `Launcher` the app uses.
//! Development only. One `Launcher` per page session.
//!
//! `cargo run --example ui_fixture -- [port]`, default port 1423.

use std::collections::HashMap;

use dango_lib::protocol::ViewTree;
use dango_lib::ui::{Effect, Event, Input, Intent, Launcher, ResultItem, TemplateInspection};
use serde::Deserialize;
use serde_json::{json, Value};
use tiny_http::{Header, Method, Response, Server};

#[derive(Deserialize)]
struct Request {
    session: String,
    development: bool,
    input: FixtureInput,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum FixtureInput {
    Intent { intent: Intent },
    Event { event: FixtureEvent },
}

/// The backend events the fixture plays the backend for.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum FixtureEvent {
    Results {
        query: String,
        items: Vec<ResultItem>,
    },
    Rendered {
        owner: String,
        invocation: u64,
        tree: ViewTree,
    },
    Failed {
        message: String,
    },
    TemplateInspected {
        field: String,
        source: String,
        inspection: TemplateInspection,
    },
    Activated {
        held: bool,
    },
    Reset,
}

impl From<FixtureInput> for Input {
    fn from(input: FixtureInput) -> Self {
        let event = match input {
            FixtureInput::Intent { intent } => return Input::Intent(intent),
            FixtureInput::Event { event } => event,
        };
        Input::Event(match event {
            FixtureEvent::Results { query, items } => Event::Results { query, items },
            FixtureEvent::Rendered {
                owner,
                invocation,
                tree,
            } => Event::Rendered {
                owner,
                invocation,
                tree,
            },
            FixtureEvent::Failed { message } => Event::Failed(message),
            FixtureEvent::TemplateInspected {
                field,
                source,
                inspection,
            } => Event::TemplateInspected {
                field,
                source,
                inspection,
            },
            FixtureEvent::Activated { held } => Event::Activated { held, probe: None },
            FixtureEvent::Reset => Event::Reset,
        })
    }
}

/// An effect as the call the app's shell would make for it, named after the
/// commands the fixtures recorded before the state machine moved to Rust.
fn call(effect: &Effect) -> Value {
    let (command, args) = match effect {
        Effect::Search(query) => ("search", json!({ "query": query })),
        Effect::RunAction(request) => {
            let mut args = json!({
                "extensionId": request.extension_id,
                "itemId": request.item_id,
                "actionId": request.action_id,
                "values": request.values,
            });
            if let Some(query) = &request.query {
                args["query"] = json!(query);
            }
            ("run_action", args)
        }
        Effect::InvokeCommand { command_id, query } => (
            "invoke_command",
            json!({ "commandId": command_id, "query": query }),
        ),
        Effect::CancelInvocation => ("cancel_invocation", Value::Null),
        Effect::Dismiss => ("dismiss", Value::Null),
        Effect::CopyText(text) => ("copy_text", json!({ "text": text })),
        Effect::InspectTemplate { field, source } => (
            "inspect_template",
            json!({ "field": field, "source": source }),
        ),
        Effect::FinishWarmup => ("warmup_done", Value::Null),
        Effect::ReportPaint(probe) => ("report_paint", json!({ "id": probe })),
    };
    json!({ "command": command, "args": args })
}

fn apply(sessions: &mut HashMap<String, Launcher>, body: &str) -> Result<Value, String> {
    let request: Request = serde_json::from_str(body).map_err(|error| error.to_string())?;
    let launcher = sessions
        .entry(request.session)
        .or_insert_with(|| Launcher::new(request.development));
    let effects = launcher.apply(request.input.into());
    Ok(json!({
        "snapshot": launcher.snapshot(),
        "calls": effects.iter().map(call).collect::<Vec<_>>(),
    }))
}

fn main() {
    let port = std::env::args().nth(1).unwrap_or_else(|| "1423".into());
    let server = Server::http(format!("127.0.0.1:{port}")).expect("fixture server port is free");
    let cors = Header::from_bytes("Access-Control-Allow-Origin", "*").unwrap();
    let mut sessions = HashMap::new();
    eprintln!("[ui_fixture] listening on 127.0.0.1:{port}");
    for mut request in server.incoming_requests() {
        let response = match (request.method(), request.url()) {
            (Method::Get, "/health") => Response::from_string("ok"),
            (Method::Post, "/apply") => {
                let mut body = String::new();
                let read = request.as_reader().read_to_string(&mut body);
                match read
                    .map_err(|e| e.to_string())
                    .and_then(|_| apply(&mut sessions, &body))
                {
                    Ok(answer) => Response::from_string(answer.to_string()),
                    Err(error) => {
                        eprintln!("[ui_fixture] bad request: {error}");
                        Response::from_string(error).with_status_code(400)
                    }
                }
            }
            _ => Response::from_string("not found").with_status_code(404),
        };
        let _ = request.respond(response.with_header(cors.clone()));
    }
}
