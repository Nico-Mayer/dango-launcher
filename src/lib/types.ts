import type { Action } from "../protocol/Action";
import type { Shortcut } from "../protocol/Shortcut";

export type { ActionResponse } from "../protocol/ActionResponse";
export type { ActivatePayload } from "../protocol/ActivatePayload";
export type { RenderPayload } from "../protocol/RenderPayload";
export type { ResultItem } from "../protocol/ResultItem";
export type { ResultsPayload } from "../protocol/ResultsPayload";
export type ActionDto = Action;

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
