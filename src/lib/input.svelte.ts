/// Which input device the user is driving, for the two questions a list asks of
/// it: may hover paint, and did this selection change come from the user.
///
/// Hover painting:
///
/// A resting cursor keeps `:hover` alive while the keyboard drives the list, so
/// a highlight sits on one row while the selection moves down another. Any key
/// press therefore silences hover until the pointer genuinely moves again:
/// identical coordinates mean the list scrolled under the cursor rather than
/// the user moving it, which is what arrowing through a list produces.
///
/// This only decides what is painted. Hover never changes the selection.
let active = $state(false);
let lastX = Number.NaN;
let lastY = Number.NaN;
let gesture = false;

function markGesture() {
  gesture = true;
  // Cleared on the next task rather than the next microtask: Svelte delivers a
  // component binding's write through its effect flush, which is a microtask,
  // so a microtask reset lands before the write it is meant to cover.
  setTimeout(() => {
    gesture = false;
  }, 0);
}

if (typeof window !== "undefined") {
  window.addEventListener(
    "pointermove",
    (event) => {
      if (event.clientX === lastX && event.clientY === lastY) return;
      lastX = event.clientX;
      lastY = event.clientY;
      active = true;
    },
    true,
  );

  window.addEventListener(
    "keydown",
    () => {
      active = false;
      markGesture();
    },
    true,
  );

  window.addEventListener("pointerdown", markGesture, true);
  window.addEventListener("click", markGesture, true);
}

export function pointerActive(): boolean {
  return active;
}

/// Whether a key press or a click is being handled at this moment.
///
/// The Command primitive re-selects the first row whenever it re-sorts its
/// items, and it re-sorts every time results are replaced, which would drag the
/// selection back to the top while results stream in for the same query. Its
/// writes are only trusted while the user is actually pressing something; the
/// re-sort runs on its own in a later task, with no gesture in flight.
export function inUserGesture(): boolean {
  return gesture;
}
