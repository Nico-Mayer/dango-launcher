// Types for Dango extensions. Dango writes this file when it starts, so edits
// to it are lost. Reference it from an extension's main.js with:
//
//   /// <reference path="../dango.d.ts" />
//
// main.js is an ES module. It exports:
//
//   - one function per command in manifest.json, named by the command's id
//     (an id with a dash needs a quoted name:
//     `export { randomTheme as "random-theme" }`)
//   - `rootItems()`, when the manifest declares `rootItems`, returning the
//     rows Dango shows in root search: RootItem[] | Promise<RootItem[]>
//   - `onAction(itemId, actionId, values, push)`, which runs an action of a
//     root item, and of any action without an `onAction` of its own

// ---- Generated from Dango's own types ----

type ViewTree = { protocolVersion: number, view: View, };

type View = { "kind": "list" } & ListView | { "kind": "grid" } & GridView | { "kind": "detail" } & DetailView | { "kind": "form" } & FormView | { "kind": "progress" } & ProgressView;

type ListView = { onSearch?: SearchHandler, loading?: boolean, emptyState?: EmptyState, 
/**
 * In the order they are shown. Items name the one they belong to.
 */
sections?: Array<Section>, 
/**
 * Shows the selected item's detail beside the list.
 */
detailPane?: boolean, filter?: Filter, 
/**
 * Who narrows the items as the user types.
 */
filtering?: Filtering, items: Array<ListItem>, };

type GridView = { onSearch?: SearchHandler, 
/**
 * From 4 to 8; any other count is refused.
 */
columns: number, 
/**
 * A tile's width over its height.
 */
aspect?: number, loading?: boolean, emptyState?: EmptyState, sections?: Array<Section>, filter?: Filter, 
/**
 * Who narrows the items as the user types.
 */
filtering?: Filtering, items: Array<GridItem>, };

type GridItem = { id: string, title: string, subtitle?: string, content: TileContent, section?: string, keywords?: Array<string>, actions?: Array<Action>, };

type TileContent = { "kind": "image", image: string, } | { "kind": "color", color: string, } | { "kind": "glyph", glyph: string, } | { "kind": "empty" };

type Filter = { options: Array<FieldOption>, selected: string, };

type Filtering = "launcher" | "command";

type Section = { id: string, title: string, subtitle?: string, };

type ListItem = { id: string, title: string, subtitle?: string, icon?: string, 
/**
 * The id of the section it is shown under. Items without one come
 * first, under no heading.
 */
section?: string, 
/**
 * A row shows the first three.
 */
accessories?: Array<Accessory>, 
/**
 * Matched by the launcher's filter as the title is, never shown.
 */
keywords?: Array<string>, 
/**
 * Shown in the list's detail pane while the item is selected.
 */
detail?: ItemDetail, 
/**
 * The selected item's action panel. The first action is primary.
 */
actions?: Array<Action>, };

type ItemDetail = { 
/**
 * `icon:<name>` or a path to an image file, shown above the body.
 */
image?: string, markdown?: string, 
/**
 * Shown as written in place of `markdown`.
 */
text?: string, metadata?: Array<Metadata>, };

type Metadata = { label: string, value: string, };

type Accessory = { "kind": "text", text: string, } | { "kind": "icon", icon: string, } | { "kind": "tag", text: string, role?: TagRole, } | { "kind": "kbd", shortcut: Shortcut, } | { "kind": "progress", value?: number, } | { "kind": "status", status: Status, };

type Status = "working" | "needsInput" | "done" | "failed" | "queued";

type TagRole = "accent" | "muted" | "danger";

type DetailView = { 
/**
 * `icon:<name>` or a path to an image file, shown above the body.
 */
image?: string, markdown: string, 
/**
 * Shown as written in place of `markdown`.
 */
text?: string, metadata?: Array<Metadata>, loading?: boolean, actions?: Array<Action>, };

type ProgressView = { title: string, subtitle?: string, 
/**
 * From 0 to 1. Without one, the bar claims no amount.
 */
value?: number, detail?: string, steps?: Array<Step>, actions?: Array<Action>, };

type Step = { title: string, status: Status, };

type FormView = { itemId?: string, fields: Array<FormField>, actions?: Array<Action>, };

type FormField = { id: string, label?: string, kind: FieldKind, value?: string, 
/**
 * What a dropdown offers. Its value is the chosen option's id.
 */
options?: Array<FieldOption>, };

type FieldOption = { id: string, title: string, };

type FieldKind = "text" | "password" | "textarea" | "toggle" | "dropdown" | "template" | "separator";

type EmptyState = { title: string, description?: string, icon?: string, 
/**
 * Offered while the view shows nothing. The first runs on Enter.
 */
actions?: Array<Action>, };

type Action = { id?: string, onAction?: ActionHandler, title: string, shortcut?: Shortcut, 
/**
 * The heading it is listed under in the action panel.
 */
section?: string, style?: ActionStyle, };

type ActionStyle = "normal" | "danger";

type Shortcut = { key: string, modifiers?: Array<Modifier>, };

type Modifier = "cmd" | "ctrl" | "alt" | "shift";

type RootItem = { id: string, title: string, icon?: string, keywords?: Array<string>, actions?: Array<ProtocolAction>, };

type Manifest = { manifestVersion: number, id: string, name: string, icon?: string, commands?: Array<ManifestCommand>, rootItems?: ManifestRootItems, };

type ManifestCommand = { id: string, title: string, mode: CommandMode, icon?: string, keywords?: Array<string>, };

type CommandMode = "view" | "no-view";

type ManifestRootItems = { refreshMinutes?: number, };

type ProtocolAction = { id: string, title: string, shortcut?: Shortcut, 
/**
 * The heading it is listed under in the action panel.
 */
section?: string, style?: ActionStyle, };

// ---- Extension code ----

/**
 * What an action handler gets. `values` holds a form's fields by id. `push`
 * opens a view of the action's own on its first call; later calls replace
 * what that view shows.
 */
type ActionEvent = {
  itemId: string;
  values: Record<string, string>;
  push(view: View): void;
};

/**
 * What an action returns. Returning nothing closes the launcher, unless the
 * action showed a view with `push`.
 */
type ActionResult =
  | { replace: View }
  | { ticker: string }
  | { back: true }
  | { copy: string };

type ActionHandler = (
  event: ActionEvent,
) => ActionResult | void | Promise<ActionResult | void>;

/**
 * Answers the query of a list or grid that searches through its command. A
 * newer query aborts this one: check `signal.aborted` after slow work.
 */
type SearchHandler = (
  query: string,
  options: { signal: { aborted: boolean } },
) => View | void | Promise<View | void>;

type ExecResult = { code: number | null; stdout: string; stderr: string };

type ExecOptions = {
  cwd?: string;
  env?: Record<string, string>;
  input?: string;
};

type FetchInit = {
  method?: string;
  headers?: Record<string, string>;
  body?: string;
};

type FetchResponse = {
  status: number;
  ok: boolean;
  headers: Record<string, string>;
  text(): Promise<string>;
  json(): Promise<unknown>;
};

type DirEntry = { name: string; isDir: boolean };

type FileStat = { size: number; isDir: boolean; modifiedMs: number | null };

interface Dango {
  /**
   * Shows a view in place of the one this command showed. An action shows a
   * view of its own with the `push` its event carries instead.
   */
  push(view: View): void;
  /** Stops with a failure the launcher shows as given. */
  fail(message: string): never;
  /** Runs a program with no window, on the PATH a shell would give. */
  exec(program: string, args?: string[], options?: ExecOptions): Promise<ExecResult>;
  fetch(url: string, init?: FetchInit): Promise<FetchResponse>;
  fs: {
    readText(path: string): Promise<string>;
    writeText(path: string, text: string): Promise<void>;
    readDir(path: string): Promise<DirEntry[]>;
    remove(path: string): Promise<void>;
    exists(path: string): Promise<boolean>;
    stat(path: string): Promise<FileStat>;
  };
  /**
   * Values kept for this extension on this machine. They survive restarts,
   * but keep only what can be fetched again or lost without harm.
   */
  storage: {
    get(key: string): Promise<unknown>;
    set(key: string, value: unknown): Promise<void>;
    remove(key: string): Promise<void>;
    keys(): Promise<string[]>;
  };
  /** A secret from this extension's `[extensions.<id>]` table in auth.toml. */
  secret(name: string): Promise<string | undefined>;
  open(target: string): Promise<void>;
  copy(text: string): Promise<void>;
  /** Pastes text into the app the user was in before the launcher. */
  paste(text: string): Promise<void>;
  /** Every key of `[extensions.<id>]` in config.toml besides enabled and commands. */
  preferences: Readonly<Record<string, unknown>>;
  env: { id: string; folder: string };
}

declare const dango: Dango;
