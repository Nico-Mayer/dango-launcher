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
let gesture: Event | null = null;

function markGesture(event: Event) {
  gesture = event;
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
    (event) => {
      active = false;
      markGesture(event);
    },
    true,
  );

  window.addEventListener("pointerdown", markGesture, true);
  window.addEventListener("click", markGesture, true);
}

export function pointerActive(): boolean {
  return active;
}

// Bits calls onValueChange synchronously for input, but re-sorts after a tick.
// Binding writes flush later too, so only the synchronous callback may pick.
export function inUserGesture(): boolean {
  return gesture !== null && gesture.eventPhase !== Event.NONE;
}
