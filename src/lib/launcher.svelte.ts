import { invoke } from "@tauri-apps/api/core";
import { untrack } from "svelte";
import type { Binding } from "../protocol/Binding";
import type { Chord } from "../protocol/Chord";
import type { Intent } from "../protocol/Intent";
import type { Text } from "../protocol/Text";

export function dispatch(intent: Intent) {
  return invoke("dispatch", { intent });
}

/// The chord for this key if the launcher declared it. `preventDefault` has to
/// be decided synchronously, so the table in the snapshot answers, not Rust.
/// Anything else stays with the focused control.
export function boundChord(bindings: Binding[], event: KeyboardEvent): Chord | null {
  const chord: Chord = {
    key: event.key.toLowerCase(),
    command: event.ctrlKey || event.metaKey,
    alt: event.altKey,
    shift: event.shiftKey,
  };
  const inMultiline = event.target instanceof HTMLTextAreaElement;
  const hit = bindings.some(({ chord: bound, inMultiline: anywhere }) =>
    bound.key === chord.key && bound.command === chord.command && bound.alt === chord.alt
      && bound.shift === chord.shift && (anywhere || !inMultiline));
  return hit ? chord : null;
}

/// A text field's value that the user owns while typing. The snapshot's text
/// is taken only when its revision moves, so an answer to an older keystroke
/// never overwrites newer typing or jumps the caret.
export function echoed(source: () => Text) {
  const revision = $derived(source().revision);
  let text = $derived.by(() => {
    void revision;
    return untrack(() => source().text);
  });
  return {
    get text() {
      return text;
    },
    set text(value: string) {
      text = value;
    },
  };
}
