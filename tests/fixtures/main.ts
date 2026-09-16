import { mount, tick, unmount } from "svelte";
import { emit } from "@tauri-apps/api/event";
import { mockConvertFileSrc, mockIPC } from "@tauri-apps/api/mocks";
import type { InvokeArgs } from "@tauri-apps/api/core";
import App from "../../src/App.svelte";
import "../../src/app.css";
import type { ResultItem } from "../../src/lib/types";
import type { ViewTree } from "../../src/protocol/ViewTree";
import { fixtureNames, longActions, rootItems, treeFor, type FixtureName } from "./data";

if (!import.meta.env.DEV) throw new Error("Browser fixtures are development-only");

mockConvertFileSrc("windows");

const calls: { command: string; args?: InvokeArgs }[] = [];
let current: FixtureName = "root";
let invocation = 0;
let streamTimer: ReturnType<typeof setInterval> | undefined;
let templateInspection: { arguments: string[]; error: string | null } = { arguments: ["name"], error: null };

function stopStream() {
  clearInterval(streamTimer);
  streamTimer = undefined;
}

async function results(query: string, items: ResultItem[], complete = true) {
  await emit("dango://results", { query, items, complete });
  await tick();
}

async function replace(tree: ViewTree) {
  await emit("dango://render", { owner: "fixture", invocation, tree });
  await tick();
}

mockIPC((command, args) => {
  calls.push({ command, args });
  const payload = args as Record<string, unknown> | undefined;
  switch (command) {
    case "search": {
      const query = String(payload?.query ?? "");
      const items = current === "long-actions"
        ? rootItems.map((item) => ({ ...item, actions: longActions }))
        : rootItems;
      // Events arrive after App registers its listeners, as native IPC does.
      setTimeout(() => void results(query, items.filter((item) =>
        item.title.toLowerCase().includes(query.toLowerCase()))), 0);
      return;
    }
    case "run_action":
      return { kind: "done" };
    case "inspect_template":
      return templateInspection;
    case "dismiss":
    case "cancel_invocation":
      stopStream();
      return;
    case "warmup_done":
    case "report_paint":
    case "invoke_command":
      return;
    default:
      throw new Error(`Unstubbed fixture command: ${command}`);
  }
}, { shouldMockEvents: true });

async function show(name: FixtureName) {
  stopStream();
  current = name;
  await emit("dango://reset");
  await tick();
  invocation += 1;
  const tree = treeFor(name);
  if (tree) await replace(tree);
  if (name === "failure") await emit("dango://failed", "Couldn't paste. Try again.");
  await emit("dango://activate", null);
  await tick();
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

function setTemplateInspection(result: typeof templateInspection) {
  templateInspection = result;
}

async function destroy() {
  stopStream();
  await unmount(app);
}

export const fixture = { destroy, calls, show, results, replace, stream, stopStream, emit, setTemplateInspection };
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
