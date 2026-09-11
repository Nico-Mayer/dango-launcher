export interface Segment {
  text: string;
  matched: boolean;
}

/// Splits a title into matched and unmatched runs from the ranker's character
/// offsets, so the frontend can bold the matched characters.
export function highlight(title: string, positions: number[]): Segment[] {
  const chars = [...title];
  const marked = new Set(positions);
  const segments: Segment[] = [];
  for (let i = 0; i < chars.length; i++) {
    const matched = marked.has(i);
    const last = segments[segments.length - 1];
    if (last && last.matched === matched) {
      last.text += chars[i];
    } else {
      segments.push({ text: chars[i], matched });
    }
  }
  return segments;
}
