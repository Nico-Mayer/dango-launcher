/// Whether hover may paint.
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
    },
    true,
  );
}

export function pointerActive(): boolean {
  return active;
}
