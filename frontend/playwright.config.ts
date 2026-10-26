import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './tests',
  testMatch: '**/*.spec.ts',
  timeout: 30_000,
  expect: {
    timeout: 5_000,
  },
  use: {
    baseURL: 'http://127.0.0.1:3118',
    viewport: { width: 1440, height: 1000 },
    trace: 'retain-on-first-failure',
    screenshot: 'only-on-failure',
  },
  projects: [
    {
      name: 'chromium',
      use: {
        ...devices['Desktop Chrome'],
        viewport: { width: 1440, height: 1000 },
      },
    },
  ],
  webServer: {
    command: 'pnpm dev --hostname 127.0.0.1',
    env: {
      NEXT_PUBLIC_E2E_TESTING: '1',
    },
    url: 'http://127.0.0.1:3118',
    reuseExistingServer:
      process.env.PLAYWRIGHT_REUSE_EXISTING_SERVER === '1',
    timeout: 120_000,
  },
});
