(() => {
  const handlers = new Map();
  let next = 0;

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

  globalThis.dango = {
    push(view) {
      __dangoNative.push(JSON.stringify(prepare(view)));
    },
    fail(message) {
      throw new DangoFailure(message);
    },
    env: __dangoNative.env ? JSON.parse(__dangoNative.env) : {},
    open: (target) => __dangoNative.open(target),
    copy: (text) => __dangoNative.copy(text),
    paste: (text) => __dangoNative.paste(text),
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

  globalThis.__dangoPrepare = prepare;
  globalThis.__dangoHasHandler = (actionId) => handlers.has(actionId);
  globalThis.__dangoAction = (request) =>
    handlers.get(request.actionId)({ itemId: request.itemId, values: request.values });
})();
