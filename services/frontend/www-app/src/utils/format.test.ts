import { describe, expect, test } from 'vitest';
import { formatDuration } from './format';

describe('formatDuration', () => {
  test('a single minute is singular', () => {
    expect(formatDuration(60)).toEqual('1 minute');
    expect(formatDuration(120)).toEqual('2 minutes');
  });
});
