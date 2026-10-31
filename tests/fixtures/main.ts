import { mount, tick, unmount } from "svelte";
import { emit } from "@tauri-apps/api/event";
import { mockConvertFileSrc, mockIPC } from "@tauri-apps/api/mocks";
import App from "../../src/App.svelte";
import "../../src/app.css";
import type { Intent } from "../../src/protocol/Intent";
import type { ResultItem } from "../../src/protocol/ResultItem";
import type { Snapshot } from "../../src/protocol/Snapshot";
import type { ViewTree } from "../../src/protocol/ViewTree";
import { fixtureNames, longActions, rootItems, treeFor, type FixtureName } from "./data";

if (!import.meta.env.DEV) throw new Error("Browser fixtures are development-only");

mockConvertFileSrc("windows");

/// The launcher state machine, served by `src-tauri/examples/ui_fixture.rs`.
const server = "http://127.0.0.1:1423";
const session = crypto.randomUUID();
const development = new URLSearchParams(location.search).get("dev") === "1";

type TemplateInspection = { arguments: string[]; error: string | null };
type Call = { command: string; args: Record<string, unknown> | null };
type BackendEvent =
  | { kind: "results"; query: string; items: ResultItem[] }
  | { kind: "rendered"; owner: string; invocation: number; tree: ViewTree }
  | { kind: "failed"; message: string }
  | { kind: "templateInspected"; field: string; source: string; inspection: TemplateInspection }
  | { kind: "activated"; held: boolean }
  | { kind: "reset" };

const calls: Call[] = [];
let current: FixtureName = "root";
let invocation = 0;
let streamTimer: ReturnType<typeof setInterval> | undefined;
let templateInspection: TemplateInspection = { arguments: ["name"], error: null };
let queue: Promise<void> = Promise.resolve();

function stopStream() {
  clearInterval(streamTimer);
  streamTimer = undefined;
}

/// One input at a time, in order, as the app's sync `dispatch` runs them.
function send(input: { type: "intent"; intent: Intent } | { type: "event"; event: BackendEvent }) {
  const next = queue.then(async () => {
    const response = await fetch(`${server}/apply`, {
      method: "POST",
      body: JSON.stringify({ session, development, input }),
    });
    if (!response.ok) throw new Error(`Fixture server: ${await response.text()}`);
    const answer = (await response.json()) as { snapshot: Snapshot; calls: Call[] };
    await emit("dango://state", answer.snapshot);
    await tick();
    answer.calls.forEach(play);
  });
  queue = next.catch((error) => console.error(error));
  return next;
}

/// The fixture's stand-in for the shell: what search and template inspection
/// would answer, from fixture data.
function play(call: Call) {
  calls.push(call);
  const args = call.args ?? {};
  switch (call.command) {
    case "search": {
      const query = String(args.query ?? "");
      const items = current === "long-actions"
        ? rootItems.map((item) => ({ ...item, actions: longActions }))
        : rootItems;
      void results(query, items.filter((item) => item.title.toLowerCase().includes(query.toLowerCase())));
      return;
    }
    case "inspect_template":
      void event({ kind: "templateInspected", field: String(args.field), source: String(args.source), inspection: templateInspection });
      return;
    case "dismiss":
    case "cancel_invocation":
      stopStream();
  }
}

const event = (backend: BackendEvent) => send({ type: "event", event: backend });

/// Resolves once every queued input, and whatever it set off, has landed.
async function settled() {
  let seen: Promise<void> | undefined;
  while (seen !== queue) {
    seen = queue;
    await seen;
  }
}

mockIPC((command, args) => {
  if (command === "dispatch") return send({ type: "intent", intent: (args as { intent: Intent }).intent });
  throw new Error(`Unstubbed fixture command: ${command}`);
}, { shouldMockEvents: true });

async function results(query: string, items: ResultItem[]) {
  await event({ kind: "results", query, items });
}

async function replace(tree: ViewTree) {
  await event({ kind: "rendered", owner: "fixture", invocation, tree });
}

async function fail(message: string) {
  await event({ kind: "failed", message });
}

async function reset() {
  await event({ kind: "reset" });
}

async function activate(held = false) {
  await event({ kind: "activated", held });
}

async function show(name: FixtureName) {
  stopStream();
  current = name;
  await reset();
  invocation += 1;
  const tree = treeFor(name);
  if (tree) await replace(tree);
  if (name === "failure") await fail("Couldn't paste. Try again.");
  await activate();
  await settled();
}

async function reactivate() {
  await activate(true);
  await settled();
}

function stream(chunks = 30, interval = 1000 / 30) {
  stopStream();
  let chunk = 0;
  streamTimer = setInterval(() => {
    chunk += 1;
    void replace({
      protocolVersion: 1,
      view: {
        kind: "detail", markdown: `Streaming text ${chunk}\n${"Latest supplied content. ".repeat(chunk)}`,
        loading: chunk < chunks, actions: longActions,
      },
    });
    if (chunk >= chunks) stopStream();
  }, interval);
}

function setTemplateInspection(result: TemplateInspection) {
  templateInspection = result;
}

async function destroy() {
  stopStream();
  await unmount(app);
}

export const fixture = {
  destroy, calls, show, reactivate, results, replace, fail, reset, activate, settled, stream, stopStream,
  setTemplateInspection,
};
declare global {
  interface Window { dangoFixture: typeof fixture }
}
window.dangoFixture = fixture;
const app = mount(App, { target: document.getElementById("app")! });
await tick();
const requested = new URLSearchParams(location.search).get("scene") ?? "root";
if (!fixtureNames.includes(requested as FixtureName)) throw new Error(`Unknown fixture: ${requested}`);
await show(requested as FixtureName);
window.addEventListener("pagehide", stopStream);

if (new URLSearchParams(location.search).has("tokens")) {
  const { default: TokenPortal } = await import("./TokenPortal.svelte");
  mount(TokenPortal, { target: document.getElementById("app")! });
}
