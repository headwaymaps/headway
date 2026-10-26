// Screenshots each map view (`zoom/lat/lng`, or a path) from the dev server on :9000 to <out>-<i>.png.
// Usage: node screenshot.mjs <out> <view>... (needs playwright-core and its headless chromium)
import { chromium } from 'playwright-core';
const [out, ...views] = process.argv.slice(2);
const browser = await chromium.launch({ args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
for (const [i, view] of views.entries()) {
  await page.goto(view.startsWith('/') ? `http://127.0.0.1:9000${view}` : `http://127.0.0.1:9000/#map=${view}`);
  await page.waitForTimeout(6000);
  await page.screenshot({ path: `${out}-${i}.png` });
}
await browser.close();
