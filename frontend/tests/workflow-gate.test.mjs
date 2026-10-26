import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import test from 'node:test';

const repositoryRoot = path.resolve(import.meta.dirname, '../..');
const workflowDirectory = path.join(repositoryRoot, '.github', 'workflows');

function workflow(name) {
  return readFileSync(path.join(workflowDirectory, name), 'utf8');
}

function jobBlock(source, jobName) {
  const match = source.match(new RegExp(
    `^  ${jobName}:\\n([\\s\\S]*?)(?=^  [a-zA-Z0-9_-]+:|(?![\\s\\S]))`,
    'm',
  ));
  assert.ok(match, `missing job ${jobName}`);
  return match[0];
}

test('CI runs for pull requests and pushes to dev and main with read-only permissions', () => {
  const source = workflow('ci.yml');

  assert.match(source, /^permissions:\n  contents: read/m);
  assert.match(source, /pull_request:[\s\S]*branches:\s*\[dev, main\]/);
  assert.match(source, /push:[\s\S]*branches:\s*\[dev, main\]/);
  assert.match(source, /uses: \.\/\.github\/workflows\/quality-gate\.yml/);
});

test('reusable quality gate separates frontend, Rust data tests, production build, and fan-in', () => {
  const source = workflow('quality-gate.yml');

  assert.match(source, /workflow_call:/);
  assert.match(source, /^permissions:\n  contents: read/m);
  for (const job of ['source-integrity', 'frontend', 'rust-sqlite', 'macos-production', 'quality-gate']) {
    jobBlock(source, job);
  }

  const sourceIntegrity = jobBlock(source, 'source-integrity');
  assert.match(sourceIntegrity, /fetch-depth: 0/);
  assert.match(sourceIntegrity, /MEETILY_CHECK_BASE_REF/);
  assert.match(sourceIntegrity, /\.\/scripts\/check-whitespace\.sh/);

  const production = jobBlock(source, 'macos-production');
  assert.match(production, /needs: \[frontend, rust-sqlite\]/);
  assert.match(production, /pnpm --dir frontend.*build/);
  assert.match(production, /tauri.*build/);

  const fanIn = jobBlock(source, 'quality-gate');
  assert.match(fanIn, /needs: \[source-integrity, frontend, rust-sqlite, macos-production\]/);
  assert.match(fanIn, /if: always\(\)/);
  assert.match(fanIn, /needs\.source-integrity\.result/);
  assert.match(fanIn, /needs\.frontend\.result/);
  assert.match(fanIn, /needs\.rust-sqlite\.result/);
  assert.match(fanIn, /needs\.macos-production\.result/);
});

test('quality jobs install frozen frontend dependencies and run policy, migrations, and critical tests', () => {
  const source = workflow('quality-gate.yml');

  assert.match(source, /pnpm\/action-setup@v4[\s\S]*version: 10\.17\.1/);
  assert.match(source, /pnpm --dir frontend install --frozen-lockfile/);
  assert.match(source, /pnpm --dir frontend check:policy/);
  assert.match(source, /pnpm --dir frontend lint/);
  assert.match(source, /pnpm --dir frontend typecheck/);
  assert.match(source, /playwright test[\s\S]*interrupted-recovery\.spec\.ts/);
  assert.match(source, /cargo fmt --all -- --check/);
  assert.match(source, /cargo clippy --workspace --all-targets -- -D warnings/);
  assert.match(source, /cargo test --workspace/);
  assert.match(
    source,
    /cargo test -p meetily --test recording_recovery\s+successful_checkpoint_recovery_preserves_neighboring_meeting_files\s+-- --ignored/,
  );
  assert.match(source, /data_integrity/);
  assert.match(source, /diarization/);
  assert.match(source, /export_integrity/);
});

test('frontend CI requires the IndexedDB transcript persistence contract', () => {
  const frontend = jobBlock(workflow('quality-gate.yml'), 'frontend');

  assert.match(
    frontend,
    /playwright test[\s\S]*tests\/indexeddb-transcript-contract\.spec\.ts/,
  );
});

test('frontend CI requires local data safety and WebView egress contracts', () => {
  const frontend = jobBlock(workflow('quality-gate.yml'), 'frontend');

  assert.match(
    frontend,
    /playwright test[\s\S]*tests\/desktop-local-safety\.spec\.ts/,
  );
});

test('release draft requires both the reusable quality gate and exact manual attestation', () => {
  const source = workflow('release.yml');
  const createRelease = jobBlock(source, 'create-release');
  const attestation = jobBlock(source, 'manual-attestation');

  for (const input of ['manual-smoke-passed', 'smoke-report-reference', 'release-commit']) {
    assert.match(source, new RegExp(`${input}:`));
  }
  assert.match(source, /^permissions:\n  contents: read/m);
  assert.match(attestation, /needs: quality-gate/);
  assert.match(attestation, /github\.event\.inputs\['release-commit'\]/);
  assert.match(attestation, /github\.sha/);
  assert.match(createRelease, /needs: \[quality-gate, manual-attestation\]/);
  assert.match(createRelease, /draft: true/);
});

test('no workflow automatically publishes a GitHub release', () => {
  const sources = ['ci.yml', 'quality-gate.yml', 'release.yml']
    .map((name) => workflow(name))
    .join('\n');

  assert.doesNotMatch(sources, /draft:\s*false/);
  assert.doesNotMatch(sources, /make_latest|publish[-_ ]release|--draft=false/i);
  assert.equal((sources.match(/createRelease\s*\(/g) ?? []).length, 1);
});
