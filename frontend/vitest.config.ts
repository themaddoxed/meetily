import { fileURLToPath, URL } from 'node:url';

import { defineConfig } from 'vitest/config';

export default defineConfig({
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  test: {
    clearMocks: true,
    environment: 'node',
    include: ['tests/lib/**/*.test.{ts,js}'],
    restoreMocks: true,
    watch: false,
  },
});
