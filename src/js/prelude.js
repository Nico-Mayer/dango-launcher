(() => {
  const handlers = new Map();
  let next = 0;
  let search = null;
  let searching = null;

  class DangoFailure extends Error {
    constructor(message) {
      super(message);
      this.name = "DangoFailure";
    }
  }

  // An action's handler stays in this session; the view only carries an id
  // Dango sends back when the action runs.
  const strip = (actions) =>
    actions.map((action) => {
      if (typeof action.onAction !== "function") return action;
      const { onAction, ...rest } = action;
      const id = `handler-${next++}`;
      handlers.set(id, onAction);
      return { ...rest, id };
    });

  const prepare = (view) => {
    const out = { ...view };
    if (typeof out.onSearch === "function") {
      search = out.onSearch;
      delete out.onSearch;
      if (!out.filtering) out.filtering = "command";
    }
    if (out.actions) out.actions = strip(out.actions);
    if (out.items) {
      out.items = out.items.map((item) =>
        item.actions ? { ...item, actions: strip(item.actions) } : item,
      );
    }
    if (out.emptyState && out.emptyState.actions) {
      out.emptyState = { ...out.emptyState, actions: strip(out.emptyState.actions) };
    }
    return out;
  };

  const fsCall = async (request) => JSON.parse(await __dangoNative.fs(JSON.stringify(request)));

  const storageCall = async (request) =>
    JSON.parse(await __dangoNative.storage(JSON.stringify(request)));

  globalThis.dango = {
    push(view) {
      __dangoNative.push(JSON.stringify(prepare(view)));
    },
    fail(message) {
      throw new DangoFailure(message);
    },
    env: __dangoNative.env ? JSON.parse(__dangoNative.env) : {},
    preferences: Object.freeze(
      __dangoNative.preferences ? JSON.parse(__dangoNative.preferences) : {},
    ),
    secret: async (name) => (await __dangoNative.secret(name)) ?? undefined,
    open: async (target) => __dangoNative.open(target),
    copy: async (text) => __dangoNative.copy(text),
    paste: async (text) => __dangoNative.paste(text),
    async fetch(url, init = {}) {
      const response = JSON.parse(await __dangoNative.fetch(JSON.stringify({ url, ...init })));
      return {
        status: response.status,
        ok: response.status >= 200 && response.status < 300,
        headers: response.headers,
        text: async () => response.body,
        json: async () => JSON.parse(response.body),
      };
    },
    fs: {
      readText: async (path) => fsCall({ op: "readText", path }),
      writeText: async (path, text) => fsCall({ op: "writeText", path, text }),
      readDir: async (path) => fsCall({ op: "readDir", path }),
      remove: async (path) => fsCall({ op: "remove", path }),
      exists: async (path) => fsCall({ op: "exists", path }),
      stat: async (path) => fsCall({ op: "stat", path }),
    },
    storage: {
      get: async (key) => storageCall({ op: "get", key }),
      set: async (key, value) => storageCall({ op: "set", key, value: JSON.stringify(value) }),
      remove: async (key) => storageCall({ op: "remove", key }),
      keys: async () => storageCall({ op: "keys" }),
    },
    async exec(program, args = [], options = {}) {
      return JSON.parse(await __dangoNative.exec(JSON.stringify({ program, args, ...options })));
    },
  };

  const write = (level) => (...parts) =>
    __dangoNative.log(
      level,
      parts.map((part) => (typeof part === "string" ? part : JSON.stringify(part))).join(" "),
    );
  globalThis.console = { log: write("log"), info: write("info"), warn: write("warn"), error: write("error") };

  // A new query aborts the search for the last one, whose answer is then
  // dropped even if the handler ignores the signal.
  globalThis.__dangoSearch = async (request) => {
    if (!search) return null;
    if (searching) searching.aborted = true;
    const signal = { aborted: false };
    searching = signal;
    const view = await search(request.query, { signal });
    if (signal.aborted || !view) return null;
    // The answer keeps searching through the same handler, or the launcher
    // would narrow it itself from the next keystroke on.
    return prepare(view.onSearch ? view : { ...view, onSearch: search });
  };

  // A handler from a view this session showed, or else the module's
  // `onAction`, which is how a root item's action reaches fresh code.
  globalThis.__dangoAction = async (request) => {
    const handler = handlers.get(request.actionId);
    const main = globalThis.__dangoMain;
    let level = null;
    const push = (view) => {
      level = __dangoNative.pushLevel(JSON.stringify(prepare(view)), level);
    };
    let result;
    if (handler) {
      result = await handler({ itemId: request.itemId, values: request.values, push });
    } else if (main && typeof main.onAction === "function") {
      result = await main.onAction(request.itemId, request.actionId, request.values, push);
    } else {
      return { missing: true };
    }
    if (result && typeof result === "object" && result.replace) {
      return { ...result, replace: prepare(result.replace) };
    }
    return result;
  };
})();
