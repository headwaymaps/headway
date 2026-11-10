import { afterEach, expect, test, vi } from 'vitest';
import { createUuid } from './uuid';

afterEach(() => vi.unstubAllGlobals());

test('prefers the native UUID generator when available', () => {
  const randomUUID = vi.fn(() => '12345678-1234-4234-8234-123456789abc');
  const getRandomValues = vi.fn();
  vi.stubGlobal('crypto', { randomUUID, getRandomValues });
  expect(createUuid()).toBe('12345678-1234-4234-8234-123456789abc');
  expect(randomUUID).toHaveBeenCalledOnce();
  expect(getRandomValues).not.toHaveBeenCalled();
});

test.each([0, 255])(
  'fallback sets UUID version and variant bits with random bytes %i',
  (value) => {
    const getRandomValues = vi.fn((bytes: Uint8Array) => bytes.fill(value));
    vi.stubGlobal('crypto', { getRandomValues });
    expect(createUuid()).toBe(
      value === 0
        ? '00000000-0000-4000-8000-000000000000'
        : 'ffffffff-ffff-4fff-bfff-ffffffffffff',
    );
    expect(getRandomValues).toHaveBeenCalledOnce();
    expect(getRandomValues.mock.calls[0]![0]).toHaveLength(16);
  },
);

test('fallback retains random byte order and formats leading zeros', () => {
  vi.stubGlobal('crypto', {
    getRandomValues: (bytes: Uint8Array) => {
      bytes.set(Array.from({ length: 16 }, (_, index) => index));
      return bytes;
    },
  });
  expect(createUuid()).toBe('00010203-0405-4607-8809-0a0b0c0d0e0f');
});

test('fallback requests fresh randomness for each ID', () => {
  let value = 0;
  const getRandomValues = vi.fn((bytes: Uint8Array) => bytes.fill(++value));
  vi.stubGlobal('crypto', { getRandomValues });
  expect(createUuid()).not.toBe(createUuid());
  expect(getRandomValues).toHaveBeenCalledTimes(2);
});
