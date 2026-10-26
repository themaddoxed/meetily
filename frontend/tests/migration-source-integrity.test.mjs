import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import test from 'node:test';

test('the published SQLx migration keeps the bytes used by existing database checksums', () => {
  const file = path.resolve(import.meta.dirname,
    '../src-tauri/migrations/20250920155811_add_openrouter_api_key.sql');
  const digest = createHash('sha256').update(readFileSync(file)).digest('hex');
  assert.equal(digest, '96839c8dd47fa1de98595deff59080efcb850fd5222cfb2bd659aabd71f381d5',
    'Do not reformat an applied SQLx migration; add a new migration for schema changes.');
});
