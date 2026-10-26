import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import {
  mkdtempSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';

const repositoryRoot = path.resolve(import.meta.dirname, '../..');
const fingerprintScript = path.join(
  repositoryRoot,
  'frontend',
  'scripts',
  'source-fingerprint.mjs',
);

function git(root, args) {
  const result = spawnSync('git', args, { cwd: root, encoding: 'utf8' });
  assert.equal(result.status, 0, result.stderr);
}

function fingerprint(root) {
  const result = spawnSync(process.execPath, [
    fingerprintScript,
    '--repository',
    root,
  ], { encoding: 'utf8' });
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /^[a-f0-9]{64}\n$/);
  return result.stdout.trim();
}

test('source fingerprint covers tracked diff and untracked path/content without exposing content', () => {
  const root = mkdtempSync(path.join(os.tmpdir(), 'meetily-source-fingerprint-'));
  try {
    git(root, ['init', '--quiet']);
    writeFileSync(path.join(root, 'tracked.txt'), 'committed\n');
    git(root, ['add', 'tracked.txt']);
    git(root, [
      '-c', 'user.name=Meetily Test',
      '-c', 'user.email=meetily-test@example.invalid',
      'commit', '--quiet', '-m', 'fixture',
    ]);

    writeFileSync(path.join(root, 'tracked.txt'), 'private tracked change one\n');
    writeFileSync(path.join(root, 'untracked-a.txt'), 'private untracked content one\n');
    const first = fingerprint(root);

    writeFileSync(path.join(root, 'tracked.txt'), 'private tracked change two\n');
    const trackedChanged = fingerprint(root);
    assert.notEqual(trackedChanged, first);

    writeFileSync(path.join(root, 'untracked-a.txt'), 'private untracked content two\n');
    const untrackedContentChanged = fingerprint(root);
    assert.notEqual(untrackedContentChanged, trackedChanged);

    writeFileSync(path.join(root, 'untracked-b.txt'), 'private untracked content two\n');
    const untrackedPathChanged = fingerprint(root);
    assert.notEqual(untrackedPathChanged, untrackedContentChanged);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
