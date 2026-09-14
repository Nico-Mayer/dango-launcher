/// Every tint an extension can name, mapped to the classes that draw it. The
/// pairs are written out rather than assembled, because Tailwind generates a
/// class only when it finds the literal string in the source.
const REGISTRY = {
  red: "bg-tint-red text-tint-red-foreground",
  amber: "bg-tint-amber text-tint-amber-foreground",
  green: "bg-tint-green text-tint-green-foreground",
  blue: "bg-tint-blue text-tint-blue-foreground",
  purple: "bg-tint-purple text-tint-purple-foreground",
  pink: "bg-tint-pink text-tint-pink-foreground",
} as const;

export type TintName = keyof typeof REGISTRY;

/// The classes for a tint, or nothing for an extension that named none and for
/// a name this build's palette does not hold.
export function tintClasses(name: string | null | undefined): string {
  return name && name in REGISTRY ? REGISTRY[name as TintName] : "";
}
