import { defineConfig } from 'vitest/config';
import { fileURLToPath } from 'node:url';

// Mirror the `src/*` path alias from tsconfig.json. (This is the only alias the
// non-test source actually imports; add more here if that changes.)
const srcDir = fileURLToPath(new URL('./src', import.meta.url));

export default defineConfig({
  resolve: {
    alias: [{ find: /^src\//, replacement: `${srcDir}/` }],
  },
  test: {
    // Matches the previous Jest environment; these are pure-logic unit tests.
    environment: 'node',
    include: ['src/**/*.test.ts'],
  },
});
