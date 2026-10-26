import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import test from 'node:test';

const repositoryRoot = path.resolve(import.meta.dirname, '../..');
const metadataResult = spawnSync(
  'cargo',
  ['metadata', '--locked', '--offline', '--no-deps'],
  {
    cwd: repositoryRoot,
    encoding: 'utf8',
  },
);

function cargoMetadata() {
  assert.equal(
    metadataResult.status,
    0,
    `${metadataResult.stdout}${metadataResult.stderr}`,
  );
  return JSON.parse(metadataResult.stdout);
}

function lockedPackage(lockfile, name, version) {
  const escapedName = name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const escapedVersion = version.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const match = lockfile.match(
    new RegExp(
      `\\[\\[package\\]\\]\\nname = "${escapedName}"\\nversion = "${escapedVersion}"\\n[\\s\\S]*?(?=\\n\\[\\[package\\]\\]|$)`,
    ),
  );

  assert.ok(match, `Cargo.lock is missing ${name} ${version}`);
  return match[0];
}

test('workspace members omit Cargo sections that are only honored at the root', () => {
  const metadata = cargoMetadata();
  const findings = metadata.packages.flatMap(({ manifest_path: manifestPath }) => {
    const manifest = readFileSync(manifestPath, 'utf8');
    const sections = manifest.match(/^[ \t]*\[(?:patch|profile)(?:\.[^\]]+)?\][ \t]*(?:#.*)?$/gm) ?? [];
    return sections.map((section) => ({
      manifest: path.relative(repositoryRoot, manifestPath),
      section,
    }));
  });

  assert.deepEqual(findings, []);
});

test('root cargo metadata has no ignored non-root configuration warnings', () => {
  cargoMetadata();
  assert.doesNotMatch(
    metadataResult.stderr,
    /warning: (?:patch|profiles?) for the non[- ]root package will be ignored/,
  );
});

test('ignored member patches do not change the locked registry sources', () => {
  const lockfile = readFileSync(path.join(repositoryRoot, 'Cargo.lock'), 'utf8');

  for (const [name, version] of [
    ['cpal', '0.15.3'],
    ['esaxx-rs', '0.1.10'],
  ]) {
    const packageRecord = lockedPackage(lockfile, name, version);
    assert.match(packageRecord, /^source = "registry\+[^"]+"$/m, `${name} source`);
    assert.doesNotMatch(packageRecord, /^source = "git\+/m, `${name} source`);
  }
});
