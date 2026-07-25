import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './examples-wasm/provider-smoke',
  testMatch: 'browser.spec.mjs',
  timeout: 30_000,
  fullyParallel: false,
  workers: 1,
  use: {
    browserName: 'chromium',
    headless: true,
  },
  reporter: [['list']],
});
