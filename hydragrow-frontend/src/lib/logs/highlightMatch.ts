// hydragrow-frontend/src/lib/logs/highlightMatch.ts

export interface MatchSegment {
  text: string;
  matched: boolean;
}

/** Tách 1 chuỗi thành các đoạn khớp/không khớp với query, không phân biệt hoa/thường — dùng để tô đậm kết quả tìm kiếm (search-ux skill). */
export function splitByMatch(text: string, query: string): MatchSegment[] {
  const q = query.trim();
  if (!q) return [{ text, matched: false }];

  const idx = text.toLowerCase().indexOf(q.toLowerCase());
  if (idx === -1) return [{ text, matched: false }];

  const segments: MatchSegment[] = [];
  if (idx > 0) segments.push({ text: text.slice(0, idx), matched: false });
  segments.push({ text: text.slice(idx, idx + q.length), matched: true });
  if (idx + q.length < text.length) segments.push({ text: text.slice(idx + q.length), matched: false });
  return segments;
}
