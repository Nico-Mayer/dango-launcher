import type { Action } from "../../src/protocol/Action";
import type { ViewTree } from "../../src/protocol/ViewTree";
import type { ResultItem } from "../../src/lib/types";

export const actions: Action[] = [
  { id: "open", title: "Open", shortcut: null },
  { id: "copy", title: "Copy", shortcut: { key: "c", modifiers: ["ctrl"] } },
];
export const longActions: Action[] = Array.from({ length: 24 }, (_, index) => ({
  id: `action-${index + 1}`,
  title: `Open action ${index + 1}`,
  shortcut: null,
}));

export const rootItems: ResultItem[] = Array.from({ length: 18 }, (_, index) => ({
  extensionId: "fixture",
  id: `result-${index + 1}`,
  title: index === 0 ? "Clipboard History" : `Result ${index + 1}`,
  subtitle: index === 0 ? "Search copied text and images" : "Fixture command",
  icon: "icon:clipboard-list",
  tint: ["blue", "green", "amber", "purple", "red", "pink"][index % 6],
  actions,
  matchPositions: index === 0 ? [0, 1, 2, 3] : [],
  suggested: index < 3,
}));

export const listTree: ViewTree = {
  protocolVersion: 1,
  view: {
    kind: "list",
    filtering: "launcher",
    loading: false,
    emptyState: { title: "Nothing to show", description: null },
    items: rootItems.map(({ id, title, subtitle, icon }) => ({
      id, title, subtitle: subtitle ?? null, icon: icon ?? null, actions,
    })),
  },
};

export const detailTree: ViewTree = {
  protocolVersion: 1,
  view: { kind: "detail", markdown: "Working…", loading: true, actions },
};

export const formTree: ViewTree = {
  protocolVersion: 1,
  view: {
    kind: "form",
    itemId: "fixture-form",
    fields: [
      { id: "name", label: "Name", kind: "text", value: "Morning note" },
      { id: "password", label: "Password", kind: "password", value: "fixture-only" },
      { id: "enabled", label: "Enabled", kind: "toggle", value: "true" },
      { id: "template", label: "Template", kind: "template", value: "Hello {{name}}" },
    ],
    actions: [{ id: "save", title: "Save", shortcut: null }, ...actions],
  },
};

export const fixtureNames = [
  "root", "list", "empty-list", "loading-list", "refreshing-list", "detail",
  "form", "template", "failure", "long-actions",
] as const;
export type FixtureName = (typeof fixtureNames)[number];

export function treeFor(name: FixtureName): ViewTree | null {
  if (name === "list") return structuredClone(listTree);
  if (["empty-list", "loading-list", "refreshing-list"].includes(name)) {
    const tree = structuredClone(listTree);
    if (tree.view.kind === "list") {
      tree.view.loading = name !== "empty-list";
      if (name !== "refreshing-list") tree.view.items = [];
    }
    return tree;
  }
  if (name === "detail") return structuredClone(detailTree);
  if (name === "form" || name === "template") {
    const tree = structuredClone(formTree);
    if (name === "template" && tree.view.kind === "form")
      tree.view.fields = tree.view.fields.filter((field) => field.kind === "template");
    return tree;
  }
  return null;
}
