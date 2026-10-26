import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import test from 'node:test';

const repositoryRoot = path.resolve(import.meta.dirname, '../..');
const guidePath = path.join(repositoryRoot, 'docs/testing.md');

test('testing guide documents targeted check routes and their command boundaries', () => {
  const guide = readFileSync(guidePath, 'utf8');

  for (const command of [
    './scripts/check.sh --docs',
    './scripts/check.sh --audio',
    './scripts/check.sh --summary',
    './scripts/check.sh --quick',
    './scripts/check.sh --full',
  ]) {
    assert.equal(guide.includes(command), true, command + ' must be documented');
  }

  assert.match(guide, /--docs[\s\S]*pnpm test:docs/);
  assert.match(guide, /cargo test -p meetily --lib audio::/);
  assert.match(guide, /cargo test -p meetily --lib summary::/);
  assert.match(guide, /не запускает browser, Rust[\s\S]*production build/i);
  assert.match(guide, /ноль совпавших тестов[\s\S]*ошибка/i);
});
