import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    // The page is a DOM program: what's worth testing about it is what it puts
    // on screen and what it asks the server for, so the tests drive real
    // elements and events rather than calling functions the page doesn't export.
    environment: 'jsdom',
    include: ['tests/**/*.test.mjs'],
  },
});
