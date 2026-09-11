import type { Shortcut } from "../protocol/Shortcut";

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

export type ActionResponse =
  | { kind: "done" }
  | { kind: "copy"; text: string }
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
