import { defineConfig, devices } from '@playwright/test';

// Points at an already-running stack, like tests/integration/run-tests.sh does.
// Start one with: bin/start-services --no-follow-logs builds/Bogota
const baseURL = process.env.FRONTEND_URL ?? 'http://localhost:8080';

export default defineConfig({
  testDir: './specs',
  // Tiles, geocoding and routing all cross the network into real services, and
  // a cold Valhalla/OTP request is not fast. These are e2e tests, not unit
  // tests: be patient rather than flaky.
  timeout: 120_000,
  expect: { timeout: 30_000 },
  // One worker: every test shares the single running stack, and several of them
  // assert on map camera state persisted in localStorage.
  workers: 1,
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: [['list'], ['html', { open: 'never' }]],
  use: {
    baseURL,
    // Real Google Chrome, not bundled Chromium.
    channel: 'chrome',
    viewport: { width: 1280, height: 900 },
    // The map is a WebGL canvas; keep artifacts for anything that fails.
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
    trace: 'retain-on-failure',
  },
  projects: [{ name: 'chrome', use: { ...devices['Desktop Chrome'] } }],
});
