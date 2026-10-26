import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import test from 'node:test';

import {
  REQUIRED_SMOKE_IDS,
  verifySmokeReport,
} from '../scripts/verify-smoke-report.mjs';

const repositoryRoot = path.resolve(import.meta.dirname, '../..');
const releaseGateScript = path.join(repositoryRoot, 'scripts', 'release-gate.sh');
const testPlanGateScript = path.join(repositoryRoot, 'scripts', 'test-plan-gate.sh');

const commitSha = '0123456789abcdef0123456789abcdef01234567';
const artifactSha = 'a'.repeat(64);
const distributionTrustSmokeIds = [
  'SMOKE-SIGNATURE',
  'SMOKE-GATEKEEPER',
  'SMOKE-NOTARIZATION',
];
const testPlanCandidateCheckIds = [
  'release.eligibility',
  'bundle.metadata',
  'bundle.executables',
  'bundle.resources',
  'bundle.architectures',
  'bundle.diarization-runtime',
  'bundle.static-scan',
  'signature.integrity',
  'signature.verify',
  'signature.hardened-runtime',
  'signature.entitlements',
  'dmg.structure',
  'dmg.verify',
  'dmg.bundle-match',
];

function candidateReport(overrides = {}) {
  return {
    schemaVersion: 2,
    status: 'PASS',
    commitSha,
    provenance: {
      mode: 'normal',
      worktreeClean: true,
      runId: 'normal-0123456789abcdef',
      sourceFingerprint: 'c'.repeat(64),
    },
    release: { eligible: true },
    artifact: { sha256: artifactSha },
    signature: { kind: 'DEVELOPER_ID' },
    checks: [
      { id: 'release.eligibility', status: 'PASS' },
      { id: 'signature.verify', status: 'PASS' },
      { id: 'gatekeeper.assess', status: 'PASS' },
      { id: 'notarization.validate', status: 'PASS' },
    ],
    ...overrides,
  };
}

function completeReport({
  commit = commitSha,
  checksum = artifactSha,
  statuses = {},
  evidence = {},
  omitted = [],
  duplicate,
  operator = 'Release Operator',
  date = '2026-07-14',
  candidateReference = 'artifacts/release-gate/012345/candidate-report.json',
  macModel = 'MacBook Pro M3 Pro',
  macosVersion = 'macOS 15.5',
  audioDevices = 'Built-in Microphone; BlackHole 2ch',
} = {}) {
  const rows = REQUIRED_SMOKE_IDS
    .filter((id) => !omitted.includes(id))
    .map((id) => `| ${id} | ${statuses[id] ?? 'PASS'} | ${evidence[id] ?? `Observed ${id} successfully`} |`);
  if (duplicate) rows.push(`| ${duplicate} | PASS | Duplicate evidence |`);

  return `# Meetily release smoke report

Operator: ${operator}
Date: ${date}
Commit SHA: ${commit}
Candidate report: ${candidateReference}
Artifact SHA-256: ${checksum}
Mac model: ${macModel}
macOS version: ${macosVersion}
Audio devices: ${audioDevices}

| ID | Status | Evidence |
| --- | --- | --- |
${rows.join('\n')}
`;
}

test('a complete report bound to the verified candidate passes', () => {
  const verification = verifySmokeReport({
    content: completeReport(),
    expectedCommit: commitSha,
    expectedArtifactSha256: artifactSha,
    candidateReport: candidateReport(),
  });

  assert.equal(verification.status, 'PASS');
  assert.deepEqual(verification.errors, []);
  assert.equal(Object.keys(verification.items).length, REQUIRED_SMOKE_IDS.length);
});

test('public gate rejects missing or legacy candidate report schemas', () => {
  for (const schemaVersion of [undefined, 1]) {
    const report = candidateReport();
    if (schemaVersion === undefined) {
      delete report.schemaVersion;
    } else {
      report.schemaVersion = schemaVersion;
    }

    const verification = verifySmokeReport({
      content: completeReport(),
      expectedCommit: commitSha,
      expectedArtifactSha256: artifactSha,
      candidateReport: report,
    });

    assert.equal(verification.status, 'FAIL');
    assert.ok(verification.errors.some((error) => /schema version 2/i.test(error)));
  }
});

test('public gate rejects a fully signed report whose JSON provenance is diagnostic or dirty', () => {
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
    const verification = verifySmokeReport({
      content: completeReport(),
      expectedCommit: commitSha,
      expectedArtifactSha256: artifactSha,
      candidateReport: candidateReport({
        provenance,
        release: { eligible: false },
      }),
    });

    assert.equal(verification.status, 'FAIL');
    assert.ok(verification.errors.some((error) => /normal clean release|eligible/i.test(error)));
  }
});

test('public gate rejects a candidate report for a different source fingerprint', () => {
  const verification = verifySmokeReport({
    content: completeReport(),
    expectedCommit: commitSha,
    expectedArtifactSha256: artifactSha,
    expectedSourceFingerprint: 'd'.repeat(64),
    candidateReport: candidateReport(),
  });

  assert.equal(verification.status, 'FAIL');
  assert.ok(verification.errors.some((error) => /source fingerprint/i.test(error)));
});

test('rejects placeholder operator, date, candidate, checksum, and hardware fields', () => {
  const verification = verifySmokeReport({
    content: completeReport({
      operator: '<operator>',
      date: 'TODO',
      candidateReference: '<candidate-report>',
      checksum: '<sha256>',
      macModel: '<mac-model>',
      macosVersion: 'NOT_RUN',
      audioDevices: '',
    }),
    expectedCommit: commitSha,
    expectedArtifactSha256: artifactSha,
    candidateReport: candidateReport(),
  });

  assert.equal(verification.status, 'FAIL');
  for (const field of [
    'Operator',
    'Date',
    'Candidate report',
    'Artifact SHA-256',
    'Mac model',
    'macOS version',
    'Audio devices',
  ]) {
    assert.ok(verification.errors.some((error) => error.includes(field)), field);
  }
});

test('rejects missing and duplicate required IDs', () => {
  const missing = REQUIRED_SMOKE_IDS[0];
  const duplicate = REQUIRED_SMOKE_IDS[1];
  const verification = verifySmokeReport({
    content: completeReport({ omitted: [missing], duplicate }),
    expectedCommit: commitSha,
    expectedArtifactSha256: artifactSha,
    candidateReport: candidateReport(),
  });

  assert.equal(verification.status, 'FAIL');
  assert.ok(verification.errors.some((error) => error.includes(`Missing required smoke ID: ${missing}`)));
  assert.ok(verification.errors.some((error) => error.includes(`Duplicate smoke ID: ${duplicate}`)));
});

test('rejects FAIL, NOT_RUN, invalid statuses, and blank evidence', () => {
  const [failed, notRun, invalid, blank] = REQUIRED_SMOKE_IDS;
  const verification = verifySmokeReport({
    content: completeReport({
      statuses: {
        [failed]: 'FAIL',
        [notRun]: 'NOT_RUN',
        [invalid]: 'MAYBE',
      },
      evidence: { [blank]: '' },
    }),
    expectedCommit: commitSha,
    expectedArtifactSha256: artifactSha,
    candidateReport: candidateReport(),
  });

  assert.equal(verification.status, 'FAIL');
  assert.ok(verification.errors.some((error) => error.includes(`${failed} must be PASS`)));
  assert.ok(verification.errors.some((error) => error.includes(`${notRun} must be PASS`)));
  assert.ok(verification.errors.some((error) => error.includes(`${invalid} has invalid status`)));
  assert.ok(verification.errors.some((error) => error.includes(`${blank} requires evidence`)));
});

test('rejects a report for another commit or artifact checksum', () => {
  const verification = verifySmokeReport({
    content: completeReport({
      commit: 'f'.repeat(40),
      checksum: 'b'.repeat(64),
    }),
    expectedCommit: commitSha,
    expectedArtifactSha256: artifactSha,
    candidateReport: candidateReport(),
  });

  assert.equal(verification.status, 'FAIL');
  assert.ok(verification.errors.some((error) => error.includes('Commit SHA does not match')));
  assert.ok(verification.errors.some((error) => error.includes('Artifact SHA-256 does not match')));
});

test('rejects signing, Gatekeeper, or notarization claims absent from candidate evidence', () => {
  const report = candidateReport({
    signature: { kind: 'AD_HOC' },
    checks: [
      { id: 'signature.verify', status: 'PASS' },
      { id: 'gatekeeper.assess', status: 'NOT_RUN' },
      { id: 'notarization.validate', status: 'FAIL' },
    ],
  });
  const verification = verifySmokeReport({
    content: completeReport(),
    expectedCommit: commitSha,
    expectedArtifactSha256: artifactSha,
    candidateReport: report,
  });

  assert.equal(verification.status, 'FAIL');
  assert.ok(verification.errors.some((error) => error.includes('Developer ID signature')));
  assert.ok(verification.errors.some((error) => error.includes('Gatekeeper')));
  assert.ok(verification.errors.some((error) => error.includes('notarization')));
});

test('test-plan profile excludes Apple distribution trust without hiding package failures', () => {
  const unsignedCandidate = candidateReport({
    status: 'FAIL',
    release: { eligible: false },
    signature: { kind: 'AD_HOC' },
    checks: [
      ...testPlanCandidateCheckIds.map((id) => ({ id, status: 'PASS' })),
      { id: 'signature.trust', status: 'FAIL' },
      { id: 'gatekeeper.assess', status: 'FAIL' },
      { id: 'notarization.validate', status: 'FAIL' },
      { id: 'dmg.signature.integrity', status: 'FAIL' },
      { id: 'dmg.signature.trust', status: 'FAIL' },
      { id: 'dmg.gatekeeper', status: 'FAIL' },
      { id: 'dmg.notarization', status: 'FAIL' },
    ],
  });
  const content = completeReport({ omitted: distributionTrustSmokeIds });

  const accepted = verifySmokeReport({
    content,
    expectedCommit: commitSha,
    expectedArtifactSha256: artifactSha,
    candidateReport: unsignedCandidate,
    profile: 'test-plan',
  });

  assert.equal(accepted.status, 'PASS');
  assert.deepEqual(accepted.errors, []);
  for (const id of distributionTrustSmokeIds) {
    assert.equal(accepted.items[id], undefined);
  }

  const rejected = verifySmokeReport({
    content,
    expectedCommit: commitSha,
    expectedArtifactSha256: artifactSha,
    candidateReport: {
      ...unsignedCandidate,
      checks: unsignedCandidate.checks.map((check) => (
        check.id === 'bundle.metadata' ? { ...check, status: 'FAIL' } : check
      )),
    },
    profile: 'test-plan',
  });

  assert.equal(rejected.status, 'FAIL');
  assert.ok(rejected.errors.some((error) => error.includes('bundle.metadata')));

  const missingCheck = verifySmokeReport({
    content,
    expectedCommit: commitSha,
    expectedArtifactSha256: artifactSha,
    candidateReport: {
      ...unsignedCandidate,
      checks: unsignedCandidate.checks.filter((check) => check.id !== 'bundle.resources'),
    },
    profile: 'test-plan',
  });

  assert.equal(missingCheck.status, 'FAIL');
  assert.ok(missingCheck.errors.some((error) => error.includes('bundle.resources')));
});

test('release gate help requires a smoke report and documents candidate evidence', () => {
  const result = spawnSync(releaseGateScript, ['--help'], {
    cwd: repositoryRoot,
    encoding: 'utf8',
  });

  assert.equal(result.status, 0);
  assert.match(result.stdout, /--smoke-report/);
  assert.match(result.stdout, /--candidate-report/);
  assert.match(result.stdout, /candidate-report\.json/);
});

test('test-plan gate help documents its non-distribution scope', () => {
  const result = spawnSync(testPlanGateScript, ['--help'], {
    cwd: repositoryRoot,
    encoding: 'utf8',
  });

  assert.equal(result.status, 0);
  assert.match(result.stdout, /--smoke-report/);
  assert.match(result.stdout, /Developer ID/i);
  assert.match(result.stdout, /Gatekeeper/i);
  assert.match(result.stdout, /notarization/i);
  assert.match(result.stdout, /public release/i);
});

test('the repository smoke template is deliberately release-blocking until completed', () => {
  const content = readFileSync(
    path.join(repositoryRoot, 'docs', 'release', 'smoke-report-template.md'),
    'utf8',
  );
  const verification = verifySmokeReport({
    content,
    expectedCommit: commitSha,
    expectedArtifactSha256: artifactSha,
    candidateReport: candidateReport(),
  });

  assert.equal(verification.status, 'FAIL');
  assert.ok(verification.errors.some((error) => error.includes('Operator')));
  assert.ok(verification.errors.some((error) => error.includes('SMOKE-LAUNCH must be PASS')));
});
