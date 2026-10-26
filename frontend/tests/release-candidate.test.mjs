import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import {
  chmodSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';

import * as releaseCandidateVerifier from '../scripts/verify-release-candidate.mjs';

const {
  classifySignature,
  selectDmgAppBundleEntry,
  verifyCandidate,
  writeCandidateReport,
} = releaseCandidateVerifier;

const repositoryRoot = path.resolve(import.meta.dirname, '../..');
const releaseScript = path.join(repositoryRoot, 'scripts', 'release-candidate.sh');

function createReleaseCommandFixture() {
  const root = mkdtempSync(path.join(os.tmpdir(), 'meetily-candidate-command-'));
  const scriptsPath = path.join(root, 'scripts');
  const frontendPath = path.join(root, 'frontend');
  const binPath = path.join(root, 'bin');
  const callLog = path.join(root, 'calls.log');
  const sourceMutationMarker = path.join(root, 'source-mutated');
  const commitSha = '0123456789abcdef0123456789abcdef01234567';
  const fixtureDmgSha256 = createHash('sha256')
    .update('verified dmg bytes')
    .digest('hex');

  mkdirSync(scriptsPath, { recursive: true });
  mkdirSync(frontendPath, { recursive: true });
  mkdirSync(binPath, { recursive: true });
  writeFileSync(
    path.join(scriptsPath, 'release-candidate.sh'),
    readFileSync(releaseScript),
  );
  chmodSync(path.join(scriptsPath, 'release-candidate.sh'), 0o755);
  writeFileSync(path.join(frontendPath, 'package.json'), '{"version":"0.4.1"}\n');

  writeFileSync(path.join(scriptsPath, 'check.sh'), `#!/bin/sh
echo "automatic check stub"
exit 0
`);
  chmodSync(path.join(scriptsPath, 'check.sh'), 0o755);

  writeFileSync(path.join(binPath, 'git'), `#!/bin/sh
if [ "$1" = "status" ]; then
  if [ -f "\${TEST_SOURCE_MUTATION_MARKER:-}" ]; then
    printf '%s\\n' "\${TEST_FINAL_GIT_STATUS:-\${TEST_GIT_STATUS:-}}"
  else
    printf '%s\\n' "\${TEST_GIT_STATUS:-}"
  fi
  exit 0
fi
if [ "$1" = "rev-parse" ]; then
  echo "${commitSha}"
  exit 0
fi
exit 99
`);
  writeFileSync(path.join(binPath, 'pnpm'), `#!/bin/sh
printf 'pnpm %s\\n' "$*" >> "$TEST_CALL_LOG"
case "$*" in
  *artifacts:seed:universal*) exit "\${TEST_SEED_EXIT:-0}" ;;
  *tauri:build:universal*)
    if [ "\${TEST_MUTATE_SOURCE_DURING_BUILD:-0}" -eq 1 ]; then
      : > "$TEST_SOURCE_MUTATION_MARKER"
    fi
    mkdir -p "$TEST_FIXTURE_ROOT/target/universal-apple-darwin/release/bundle/dmg"
    printf '%s' 'verified dmg bytes' > "$TEST_FIXTURE_ROOT/target/universal-apple-darwin/release/bundle/dmg/meetily_0.4.1_universal.dmg"
    exit "\${TEST_BUILD_EXIT:-0}"
    ;;
esac
exit 99
`);
  writeFileSync(path.join(binPath, 'node'), `#!/bin/sh
case "$*" in
  *source-fingerprint.mjs*)
    if [ -f "\${TEST_SOURCE_MUTATION_MARKER:-}" ]; then
      fingerprint="\${TEST_FINAL_SOURCE_FINGERPRINT:-\${TEST_SOURCE_FINGERPRINT:-${'f'.repeat(64)}}}"
      fingerprint_exit="\${TEST_FINAL_SOURCE_FINGERPRINT_EXIT:-\${TEST_SOURCE_FINGERPRINT_EXIT:-0}}"
    else
      fingerprint="\${TEST_SOURCE_FINGERPRINT:-${'f'.repeat(64)}}"
      fingerprint_exit="\${TEST_SOURCE_FINGERPRINT_EXIT:-0}"
    fi
    [ "$fingerprint_exit" -eq 0 ] && printf '%s\\n' "$fingerprint"
    exit "$fingerprint_exit"
    ;;
esac
if [ "$1" = "-p" ]; then
  [ "\${TEST_VERSION_EXIT:-0}" -eq 0 ] && echo "0.4.1"
  exit "\${TEST_VERSION_EXIT:-0}"
fi
if [ "$1" = "-e" ]; then
  exec ${JSON.stringify(process.execPath)} "$@"
fi
printf 'node %s\\n' "$*" >> "$TEST_CALL_LOG"
previous=""
output_path=""
commit_sha=""
candidate_mode=""
worktree_clean=""
run_id=""
source_fingerprint=""
dmg_path=""
for argument in "$@"; do
  case "$previous" in
    --output) output_path="$argument" ;;
    --commit) commit_sha="$argument" ;;
    --mode) candidate_mode="$argument" ;;
    --worktree-clean) worktree_clean="$argument" ;;
    --run-id) run_id="$argument" ;;
    --source-fingerprint) source_fingerprint="$argument" ;;
    --dmg) dmg_path="$argument" ;;
  esac
  previous="$argument"
done
if [ -n "$output_path" ]; then
  report_status=PASS
  release_eligible=true
  check_status=PASS
  if [ "$candidate_mode" = diagnostic ]; then
    report_status=FAIL
    release_eligible=false
    check_status=FAIL
  fi
  if [ "\${TEST_VERIFIER_REPORT_MODE:-pass}" = fail ]; then
    report_status=FAIL
    release_eligible=false
    check_status=FAIL
  fi
  case "\${TEST_VERIFIER_REPORT_MODE:-pass}" in
    missing) ;;
    empty) : > "$output_path" ;;
    invalid) printf '%s\\n' '{invalid json' > "$output_path" ;;
    minimal-pass) printf '%s\\n' '{"status":"PASS"}' > "$output_path" ;;
    mismatch)
      printf '%s\\n' '{"schemaVersion":2,"status":"PASS","commitSha":"0000000000000000000000000000000000000000","release":{"eligible":true},"provenance":{"mode":"normal","worktreeClean":true,"runId":"wrong-run","sourceFingerprint":"eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"},"artifact":{"path":"/tmp/wrong.dmg","sha256":"0000000000000000000000000000000000000000000000000000000000000000","embeddedBundleFingerprint":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"},"bundle":{"fingerprint":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"},"checks":[{"id":"fixture","status":"PASS"}]}' > "$output_path"
      ;;
    *)
      printf '{"schemaVersion":2,"status":"%s","commitSha":"%s","release":{"eligible":%s},"provenance":{"mode":"%s","worktreeClean":%s,"runId":"%s","sourceFingerprint":"%s"},"artifact":{"path":"%s","sha256":"${fixtureDmgSha256}","embeddedBundleFingerprint":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"},"bundle":{"fingerprint":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"},"checks":[{"id":"fixture","status":"%s"}]}\\n' "$report_status" "$commit_sha" "$release_eligible" "$candidate_mode" "$worktree_clean" "$run_id" "$source_fingerprint" "$dmg_path" "$check_status" > "$output_path"
      ;;
  esac
fi
if [ "\${TEST_MUTATE_SOURCE_DURING_VERIFIER:-0}" -eq 1 ]; then
  : > "$TEST_SOURCE_MUTATION_MARKER"
fi
exit "\${TEST_VERIFIER_EXIT:-0}"
`);
  for (const name of ['git', 'pnpm', 'node']) {
    chmodSync(path.join(binPath, name), 0o755);
  }

  return {
    root,
    commitSha,
    callLog,
    releaseScript: path.join(scriptsPath, 'release-candidate.sh'),
    env: {
      ...process.env,
      PATH: `${binPath}:${process.env.PATH}`,
      TEST_CALL_LOG: callLog,
      TEST_FIXTURE_ROOT: root,
      TEST_SOURCE_MUTATION_MARKER: sourceMutationMarker,
    },
  };
}

function diagnosticRuns(fixture, category) {
  const root = path.join(
    fixture.root,
    'artifacts',
    'release-gate',
    category,
    fixture.commitSha,
  );
  return existsSync(root)
    ? readdirSync(root, { withFileTypes: true })
      .filter((entry) => entry.isDirectory())
      .map((entry) => entry.name)
      .sort()
      .map((name) => path.join(root, name))
    : [];
}

function onlyDiagnosticRun(fixture, category) {
  const runs = diagnosticRuns(fixture, category);
  assert.equal(runs.length, 1, `expected one ${category} evidence run`);
  return runs[0];
}

function command(status, output = '') {
  return { status, stdout: output, stderr: '' };
}

function dmgEntry(name, type = 'directory') {
  return {
    name,
    isDirectory: () => type === 'directory',
  };
}

function entitlementPlist(entries) {
  const body = Object.entries(entries).map(([key, value]) => [
    `    <key>${key}</key>`,
    value ? '    <true/>' : '    <false/>',
  ].join('\n')).join('\n');
  return `<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0">
<dict>
${body}
</dict>
</plist>`;
}

function createCandidateFixture() {
  const root = mkdtempSync(path.join(os.tmpdir(), 'meetily-candidate-'));
  const appPath = path.join(root, 'meetily.app');
  const macosPath = path.join(appPath, 'Contents', 'MacOS');
  const resourcesPath = path.join(appPath, 'Contents', 'Resources');
  const templatesPath = path.join(resourcesPath, 'templates');
  const diarizationModelsPath = path.join(resourcesPath, 'diarization', 'models');
  const diarizationLicensesPath = path.join(resourcesPath, 'diarization', 'licenses');
  const dmgPath = path.join(root, 'meetily_0.4.1_universal.dmg');

  mkdirSync(macosPath, { recursive: true });
  mkdirSync(templatesPath, { recursive: true });
  mkdirSync(diarizationModelsPath, { recursive: true });
  mkdirSync(diarizationLicensesPath, { recursive: true });
  for (const executable of ['meetily', 'llama-helper', 'ffmpeg']) {
    const executablePath = path.join(macosPath, executable);
    writeFileSync(executablePath, `universal-${executable}`);
    chmodSync(executablePath, 0o755);
  }
  writeFileSync(path.join(resourcesPath, 'app_icon.icns'), 'icon');
  writeFileSync(path.join(templatesPath, 'standard_meeting.json'), '{"name":"Standard"}');
  writeFileSync(path.join(resourcesPath, 'frontend.js'), 'console.log("production")');
  writeFileSync(path.join(resourcesPath, 'diarization', 'NOTICE.md'), 'reviewed notice');
  writeFileSync(path.join(diarizationLicensesPath, 'Apache-2.0.txt'), 'apache license');
  writeFileSync(path.join(diarizationLicensesPath, 'MIT-pyannote.txt'), 'mit license');
  const diarizationModel = Buffer.from('fixture diarization model');
  const diarizationModelName = 'fixture-model.onnx';
  writeFileSync(path.join(diarizationModelsPath, diarizationModelName), diarizationModel);
  writeFileSync(dmgPath, 'verified dmg bytes');

  const input = {
    appPath,
    dmgPath,
    commitSha: '0123456789abcdef0123456789abcdef01234567',
    provenance: {
      mode: 'normal',
      worktreeClean: true,
      runId: 'normal-0123456789abcdef',
      sourceFingerprint: 'c'.repeat(64),
    },
    repositoryPath: '/Users/release/Meetily',
    bundleFingerprint: 'a'.repeat(64),
    expected: {
      productName: 'meetily',
      version: '0.4.1',
      bundleIdentifier: 'com.meetily.ai',
      executableName: 'meetily',
      iconFile: 'app_icon.icns',
      templates: ['standard_meeting.json'],
      diarizationArtifacts: [{
        kind: 'model',
        resourcePath: `diarization/models/${diarizationModelName}`,
        size: diarizationModel.length,
        sha256: createHash('sha256').update(diarizationModel).digest('hex'),
      }],
      requiredEntitlements: {
        'com.apple.security.device.audio-input': true,
        'com.apple.security.device.screen-capture': true,
      },
    },
    metadata: {
      productName: 'meetily',
      version: '0.4.1',
      bundleIdentifier: 'com.meetily.ai',
      executableName: 'meetily',
      iconFile: 'app_icon.icns',
    },
    architectures: {
      meetily: ['arm64', 'x86_64'],
      'llama-helper': ['arm64', 'x86_64'],
      ffmpeg: ['arm64', 'x86_64'],
    },
    diarizationRuntimeSlices: {
      arm64: true,
      x86_64: true,
    },
    evidence: {
      signatureDetails: command(
        0,
        'Authority=Developer ID Application: Meetily Team\nflags=0x10000(runtime)',
      ),
      signatureVerification: command(0, 'valid on disk'),
      entitlements: command(0, entitlementPlist({
        'com.apple.security.device.audio-input': true,
        'com.apple.security.device.screen-capture': true,
      })),
      gatekeeper: command(0, 'source=Notarized Developer ID'),
      notarization: command(0, 'The validate action worked!'),
      dmgVerification: command(0, 'verified'),
      dmgSignatureDetails: command(0, 'Authority=Developer ID Application: Meetily Team'),
      dmgSignatureVerification: command(0, 'valid on disk'),
      dmgGatekeeper: command(0, 'source=Notarized Developer ID'),
      dmgNotarization: command(0, 'The validate action worked!'),
      dmgBundleFingerprint: {
        status: 0,
        fingerprint: 'a'.repeat(64),
        detail: 'Mounted DMG contains meetily.app.',
      },
    },
  };

  return { root, appPath, dmgPath, input };
}

function check(report, id) {
  return report.checks.find((item) => item.id === id);
}

test('validates metadata, resources, Universal executables, signing, and artifact identity', () => {
  const fixture = createCandidateFixture();
  try {
    const report = verifyCandidate(fixture.input);

    assert.equal(report.status, 'PASS');
    assert.equal(report.commitSha, fixture.input.commitSha);
    assert.equal(
      report.artifact.sha256,
      createHash('sha256').update(readFileSync(fixture.dmgPath)).digest('hex'),
    );
    assert.equal(report.bundle.fingerprint, fixture.input.bundleFingerprint);
    assert.deepEqual(
      report.bundle.diarizationRuntimeSlices,
      fixture.input.diarizationRuntimeSlices,
    );
    assert.equal(
      report.artifact.embeddedBundleFingerprint,
      fixture.input.evidence.dmgBundleFingerprint.fingerprint,
    );
    for (const id of [
      'release.eligibility',
      'bundle.metadata',
      'bundle.resources',
      'bundle.executables',
      'bundle.architectures',
      'bundle.diarization-runtime',
      'bundle.static-scan',
      'signature.integrity',
      'signature.trust',
      'signature.hardened-runtime',
      'signature.entitlements',
      'gatekeeper.assess',
      'notarization.validate',
      'dmg.structure',
      'dmg.signature.integrity',
      'dmg.signature.trust',
      'dmg.gatekeeper',
      'dmg.notarization',
      'dmg.bundle-match',
    ]) {
      assert.equal(check(report, id).status, 'PASS', id);
    }
    assert.deepEqual(report.provenance, fixture.input.provenance);
    assert.equal(report.release.eligible, true);
    assert.equal(report.signature.kind, 'DEVELOPER_ID');
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
});

test('selects the actual root app entry when product name differs only by case', () => {
  assert.equal(
    selectDmgAppBundleEntry([
      dmgEntry('Applications', 'symlink'),
      dmgEntry('Meetily.app'),
    ], 'meetily'),
    'Meetily.app',
  );
  assert.equal(
    selectDmgAppBundleEntry([dmgEntry('MEETILY.APP')], 'meetily'),
    'MEETILY.APP',
  );
});

test('rejects a differently named, multiple, or missing root app bundle', () => {
  assert.equal(selectDmgAppBundleEntry([dmgEntry('Other.app')], 'meetily'), null);
  assert.equal(
    selectDmgAppBundleEntry([
      dmgEntry('Meetily.app'),
      dmgEntry('Other.APP'),
    ], 'meetily'),
    null,
  );
  assert.equal(
    selectDmgAppBundleEntry([
      dmgEntry('Applications', 'symlink'),
      dmgEntry('meetily.app', 'file'),
    ], 'meetily'),
    null,
  );
});

test('fully signed diagnostic or dirty provenance can never produce a releasable PASS report', () => {
  for (const provenance of [
    {
      mode: 'diagnostic',
      worktreeClean: true,
      runId: 'diagnostic-clean-run',
      sourceFingerprint: 'd'.repeat(64),
    },
    {
      mode: 'normal',
      worktreeClean: false,
      runId: 'dirty-normal-run',
      sourceFingerprint: 'e'.repeat(64),
    },
  ]) {
    const fixture = createCandidateFixture();
    try {
      fixture.input.provenance = provenance;

      const report = verifyCandidate(fixture.input);

      assert.equal(check(report, 'release.eligibility').status, 'FAIL');
      assert.equal(report.release.eligible, false);
      assert.equal(report.status, 'FAIL');
      assert.deepEqual(report.provenance, provenance);
      assert.equal(check(report, 'signature.trust').status, 'PASS');
      assert.equal(check(report, 'notarization.validate').status, 'PASS');
    } finally {
      rmSync(fixture.root, { recursive: true, force: true });
    }
  }
});

test('rejects wrong identity and every missing executable, configured icon, or template', () => {
  const fixture = createCandidateFixture();
  try {
    fixture.input.metadata.bundleIdentifier = 'com.example.dev';
    rmSync(path.join(fixture.appPath, 'Contents', 'MacOS', 'ffmpeg'));
    rmSync(path.join(fixture.appPath, 'Contents', 'Resources', 'app_icon.icns'));
    rmSync(path.join(
      fixture.appPath,
      'Contents',
      'Resources',
      'templates',
      'standard_meeting.json',
    ));

    const report = verifyCandidate(fixture.input);

    assert.equal(report.status, 'FAIL');
    assert.equal(check(report, 'bundle.metadata').status, 'FAIL');
    assert.equal(check(report, 'bundle.executables').status, 'FAIL');
    assert.equal(check(report, 'bundle.resources').status, 'FAIL');
    assert.match(check(report, 'bundle.executables').detail, /ffmpeg/);
    assert.match(check(report, 'bundle.resources').detail, /app_icon\.icns/);
    assert.match(check(report, 'bundle.resources').detail, /standard_meeting\.json/);
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
});

test('rejects a packaged diarization model whose checksum changes', () => {
  const fixture = createCandidateFixture();
  try {
    const modelPath = path.join(
      fixture.appPath,
      'Contents',
      'Resources',
      fixture.input.expected.diarizationArtifacts[0].resourcePath,
    );
    writeFileSync(modelPath, 'tampered diarization model');

    const report = verifyCandidate(fixture.input);
    assert.equal(check(report, 'bundle.resources').status, 'FAIL');
    assert.match(check(report, 'bundle.resources').detail, /checksum or size mismatch/);
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
});

test('requires arm64 and x86_64 slices in the app and both sidecars', () => {
  const fixture = createCandidateFixture();
  try {
    fixture.input.architectures.meetily = ['arm64'];
    fixture.input.architectures.ffmpeg = ['x86_64'];

    const report = verifyCandidate(fixture.input);

    assert.equal(check(report, 'bundle.architectures').status, 'FAIL');
    assert.match(check(report, 'bundle.architectures').detail, /meetily.*x86_64/);
    assert.match(check(report, 'bundle.architectures').detail, /ffmpeg.*arm64/);
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
});

test('requires the pinned diarization runtime in both executable slices', () => {
  const fixture = createCandidateFixture();
  try {
    fixture.input.diarizationRuntimeSlices.arm64 = false;

    const report = verifyCandidate(fixture.input);

    assert.equal(check(report, 'bundle.diarization-runtime').status, 'FAIL');
    assert.match(check(report, 'bundle.diarization-runtime').detail, /arm64/);
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
});

test('computes a deterministic bundle fingerprint that changes with packaged content', () => {
  const fixture = createCandidateFixture();
  try {
    assert.equal(typeof releaseCandidateVerifier.bundleFingerprint, 'function');
    const first = releaseCandidateVerifier.bundleFingerprint(fixture.appPath);
    const second = releaseCandidateVerifier.bundleFingerprint(fixture.appPath);
    assert.equal(first, second);
    assert.match(first, /^[a-f0-9]{64}$/);

    const executablePath = path.join(fixture.appPath, 'Contents', 'MacOS', 'meetily');
    chmodSync(executablePath, 0o644);
    assert.notEqual(releaseCandidateVerifier.bundleFingerprint(fixture.appPath), first);
    chmodSync(executablePath, 0o755);

    writeFileSync(
      path.join(fixture.appPath, 'Contents', 'Resources', 'frontend.js'),
      'console.log("changed production content")',
    );
    assert.notEqual(releaseCandidateVerifier.bundleFingerprint(fixture.appPath), first);
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
});

test('rejects developer paths, E2E hooks, and credential literals in packaged text', () => {
  const fixture = createCandidateFixture();
  try {
    writeFileSync(
      path.join(fixture.appPath, 'Contents', 'Resources', 'frontend.js'),
      [
        'const source = "/Users/release/Meetily/frontend/src";',
        'window.__emitTauriEvent = () => {};',
        `const ${'password'} = "${'accidentalReleasePassword123456'}";`,
      ].join('\n'),
    );

    const report = verifyCandidate(fixture.input);
    const scan = check(report, 'bundle.static-scan');

    assert.equal(scan.status, 'FAIL');
    assert.match(scan.detail, /developer-home path/);
    assert.match(scan.detail, /E2E bootstrap marker/);
    assert.match(scan.detail, /credential literal/);
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
});

test('rejects the developer home path embedded in a packaged executable', () => {
  const fixture = createCandidateFixture();
  try {
    fixture.input.repositoryPath = '/opt/clean-room/Meetily';
    writeFileSync(
      path.join(fixture.appPath, 'Contents', 'MacOS', 'meetily'),
      Buffer.concat([
        Buffer.from([0xcf, 0xfa, 0xed, 0xfe, 0x00]),
        Buffer.from('/Users/foreign-builder/private/Meetily/src/example.rs\0'),
      ]),
    );
    chmodSync(path.join(fixture.appPath, 'Contents', 'MacOS', 'meetily'), 0o755);

    const report = verifyCandidate(fixture.input);
    const scan = check(report, 'bundle.static-scan');

    assert.equal(scan.status, 'FAIL');
    assert.match(scan.detail, /MacOS\/meetily: developer-home path/);
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
});

test('records audited third-party build paths without treating them as workstation leaks', () => {
  const fixture = createCandidateFixture();
  try {
    writeFileSync(
      path.join(fixture.appPath, 'Contents', 'MacOS', 'meetily'),
      [
        '/Users/runner/work/ort-artifacts/ort-artifacts/onnxruntime/source.cc',
        '/Users/runner/work/onnxruntime-build/onnxruntime-build/onnxruntime/source.cc',
        '/Users/runner/work/sherpa-onnx/sherpa-onnx/sherpa-onnx/c-api/c-api.cc',
        '',
      ].join('\0'),
    );
    writeFileSync(
      path.join(fixture.appPath, 'Contents', 'MacOS', 'ffmpeg'),
      '/Users/Shared/ffmpeg/sw/etc/fonts/fonts.conf\0',
    );
    chmodSync(path.join(fixture.appPath, 'Contents', 'MacOS', 'meetily'), 0o755);
    chmodSync(path.join(fixture.appPath, 'Contents', 'MacOS', 'ffmpeg'), 0o755);

    const report = verifyCandidate(fixture.input);
    const scan = check(report, 'bundle.static-scan');

    assert.equal(scan.status, 'PASS');
    assert.match(scan.detail, /audited third-party build paths/i);
    assert.match(scan.detail, /meetily.*ort-artifacts/i);
    assert.match(scan.detail, /meetily.*onnxruntime-build/i);
    assert.match(scan.detail, /meetily.*sherpa-onnx/i);
    assert.match(scan.detail, /ffmpeg/i);
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
});

test('distinguishes ad-hoc, Developer ID, and unsigned evidence', () => {
  assert.equal(classifySignature('Signature=adhoc'), 'AD_HOC');
  assert.equal(
    classifySignature('Authority=Developer ID Application: Meetily Team'),
    'DEVELOPER_ID',
  );
  assert.equal(classifySignature('code object is not signed at all'), 'UNSIGNED');
});

test('ad-hoc app can pass signature integrity but fails distribution trust', () => {
  const fixture = createCandidateFixture();
  try {
    fixture.input.evidence.signatureDetails = command(0, 'Signature=adhoc\nflags=0x10000(runtime)');

    const report = verifyCandidate(fixture.input);

    assert.equal(check(report, 'signature.integrity').status, 'PASS');
    assert.equal(check(report, 'signature.trust').status, 'FAIL');
    assert.equal(report.signature.kind, 'AD_HOC');
    assert.equal(report.status, 'FAIL');
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
});

test('fails when the packaged app lacks hardened runtime evidence', () => {
  const fixture = createCandidateFixture();
  try {
    fixture.input.evidence.signatureDetails = command(
      0,
      'Authority=Developer ID Application: Meetily Team\nflags=0x0(none)',
    );

    const report = verifyCandidate(fixture.input);

    assert.equal(check(report, 'signature.hardened-runtime').status, 'FAIL');
    assert.equal(report.status, 'FAIL');
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
});

test('fails when a required entitlement is absent from the packaged app', () => {
  const fixture = createCandidateFixture();
  try {
    fixture.input.evidence.entitlements = command(0, entitlementPlist({
      'com.apple.security.device.audio-input': true,
    }));

    const report = verifyCandidate(fixture.input);
    const entitlements = check(report, 'signature.entitlements');

    assert.equal(entitlements.status, 'FAIL');
    assert.match(entitlements.detail, /com\.apple\.security\.device\.screen-capture/);
    assert.equal(report.status, 'FAIL');
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
});

test('rejects every explicitly dangerous entitlement when enabled', () => {
  const fixture = createCandidateFixture();
  const dangerousEntitlements = [
    'com.apple.security.get-task-allow',
    'com.apple.security.cs.disable-library-validation',
    'com.apple.security.cs.allow-unsigned-executable-memory',
    'com.apple.security.cs.allow-dyld-environment-variables',
  ];
  try {
    fixture.input.evidence.entitlements = command(0, entitlementPlist({
      'com.apple.security.device.audio-input': true,
      'com.apple.security.device.screen-capture': true,
      ...Object.fromEntries(dangerousEntitlements.map((key) => [key, true])),
    }));

    const report = verifyCandidate(fixture.input);
    const entitlements = check(report, 'signature.entitlements');

    assert.equal(entitlements.status, 'FAIL');
    for (const key of dangerousEntitlements) {
      assert.match(entitlements.detail, new RegExp(key.replaceAll('.', '\\.')));
    }
    assert.equal(report.status, 'FAIL');
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
});

test('DMG structural success does not hide unsigned, untrusted, or unstapled evidence', () => {
  const fixture = createCandidateFixture();
  try {
    fixture.input.evidence.dmgSignatureDetails = command(1, 'code object is not signed at all');
    fixture.input.evidence.dmgSignatureVerification = command(1, 'code object is not signed at all');
    fixture.input.evidence.dmgGatekeeper = command(1, 'source=no usable signature');
    fixture.input.evidence.dmgNotarization = command(65, 'Could not validate ticket');

    const report = verifyCandidate(fixture.input);

    assert.equal(check(report, 'dmg.structure').status, 'PASS');
    assert.equal(check(report, 'dmg.signature.integrity').status, 'FAIL');
    assert.equal(check(report, 'dmg.signature.trust').status, 'FAIL');
    assert.equal(check(report, 'dmg.gatekeeper').status, 'FAIL');
    assert.equal(check(report, 'dmg.notarization').status, 'FAIL');
    assert.equal(report.status, 'FAIL');
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
});

test('fails when the app inside the DMG does not match the verified app bundle', () => {
  const fixture = createCandidateFixture();
  try {
    fixture.input.evidence.dmgBundleFingerprint.fingerprint = 'b'.repeat(64);

    const report = verifyCandidate(fixture.input);
    const bundleMatch = check(report, 'dmg.bundle-match');

    assert.equal(bundleMatch.status, 'FAIL');
    assert.match(bundleMatch.detail, /does not match/i);
    assert.equal(report.bundle.fingerprint, 'a'.repeat(64));
    assert.equal(report.artifact.embeddedBundleFingerprint, 'b'.repeat(64));
    assert.equal(report.status, 'FAIL');
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
});

test('missing packaged signing evidence is NOT_RUN and never passes the report', () => {
  const fixture = createCandidateFixture();
  try {
    fixture.input.evidence.signatureDetails = undefined;
    fixture.input.evidence.signatureVerification = undefined;
    fixture.input.evidence.entitlements = undefined;
    fixture.input.evidence.dmgVerification = undefined;
    fixture.input.evidence.dmgSignatureDetails = undefined;
    fixture.input.evidence.dmgSignatureVerification = undefined;
    fixture.input.evidence.dmgGatekeeper = undefined;
    fixture.input.evidence.dmgNotarization = undefined;
    fixture.input.evidence.dmgBundleFingerprint = undefined;

    const report = verifyCandidate(fixture.input);

    for (const id of [
      'signature.integrity',
      'signature.trust',
      'signature.hardened-runtime',
      'signature.entitlements',
      'dmg.structure',
      'dmg.signature.integrity',
      'dmg.signature.trust',
      'dmg.gatekeeper',
      'dmg.notarization',
      'dmg.bundle-match',
    ]) {
      assert.equal(check(report, id).status, 'NOT_RUN', id);
    }
    assert.equal(report.status, 'FAIL');
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
});

test('Gatekeeper or notarization without evidence is NOT_RUN and never PASS', () => {
  const fixture = createCandidateFixture();
  try {
    fixture.input.evidence.gatekeeper = undefined;
    fixture.input.evidence.notarization = undefined;

    const report = verifyCandidate(fixture.input);

    assert.equal(check(report, 'gatekeeper.assess').status, 'NOT_RUN');
    assert.equal(check(report, 'notarization.validate').status, 'NOT_RUN');
    assert.equal(report.status, 'FAIL');
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
});

test('writes the commit-bound and checksum-bound JSON report', () => {
  const fixture = createCandidateFixture();
  try {
    const report = verifyCandidate(fixture.input);
    const reportPath = path.join(fixture.root, 'evidence', 'candidate.json');
    writeCandidateReport(report, reportPath);

    const saved = JSON.parse(readFileSync(reportPath, 'utf8'));
    assert.equal(saved.commitSha, fixture.input.commitSha);
    assert.equal(saved.artifact.sha256, report.artifact.sha256);
    assert.deepEqual(saved, report);
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
});

test('candidate command help documents normal and explicitly ungated diagnostic modes', () => {
  const result = spawnSync(releaseScript, ['--help'], {
    cwd: repositoryRoot,
    encoding: 'utf8',
  });

  assert.equal(result.status, 0);
  assert.match(result.stdout, /--diagnostic-build/);
  assert.match(result.stdout, /UNGATED/);
  assert.match(result.stdout, /artifacts\/release-gate/);
});

test('normal candidate mode refuses tracked changes before running the build', () => {
  const fixture = createReleaseCommandFixture();
  try {
    const result = spawnSync(fixture.releaseScript, [], {
      cwd: fixture.root,
      encoding: 'utf8',
      env: { ...fixture.env, TEST_GIT_STATUS: ' M frontend/src/app/page.tsx' },
    });

    assert.equal(result.status, 1);
    assert.match(`${result.stdout}${result.stderr}`, /worktree changes/i);
    assert.doesNotMatch(`${result.stdout}${result.stderr}`, /Building Universal Meetily/);
    const evidenceDirectory = onlyDiagnosticRun(fixture, 'preflight-dirty');
    const status = readFileSync(path.join(evidenceDirectory, 'status.env'), 'utf8');
    assert.match(status, /^STATUS=BLOCKED_DIRTY_WORKTREE$/m);
    assert.match(status, /^MODE=normal$/m);
    assert.match(status, /^WORKTREE_CLEAN=false$/m);
    assert.match(status, /^SOURCE_FINGERPRINT=[a-f0-9]{64}$/m);
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
});

test('normal candidate mode refuses untracked changes before running the build', () => {
  const fixture = createReleaseCommandFixture();
  try {
    const result = spawnSync(fixture.releaseScript, [], {
      cwd: fixture.root,
      encoding: 'utf8',
      env: { ...fixture.env, TEST_GIT_STATUS: '?? local-release-notes.txt' },
    });

    assert.equal(result.status, 1);
    assert.match(`${result.stdout}${result.stderr}`, /worktree changes/i);
    assert.doesNotMatch(`${result.stdout}${result.stderr}`, /Building Universal Meetily/);
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
});

test('normal candidate blocks when the worktree becomes dirty during packaging', () => {
  const fixture = createReleaseCommandFixture();
  try {
    const finalFingerprint = 'e'.repeat(64);
    const result = spawnSync(fixture.releaseScript, [], {
      cwd: fixture.root,
      encoding: 'utf8',
      env: {
        ...fixture.env,
        TEST_GIT_STATUS: '',
        TEST_SOURCE_FINGERPRINT: 'f'.repeat(64),
        TEST_MUTATE_SOURCE_DURING_BUILD: '1',
        TEST_FINAL_GIT_STATUS: ' M frontend/src/app/page.tsx',
        TEST_FINAL_SOURCE_FINGERPRINT: finalFingerprint,
      },
    });
    const status = readFileSync(path.join(
      fixture.root,
      'artifacts',
      'release-gate',
      fixture.commitSha,
      'status.env',
    ), 'utf8');

    assert.equal(result.status, 1);
    assert.match(`${result.stdout}${result.stderr}`, /source changed/i);
    assert.match(status, /^STATUS=BLOCKED_SOURCE_CHANGED$/m);
    assert.match(status, /^FINAL_WORKTREE_CLEAN=false$/m);
    assert.match(status, new RegExp(`^FINAL_SOURCE_FINGERPRINT=${finalFingerprint}$`, 'm'));
    assert.match(status, /^SOURCE_STABLE=false$/m);
    assert.doesNotMatch(`${result.stdout}${result.stderr}`, /Candidate verified/);
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
});

test('normal candidate blocks a clean commit or source fingerprint change during packaging', () => {
  const fixture = createReleaseCommandFixture();
  try {
    const result = spawnSync(fixture.releaseScript, [], {
      cwd: fixture.root,
      encoding: 'utf8',
      env: {
        ...fixture.env,
        TEST_GIT_STATUS: '',
        TEST_SOURCE_FINGERPRINT: 'a'.repeat(64),
        TEST_MUTATE_SOURCE_DURING_BUILD: '1',
        TEST_FINAL_GIT_STATUS: '',
        TEST_FINAL_SOURCE_FINGERPRINT: 'b'.repeat(64),
      },
    });
    const status = readFileSync(path.join(
      fixture.root,
      'artifacts',
      'release-gate',
      fixture.commitSha,
      'status.env',
    ), 'utf8');

    assert.equal(result.status, 1);
    assert.match(status, /^STATUS=BLOCKED_SOURCE_CHANGED$/m);
    assert.match(status, /^FINAL_WORKTREE_CLEAN=true$/m);
    assert.match(status, /^SOURCE_STABLE=false$/m);
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
});

test('normal candidate never publishes a report when source changes during verification', () => {
  const fixture = createReleaseCommandFixture();
  try {
    const result = spawnSync(fixture.releaseScript, [], {
      cwd: fixture.root,
      encoding: 'utf8',
      env: {
        ...fixture.env,
        TEST_GIT_STATUS: '',
        TEST_SOURCE_FINGERPRINT: 'a'.repeat(64),
        TEST_MUTATE_SOURCE_DURING_VERIFIER: '1',
        TEST_FINAL_GIT_STATUS: ' M frontend/src/app/page.tsx',
        TEST_FINAL_SOURCE_FINGERPRINT: 'b'.repeat(64),
      },
    });
    const evidenceDirectory = path.join(
      fixture.root,
      'artifacts',
      'release-gate',
      fixture.commitSha,
    );
    const status = readFileSync(path.join(evidenceDirectory, 'status.env'), 'utf8');

    assert.equal(result.status, 1);
    assert.match(status, /^STATUS=BLOCKED_SOURCE_CHANGED$/m);
    assert.equal(existsSync(path.join(evidenceDirectory, 'candidate-report.json')), false);
    assert.equal(existsSync(path.join(evidenceDirectory, 'candidate-report.pending.json')), false);
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
});

test('normal candidate blocks when verifier exits zero without a valid PASS report', () => {
  for (const reportMode of [
    'missing',
    'empty',
    'invalid',
    'fail',
    'minimal-pass',
    'mismatch',
  ]) {
    const fixture = createReleaseCommandFixture();
    try {
      const result = spawnSync(fixture.releaseScript, [], {
        cwd: fixture.root,
        encoding: 'utf8',
        env: {
          ...fixture.env,
          TEST_GIT_STATUS: '',
          TEST_VERIFIER_REPORT_MODE: reportMode,
        },
      });
      const evidenceDirectory = path.join(
        fixture.root,
        'artifacts',
        'release-gate',
        fixture.commitSha,
      );
      const status = readFileSync(path.join(evidenceDirectory, 'status.env'), 'utf8');
      const output = `${result.stdout}${result.stderr}`;

      assert.equal(result.status, 1, reportMode);
      assert.match(status, /^STATUS=BLOCKED_CANDIDATE_VERIFICATION$/m, reportMode);
      assert.equal(
        existsSync(path.join(evidenceDirectory, 'candidate-report.json')),
        false,
        reportMode,
      );
      assert.equal(
        existsSync(path.join(evidenceDirectory, 'candidate-report.pending.json')),
        false,
        reportMode,
      );
      assert.doesNotMatch(output, /Candidate verified/, reportMode);
    } finally {
      rmSync(fixture.root, { recursive: true, force: true });
    }
  }
});

test('normal candidate never publishes a report when verifier exits nonzero', () => {
  for (const reportMode of ['fail', 'pass']) {
    const fixture = createReleaseCommandFixture();
    try {
      const result = spawnSync(fixture.releaseScript, [], {
        cwd: fixture.root,
        encoding: 'utf8',
        env: {
          ...fixture.env,
          TEST_GIT_STATUS: '',
          TEST_VERIFIER_EXIT: '7',
          TEST_VERIFIER_REPORT_MODE: reportMode,
        },
      });
      const evidenceDirectory = path.join(
        fixture.root,
        'artifacts',
        'release-gate',
        fixture.commitSha,
      );
      const status = readFileSync(path.join(evidenceDirectory, 'status.env'), 'utf8');
      const output = `${result.stdout}${result.stderr}`;

      assert.equal(result.status, 1, reportMode);
      assert.match(status, /^STATUS=BLOCKED_CANDIDATE_VERIFICATION$/m, reportMode);
      assert.match(status, /^VERIFIER_EXIT=7$/m, reportMode);
      assert.equal(
        existsSync(path.join(evidenceDirectory, 'candidate-report.json')),
        false,
        reportMode,
      );
      assert.equal(
        existsSync(path.join(evidenceDirectory, 'candidate-report.pending.json')),
        false,
        reportMode,
      );
      assert.doesNotMatch(output, /Candidate verified/, reportMode);
    } finally {
      rmSync(fixture.root, { recursive: true, force: true });
    }
  }
});

test('diagnostic mode packages dirty worktree only as separately recorded UNGATED evidence', () => {
  const fixture = createReleaseCommandFixture();
  try {
    const result = spawnSync(fixture.releaseScript, ['--diagnostic-build'], {
      cwd: fixture.root,
      encoding: 'utf8',
      env: { ...fixture.env, TEST_GIT_STATUS: '?? local-release-notes.txt' },
    });
    const output = `${result.stdout}${result.stderr}`;
    const evidenceDirectory = onlyDiagnosticRun(fixture, 'diagnostic-dirty');
    const status = readFileSync(path.join(evidenceDirectory, 'status.env'), 'utf8');
    const calls = readFileSync(fixture.callLog, 'utf8');

    assert.equal(result.status, 1);
    assert.match(output, /DIRTY/);
    assert.match(output, /UNGATED/);
    assert.match(output, /Building Universal Meetily/);
    assert.match(calls, /artifacts:seed:universal/);
    assert.match(calls, /tauri:build:universal/);
    assert.match(status, /^STATUS=UNGATED$/m);
    assert.match(status, /^MODE=diagnostic$/m);
    assert.match(status, /^WORKTREE_CLEAN=false$/m);
    assert.match(status, /^RUN_ID=run-[A-Za-z0-9._-]+$/m);
    assert.match(status, /^SOURCE_FINGERPRINT=[a-f0-9]{64}$/m);
    assert.match(calls, /--mode diagnostic/);
    assert.match(calls, /--worktree-clean false/);
    assert.match(calls, /--run-id run-/);
    assert.match(calls, /--source-fingerprint [a-f0-9]{64}/);
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
});

test('diagnostic mode keeps a clean worktree in separate UNGATED diagnostic evidence', () => {
  const fixture = createReleaseCommandFixture();
  try {
    const result = spawnSync(fixture.releaseScript, ['--diagnostic-build'], {
      cwd: fixture.root,
      encoding: 'utf8',
      env: { ...fixture.env, TEST_GIT_STATUS: '' },
    });
    const evidenceDirectory = onlyDiagnosticRun(fixture, 'diagnostic');
    const status = readFileSync(path.join(evidenceDirectory, 'status.env'), 'utf8');
    const calls = readFileSync(fixture.callLog, 'utf8');

    assert.equal(result.status, 1);
    assert.match(`${result.stdout}${result.stderr}`, /UNGATED/);
    assert.match(calls, /artifacts:seed:universal/);
    assert.match(calls, /tauri:build:universal/);
    assert.match(status, /^STATUS=UNGATED$/m);
    assert.match(status, /^MODE=diagnostic$/m);
    assert.match(status, /^WORKTREE_CLEAN=true$/m);
    assert.equal(existsSync(path.join(
      fixture.root,
      'artifacts',
      'release-gate',
      'diagnostic-dirty',
      fixture.commitSha,
    )), false);
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
});

test('diagnostic runs at the same commit use unique directories and source identities', () => {
  const fixture = createReleaseCommandFixture();
  try {
    for (const sourceFingerprint of ['a'.repeat(64), 'b'.repeat(64)]) {
      const result = spawnSync(fixture.releaseScript, ['--diagnostic-build'], {
        cwd: fixture.root,
        encoding: 'utf8',
        env: {
          ...fixture.env,
          TEST_GIT_STATUS: '?? local-release-notes.txt',
          TEST_SOURCE_FINGERPRINT: sourceFingerprint,
        },
      });
      assert.equal(result.status, 1);
    }

    const runs = diagnosticRuns(fixture, 'diagnostic-dirty');
    assert.equal(runs.length, 2);
    assert.notEqual(runs[0], runs[1]);
    assert.deepEqual(
      runs.map((run) => readFileSync(path.join(run, 'status.env'), 'utf8'))
        .map((status) => status.match(/^SOURCE_FINGERPRINT=([a-f0-9]{64})$/m)?.[1])
        .sort(),
      ['a'.repeat(64), 'b'.repeat(64)],
    );
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
});

test('artifact seed failure writes a terminal status with every downstream exit field', () => {
  const fixture = createReleaseCommandFixture();
  try {
    const result = spawnSync(fixture.releaseScript, [], {
      cwd: fixture.root,
      encoding: 'utf8',
      env: { ...fixture.env, TEST_GIT_STATUS: '', TEST_SEED_EXIT: '7' },
    });
    const status = readFileSync(path.join(
      fixture.root,
      'artifacts',
      'release-gate',
      fixture.commitSha,
      'status.env',
    ), 'utf8');

    assert.equal(result.status, 1);
    assert.match(status, /^STATUS=ARTIFACT_SEED_FAILED$/m);
    assert.match(status, /^ARTIFACT_SEED_EXIT=7$/m);
    assert.match(status, /^BUILD_EXIT=NOT_RUN$/m);
    assert.match(status, /^VERSION_RESOLUTION_EXIT=NOT_RUN$/m);
    assert.match(status, /^VERIFIER_EXIT=NOT_RUN$/m);
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
});

test('version resolution failure writes terminal status after a successful build', () => {
  const fixture = createReleaseCommandFixture();
  try {
    const result = spawnSync(fixture.releaseScript, [], {
      cwd: fixture.root,
      encoding: 'utf8',
      env: { ...fixture.env, TEST_GIT_STATUS: '', TEST_VERSION_EXIT: '9' },
    });
    const status = readFileSync(path.join(
      fixture.root,
      'artifacts',
      'release-gate',
      fixture.commitSha,
      'status.env',
    ), 'utf8');

    assert.equal(result.status, 1);
    assert.match(status, /^STATUS=VERSION_RESOLUTION_FAILED$/m);
    assert.match(status, /^ARTIFACT_SEED_EXIT=0$/m);
    assert.match(status, /^BUILD_EXIT=0$/m);
    assert.match(status, /^VERSION_RESOLUTION_EXIT=9$/m);
    assert.match(status, /^VERIFIER_EXIT=NOT_RUN$/m);
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
});
