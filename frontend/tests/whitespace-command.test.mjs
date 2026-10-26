import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';

const checker = path.resolve(import.meta.dirname, '../../scripts/check-whitespace.sh');
const zero = '0'.repeat(40);

function inRepository(content, run) {
  const cwd = mkdtempSync(path.join(os.tmpdir(), 'meetily-whitespace-'));
  const git = (...args) => execFileSync('git', args, { cwd, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim();
  const commit = () => {
    git('add', '.');
    git('-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid',
      '-c', 'commit.gpgsign=false', 'commit', '-m', 'fixture');
    return git('rev-parse', 'HEAD');
  };
  const check = (base) => {
    const env = { ...process.env };
    delete env.MEETILY_CHECK_BASE_REF;
    if (base !== undefined) env.MEETILY_CHECK_BASE_REF = base;
    const result = spawnSync(checker, [], { cwd, env, encoding: 'utf8' });
    return { ...result, output: result.stdout + result.stderr };
  };
  try {
    git('init', '--initial-branch=fixture');
    writeFileSync(path.join(cwd, 'source.txt'), content);
    commit();
    run({ cwd, git, commit, check });
  } finally {
    rmSync(cwd, { recursive: true, force: true });
  }
}

test('an initial push checks a clean root against the empty tree', () => {
  inRepository('clean\n', ({ check }) => {
    const result = check(zero);
    assert.equal(result.status, 0, result.output);
    assert.match(result.output, /whole committed tree/);
  });
});

test('an initial push still rejects whitespace already committed in the root', () => {
  inRepository('bad \n', ({ check }) => {
    const result = check(zero);
    assert.notEqual(result.status, 0);
    assert.match(result.output, /source\.txt:1: trailing whitespace/);
  });
});

test('a new branch push checks the entire tree, including earlier commits', () => {
  inRepository('bad \n', ({ cwd, commit, check }) => {
    writeFileSync(path.join(cwd, 'other.txt'), 'clean\n');
    commit();
    const result = check(zero);
    assert.notEqual(result.status, 0);
    assert.match(result.output, /source\.txt:1: trailing whitespace/);
  });
});

test('a standalone root without a target ref can be checked locally', () => {
  inRepository('clean\n', ({ check }) => {
    const result = check();
    assert.equal(result.status, 0, result.output);
  });
});

test('an invalid nonzero base fails closed', () => {
  inRepository('clean\n', ({ check }) => {
    const result = check('missing-base');
    assert.equal(result.status, 2);
    assert.match(result.output, /base does not exist/);
  });
});

test('a normal push rejects whitespace added after its valid base', () => {
  inRepository('clean\n', ({ cwd, git, commit, check }) => {
    const base = git('rev-parse', 'HEAD');
    writeFileSync(path.join(cwd, 'source.txt'), 'bad \n');
    commit();
    const result = check(base);
    assert.notEqual(result.status, 0);
    assert.match(result.output, /trailing whitespace/);
  });
});
