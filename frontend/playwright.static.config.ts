import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './tests',
  testMatch: 'universal-build-config.spec.ts',
  projects: [{ name: 'static' }],
});
