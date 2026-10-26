import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import {
  chmodSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';

const repositoryRoot = path.resolve(import.meta.dirname, '../..');
const checkScript = path.join(repositoryRoot, 'scripts/check.sh');

function withStubbedCommands(run) {
  const directory = mkdtempSync(path.join(os.tmpdir(), 'meetily-check-command-'));
  const logPath = path.join(directory, 'commands.log');
  const stub = `#!/bin/sh
command_name=\${0##*/}
printf '%s|%s|%s\\n' "$PWD" "$command_name" "$*" >> "$MEETILY_TEST_LOG"
if [ "$command_name" = "uname" ]; then
  printf '%s\\n' "\${MEETILY_TEST_UNAME:-Darwin}"
  exit 0
fi
if [ "$command_name" = "cargo" ] && printf '%s\\n' "$*" | grep -q -- '--list'; then
  printf '%s\\n' "\${MEETILY_TEST_CARGO_LIST_OUTPUT-audio::recording_session_state::tests::permits_only_the_recording_session_contract_edges: test}"
fi
if [ "\${MEETILY_TEST_FAIL_MATCH:-}" = "$command_name $*" ]; then
  exit 23
fi
exit 0
`;

  try {
    for (const command of ['git', 'pnpm', 'cargo', 'uname']) {
      const commandPath = path.join(directory, command);
      writeFileSync(commandPath, stub);
      chmodSync(commandPath, 0o755);
    }
    writeFileSync(logPath, '');
    return run({ directory, logPath });
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
}

function runCheck(args, options = {}) {
  return withStubbedCommands(({ directory, logPath }) => {
    const result = spawnSync(checkScript, args, {
      cwd: repositoryRoot,
      encoding: 'utf8',
      env: {
        ...process.env,
        PATH: `${directory}:${process.env.PATH}`,
        MEETILY_TEST_LOG: logPath,
        MEETILY_CHECK_NO_XCODE: '1',
        MEETILY_CHECK_BASE_REF: 'dev',
        ...options.env,
      },
    });

    return {
      ...result,
      output: `${result.stdout}${result.stderr}`,
      commands: readFileSync(logPath, 'utf8'),
    };
  });
}

test('help documents both modes, every automatic stage, and the separate candidate command', () => {
  const result = runCheck(['--help']);

  assert.equal(result.status, 0);
  for (const expected of [
    '--quick',
    '--full',
    'git diff --check <base>...HEAD',
    'pnpm check:policy',
    'pnpm lint',
    'pnpm typecheck',
    'pnpm test:frontend',
    'playwright test',
    'pnpm build',
    'cargo fmt --all -- --check',
    'cargo clippy --workspace --all-targets -- -D warnings',
    'cargo test --workspace',
    'cargo test --test recording_recovery successful_checkpoint_recovery_preserves_neighboring_meeting_files -- --ignored',
    'test:docs',
    'cargo test -p meetily --lib audio::',
    'cargo test -p meetily --lib summary::',
    './scripts/release-candidate.sh',
  ]) {
    assert.match(result.output, new RegExp(expected.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')));
  }
});

test('docs mode runs only the non-empty documentation contract', () => {
  const result = runCheck(['--docs']);

  assert.equal(result.status, 0);
  assert.match(result.commands, /pnpm\|test:docs/);
  assert.doesNotMatch(result.commands, /playwright|cargo|pnpm\|build/);
  assert.match(result.output, /PASS.*Documentation guide/);
});

test('audio mode runs its filtered Rust tests after confirming a non-empty selection', () => {
  const result = runCheck(['--audio'], {
    env: {
      MEETILY_TEST_CARGO_LIST_OUTPUT:
        'audio::recording_session_state::tests::permits_only_the_recording_session_contract_edges: test',
    },
  });

  assert.equal(result.status, 0);
  assert.match(result.commands, /cargo\|test -p meetily --lib audio:: -- --list/);
  assert.match(result.commands, /cargo\|test -p meetily --lib audio:: --$/m);
  assert.doesNotMatch(result.commands, /playwright|pnpm\|build|cargo\|test --workspace/);
  assert.match(result.output, /PASS.*Audio tests/);
});

test('summary mode runs its filtered Rust tests after confirming a non-empty selection', () => {
  const result = runCheck(['--summary'], {
    env: {
      MEETILY_TEST_CARGO_LIST_OUTPUT:
        'summary::persistence::tests::repeated_saves_replace_one_summary_file: test',
    },
  });

  assert.equal(result.status, 0);
  assert.match(result.commands, /cargo\|test -p meetily --lib summary:: -- --list/);
  assert.match(result.commands, /cargo\|test -p meetily --lib summary:: --$/m);
  assert.doesNotMatch(result.commands, /playwright|pnpm\|build|cargo\|test --workspace/);
  assert.match(result.output, /PASS.*Summary tests/);
});

test('a filtered route fails when its cargo selection contains zero tests', () => {
  const result = runCheck(['--audio'], {
    env: { MEETILY_TEST_CARGO_LIST_OUTPUT: '' },
  });

  assert.equal(result.status, 1);
  assert.match(result.output, /No tests matched the audio filter/);
  assert.doesNotMatch(result.output, /PASS.*Audio tests/);
  assert.doesNotMatch(result.commands, /cargo\|test -p meetily --lib audio:: --$/m);
});

test('patch whitespace checks the committed branch range as well as local changes', () => {
  const result = runCheck(['--quick']);

  assert.equal(result.status, 0);
  assert.match(result.commands, /git\|diff --check dev\.\.\.HEAD/);
  assert.match(result.commands, /git\|diff --cached --check/);
  assert.match(result.commands, /git\|diff --check$/m);
});

test('committed whitespace in the branch range blocks the aggregate check', () => {
  const result = runCheck(['--quick'], {
    env: { MEETILY_TEST_FAIL_MATCH: 'git diff --check dev...HEAD' },
  });

  assert.equal(result.status, 1);
  assert.match(result.output, /FAIL.*Patch whitespace/);
  assert.match(result.output, /FAILED COMMAND: \.\/scripts\/check-whitespace\.sh/);
});

test('an unknown option exits nonzero before running child commands', () => {
  const result = runCheck(['--surprise']);

  assert.notEqual(result.status, 0);
  assert.match(result.output, /Unknown option: --surprise/);
  assert.equal(result.commands, '');
});

test('a failed stage names its standalone command and makes the aggregate command fail', () => {
  const result = runCheck(['--quick'], {
    env: { MEETILY_TEST_FAIL_MATCH: 'pnpm lint' },
  });

  assert.equal(result.status, 1);
  assert.match(result.output, /FAIL.*Frontend lint/);
  assert.match(result.output, /FAILED COMMAND: cd frontend && pnpm lint/);
  assert.match(result.commands, /pnpm\|lint/);
  assert.match(result.commands, /cargo\|test --workspace/);
});

test('quick mode runs every stage except the production frontend build', () => {
  const result = runCheck(['--quick']);

  assert.equal(result.status, 0);
  assert.match(result.output, /SKIP.*Production frontend build.*quick mode/);
  assert.doesNotMatch(result.commands, /pnpm\|build/);
  assert.match(result.commands, /pnpm\|exec playwright test/);
  assert.match(result.commands, /cargo\|fmt --all -- --check/);
  assert.match(result.commands, /cargo\|clippy --workspace --all-targets -- -D warnings/);
  assert.match(result.commands, /cargo\|test --workspace/);
  assert.match(
    result.commands,
    /cargo\|test --test recording_recovery successful_checkpoint_recovery_preserves_neighboring_meeting_files -- --ignored/,
  );
  assert.match(result.output, /PASS.*Packaged FFmpeg recovery/);
});

test('critical browser gate requires the IndexedDB transcript persistence contract', () => {
  const result = runCheck(['--quick']);

  assert.equal(result.status, 0);
  assert.match(
    result.commands,
    /pnpm\|exec playwright test[^\n]*tests\/indexeddb-transcript-contract\.spec\.ts/,
  );
});

test('critical browser gate requires local data safety and WebView egress contracts', () => {
  const result = runCheck(['--quick']);

  assert.equal(result.status, 0);
  assert.match(
    result.commands,
    /pnpm\|exec playwright test[^\n]*tests\/desktop-local-safety\.spec\.ts/,
  );
});

test('non-macOS mode honestly skips the packaged FFmpeg recovery integration', () => {
  const result = runCheck(['--quick'], {
    env: { MEETILY_TEST_UNAME: 'Linux' },
  });

  assert.equal(result.status, 0);
  assert.match(result.output, /SKIP.*Packaged FFmpeg recovery.*requires macOS/);
  assert.doesNotMatch(
    result.commands,
    /cargo\|test --test recording_recovery successful_checkpoint_recovery_preserves_neighboring_meeting_files -- --ignored/,
  );
});

test('full mode includes the production static export', () => {
  const result = runCheck(['--full']);

  assert.equal(result.status, 0);
  assert.match(result.commands, /pnpm\|build/);
  assert.match(result.output, /PASS.*Production frontend build/);
});

test('an unavailable child command is reported as FAIL and never as PASS', () => {
  const result = runCheck(['--quick'], {
    env: { MEETILY_TEST_FAIL_MATCH: 'cargo fmt --all -- --check' },
  });

  assert.equal(result.status, 1);
  assert.match(result.output, /FAIL.*Rust format/);
  assert.doesNotMatch(result.output, /PASS.*Rust format/);
});
