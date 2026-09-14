export const isMac =
  typeof navigator !== "undefined" && /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent);

export const modKey = isMac ? "⌘" : "Ctrl";

export function shortcutLabel(key: string, modifiers: string[]): string {
  const parts = modifiers.map((m) => {
    if (m === "cmd" || m === "ctrl") return modKey;
    if (m === "alt") return isMac ? "⌥" : "Alt";
    if (m === "shift") return isMac ? "⇧" : "Shift";
    return m;
  });
  parts.push(key.toUpperCase());
  return parts.join(isMac ? "" : "+");
}
