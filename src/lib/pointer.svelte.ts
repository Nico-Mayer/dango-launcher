/// Which input device currently owns the selection.
///
/// Keyboard and mouse both want to move it, and they fight: arrowing through a
/// list scrolls it, that slides an item under a resting cursor, and the browser
/// synthesises a pointer event at the unchanged coordinates. Treating that as
/// hover drags the selection straight back off the keyboard.
///
/// So the pointer only takes ownership when it genuinely moves, and any key
/// press hands ownership back. A component asks `pointerOwnsSelection()` before
/// letting hover select anything.
let pointerOwns = $state(false);
let lastX = Number.NaN;
let lastY = Number.NaN;

if (typeof window !== "undefined") {
  window.addEventListener(
    "pointermove",
    (event) => {
      // Identical coordinates mean the cursor did not move; the list moved
      // under it.
      if (event.clientX === lastX && event.clientY === lastY) return;
      lastX = event.clientX;
      lastY = event.clientY;
      pointerOwns = true;
    },
    true,
  );

  window.addEventListener(
    "keydown",
    () => {
      pointerOwns = false;
    },
    true,
  );
}

export function pointerOwnsSelection(): boolean {
  return pointerOwns;
}
