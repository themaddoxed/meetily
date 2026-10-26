import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import test from 'node:test';

const source = readFileSync(
  path.resolve(import.meta.dirname, '../playwright.config.ts'),
  'utf8',
);

test('browser failures retain a first-attempt trace even when retries are disabled', () => {
  assert.match(source, /trace:\s*['"]retain-on-first-failure['"]/);
  assert.doesNotMatch(source, /trace:\s*['"]on-first-retry['"]/);
});

test('existing development servers are reused only through an explicit opt-in', () => {
  assert.match(
    source,
    /reuseExistingServer:\s*process\.env\.PLAYWRIGHT_REUSE_EXISTING_SERVER\s*===\s*['"]1['"]/,
  );
});

test('the E2E flag is passed through webServer.env rather than shell syntax', () => {
  assert.match(
    source,
    /command:\s*['"]pnpm dev --hostname 127\.0\.0\.1['"]/,
  );
  assert.match(
    source,
    /env:\s*\{[\s\S]*NEXT_PUBLIC_E2E_TESTING:\s*['"]1['"][\s\S]*\}/,
  );
  assert.doesNotMatch(source, /command:\s*['"]NEXT_PUBLIC_E2E_TESTING=/);
});
