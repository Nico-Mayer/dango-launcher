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

  globalThis.dango = {
    push(view) {
      __dangoNative.push(JSON.stringify(prepare(view)));
    },
    fail(message) {
      throw new DangoFailure(message);
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
