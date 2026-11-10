/** Shorten popup text at a word boundary, counting Unicode code points. */
export function previewText(text: string, limit: number): string {
  const normalized = text.replace(/\s+/gu, ' ').trim();
  const characters = Array.from(normalized);
  if (characters.length <= limit) return normalized;
  const prefix = characters.slice(0, limit).join('');
  // Omit an oversized word rather than splitting it.
  const boundary = prefix.lastIndexOf(' ');
  const snippet = /\s/u.test(characters[limit]!)
    ? prefix
    : boundary < 0
      ? ''
      : prefix.slice(0, boundary);
  return `${snippet.trimEnd()}…`;
}
