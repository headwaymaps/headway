import vue from '@vitejs/plugin-vue';
import { defineConfig } from 'vitest/config';
import { fileURLToPath } from 'node:url';

// Mirror the `src/*` path alias from tsconfig.json. (This is the only alias the
// non-test source actually imports; add more here if that changes.)
const srcDir = fileURLToPath(new URL('./src', import.meta.url));

export default defineConfig({
  // Some units under test reach a `.vue` component through their imports.
  plugins: [vue()],
  resolve: {
    alias: [{ find: /^src\//, replacement: `${srcDir}/` }],
  },
  test: {
    // Matches the previous Jest environment; these are pure-logic unit tests.
    environment: 'node',
    include: ['src/**/*.test.ts'],
  },
});
