import type { Shortcut } from "../protocol/Shortcut";
import type { ViewTree } from "../protocol/ViewTree";

export interface ActionDto {
  id: string;
  title: string;
  shortcut?: Shortcut | null;
}

export interface ResultItem {
  /// The extension that contributed the result; acting on it is dispatched
  /// back to that owner.
  extensionId: string;
  id: string;
  title: string;
  subtitle?: string | null;
  icon?: string | null;
  actions: ActionDto[];
  matchPositions: number[];
}

export interface ResultsPayload {
  query: string;
  items: ResultItem[];
  complete: boolean;
}

/// A pushed view with the extension whose command produced it. A command can be
/// started from its own hotkey, without the frontend ever asking for it, so the
/// owner arrives with the tree rather than from the invocation.
export interface RenderPayload {
  owner: string;
  tree: ViewTree;
}

export type ActionResponse =
  | { kind: "done" }
  | { kind: "copy"; text: string }
  /// The action changed what the view was showing, so it is replaced and the
  /// user stays where they are.
  | { kind: "replaced"; tree: ViewTree }
  | { kind: "removed"; tree: ViewTree }
  | { kind: "failed"; message: string };

/// True when the pressed key event matches a declared shortcut.
export function matchesShortcut(event: KeyboardEvent, shortcut: Shortcut): boolean {
  if (event.key.toLowerCase() !== shortcut.key.toLowerCase()) return false;
  const mods = shortcut.modifiers;
  const wantCtrl = mods.includes("ctrl") || mods.includes("cmd");
  const wantAlt = mods.includes("alt");
  const wantShift = mods.includes("shift");
  return (
    (event.ctrlKey || event.metaKey) === wantCtrl &&
    event.altKey === wantAlt &&
    event.shiftKey === wantShift
  );
}
