import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { gzipSync } from 'node:zlib';
import { mkdirSync, mkdtempSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';

const scanner = path.resolve(import.meta.dirname, '../scripts/check-publication-privacy.mjs');

function fixture(contents, run, identity = ['Fixture', 'fixture@example.invalid']) {
  const cwd = mkdtempSync(path.join(os.tmpdir(), 'meetily-privacy-'));
  const git = (...args) => execFileSync('git', args, { cwd, stdio: 'pipe' });
  try {
    git('init', '--initial-branch=fixture');
    writeFileSync(path.join(cwd, 'evidence.txt'), contents);
    git('add', '.');
    git('-c', `user.name=${identity[0]}`, '-c', `user.email=${identity[1]}`,
      '-c', 'commit.gpgsign=false', 'commit', '-m', 'fixture');
    const check = () => {
      const result = spawnSync(process.execPath, [scanner, '--root', cwd], { encoding: 'utf8' });
      return { ...result, output: result.stdout + result.stderr };
    };
    run({ cwd, check, git });
  } finally {
    rmSync(cwd, { recursive: true, force: true });
  }
}

test('generic homes and neutral commit metadata are publishable', () => {
  fixture('/Users/developer/project\ncontact@example.invalid\n', ({ check }) => {
    const result = check();
    assert.equal(result.status, 0, result.output);
  });
});

test('a personal home path is rejected without echoing the private value', () => {
  fixture('/Users/fictional-person/project\n', ({ check }) => {
    const result = check();
    assert.equal(result.status, 1);
    assert.match(result.output, /privacy.personal-home/);
    assert.doesNotMatch(result.output, /fictional-person/);
  });
});

test('a private contact address in a newly added file is rejected', () => {
  fixture('safe\n', ({ cwd, check }) => {
    writeFileSync(path.join(cwd, 'new.txt'), 'fictional-person@private.invalid\n');
    const result = check();
    assert.equal(result.status, 1);
    assert.match(result.output, /privacy.contact-email/);
    assert.doesNotMatch(result.output, /fictional-person@private.invalid/);
  });
});

test('gzip logs are decoded before checking contact fields', () => {
  fixture('safe\n', ({ cwd, check }) => {
    writeFileSync(path.join(cwd, 'log.gz'), gzipSync('fictional-person@private.invalid\n'));
    const result = check();
    assert.equal(result.status, 1);
    assert.match(result.output, /privacy.contact-email: log.gz/);
  });
});

test('private Git identities are caught even when source files are clean', () => {
  fixture('safe\n', ({ check }) => {
    const result = check();
    assert.equal(result.status, 1);
    assert.match(result.output, /privacy.commit-identity/);
    assert.doesNotMatch(result.output, /Fictional Person|fictional-person@private.invalid/);
  }, ['Fictional Person', 'fictional-person@private.invalid']);
});

test('a binary runtime database is rejected before the text-only scan', () => {
  fixture('safe\n', ({ cwd, check }) => {
    writeFileSync(path.join(cwd, 'meeting.db'), Buffer.from('SQLite format 3\0data'));
    const result = check();
    assert.equal(result.status, 1);
    assert.match(result.output, /privacy.runtime-data: meeting.db/);
  });
});

test('a dangling tracked-source symlink fails closed without following it', () => {
  fixture('safe\n', ({ cwd, check }) => {
    symlinkSync(path.join(cwd, 'missing'), path.join(cwd, 'link.txt'));
    const result = check();
    assert.equal(result.status, 1);
    assert.match(result.output, /privacy.symlink: link.txt/);
  });
});

test('an email on a numeric-leading domain is not a dependency identifier', () => {
  fixture('fixture@123-provider.invalid\n', ({ check }) => {
    const result = check();
    assert.equal(result.status, 1);
    assert.match(result.output, /privacy.contact-email/);
  });
});

test('numeric-leading domains cannot bypass Git email checks', () => {
  fixture('safe\n', ({ check }) => {
    const result = check();
    assert.equal(result.status, 1);
    assert.match(result.output, /privacy.commit-identity/);
  }, ['Fixture', 'fixture@123-provider.invalid']);
});

for (const home of ['/Users/fictional-person', 'C:\\Users\\fictional-person']) {
  test(`a home without a child path is rejected (${home.startsWith('/') ? 'POSIX' : 'Windows'})`, () => {
    fixture(home + '\n', ({ check }) => {
      const result = check();
      assert.equal(result.status, 1);
      assert.match(result.output, /privacy.personal-home/);
    });
  });
}

test('known retina asset references are filenames, not a blanket email exception', () => {
  fixture('![icon@2x.png](assets/icon@2x.png)\n', ({ cwd, check }) => {
    mkdirSync(path.join(cwd, 'assets'));
    writeFileSync(path.join(cwd, 'assets/icon@2x.png'), Buffer.from([0, 1, 2]));
    assert.equal(check().status, 0);
    writeFileSync(path.join(cwd, 'contact.txt'), 'Contact: icon@2x.png\n');
    assert.equal(check().status, 1);
  });
});
