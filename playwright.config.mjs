import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './examples-wasm',
  timeout: 60_000,
  fullyParallel: false,
  workers: 1,
  use: {
    browserName: 'chromium',
    headless: true,
  },
  projects: [
    {
      name: 'provider-smoke',
      testMatch: '**/provider-smoke/browser.spec.mjs',
      timeout: 30_000,
    },
    {
      name: 'pages-smoke',
      testMatch: '**/pages-smoke/browser.spec.mjs',
      timeout: 90_000,
    },
  ],
  reporter: [['list']],
});
