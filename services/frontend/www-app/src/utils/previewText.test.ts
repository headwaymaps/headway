import { expect, test } from 'vitest';
import { previewText } from './previewText';
test('short and exact-limit names remain complete', () => {
  expect(previewText('Cafe', 50)).toBe('Cafe');
  expect(previewText('x'.repeat(50), 50)).toBe('x'.repeat(50));
});
test('long names omit the partial final word', () => {
  const prefix = 'Nice '.repeat(9).trimEnd();
  expect(previewText(`${prefix} wonderful cafe`, 50)).toBe(`${prefix}…`);
});
test('a whole word at the boundary is preserved', () => {
  expect(previewText('hello world again', 11)).toBe('hello world…');
});
test('descriptions truncate at 140 without cutting the final word', () => {
  const prefix = 'quiet '.repeat(23).trimEnd();
  expect(previewText(`${prefix} wonderful space`, 140)).toBe(`${prefix}…`);
});
test('whitespace collapses to keep bubbles compact', () => {
  expect(previewText('  One\n\n two\tthree  ', 50)).toBe('One two three');
});
test('a word longer than the limit is omitted rather than split', () => {
  expect(previewText('x'.repeat(51), 50)).toBe('…');
});
test('Unicode code points are counted without breaking emoji', () => {
  expect(previewText('🌟 '.repeat(26).trim(), 50)).toBe(
    '🌟 '.repeat(25).trimEnd() + '…',
  );
});
