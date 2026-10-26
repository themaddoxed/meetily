#!/usr/bin/env node

import { createHash } from 'node:crypto';
import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export const REQUIRED_SMOKE_IDS = [
  'SMOKE-LAUNCH',
  'SMOKE-CLEAN-PERMISSIONS',
  'SMOKE-AUDIO-SOURCES',
  'SMOKE-MIC-CAPTURE',
  'SMOKE-SYSTEM-CAPTURE',
  'SMOKE-LIVE-TRANSCRIPT',
  'SMOKE-NORMAL-FINISH',
  'SMOKE-FORCED-QUIT-RECOVERY',
  'SMOKE-PERMANENT-SAVE',
  'SMOKE-SPEAKER-RENAME',
  'SMOKE-DIARIZATION-SUCCESS',
  'SMOKE-DIARIZATION-FAILURE',
  'SMOKE-SUMMARY-SUCCESS',
  'SMOKE-SUMMARY-FAILURE',
  'SMOKE-EXPORT-UI',
  'SMOKE-EXPORT-ARTIFACT',
  'SMOKE-OFFLINE',
  'SMOKE-RESTART-REOPEN',
  'SMOKE-UPGRADE-PREVIOUS',
  'SMOKE-OLD-DATA',
  'SMOKE-LOGS',
  'SMOKE-CHECKSUM',
  'SMOKE-SIGNATURE',
  'SMOKE-GATEKEEPER',
  'SMOKE-NOTARIZATION',
];

const DISTRIBUTION_TRUST_SMOKE_IDS = new Set([
  'SMOKE-SIGNATURE',
  'SMOKE-GATEKEEPER',
  'SMOKE-NOTARIZATION',
]);

const DISTRIBUTION_TRUST_CANDIDATE_CHECK_IDS = new Set([
  'signature.trust',
  'gatekeeper.assess',
  'notarization.validate',
  'dmg.signature.integrity',
  'dmg.signature.trust',
  'dmg.gatekeeper',
  'dmg.notarization',
]);

const REQUIRED_TEST_PLAN_CANDIDATE_CHECK_IDS = [
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

const METADATA_FIELDS = [
  'Operator',
  'Date',
  'Commit SHA',
  'Candidate report',
  'Artifact SHA-256',
  'Mac model',
  'macOS version',
  'Audio devices',
];

function isPlaceholder(value) {
  return (
    !value
    || /^<.*>$/.test(value)
    || /^(?:TODO|TBD|NOT_RUN|UNKNOWN|N\/A)$/i.test(value)
  );
}

function parseMetadata(content) {
  return Object.fromEntries(METADATA_FIELDS.map((field) => {
    const match = content.match(new RegExp(`^${field}:[ \\t]*(.*)$`, 'm'));
    return [field, match?.[1]?.trim() ?? ''];
  }));
}

function parseItems(content, errors) {
  const items = {};
  for (const line of content.split(/\r?\n/)) {
    if (!line.trim().startsWith('|')) continue;
    const cells = line.trim().slice(1, -1).split('|').map((cell) => cell.trim());
    if (cells.length < 3) continue;
    const [id, status, evidence] = cells;
    if (id === 'ID' || /^-+$/.test(id)) continue;
    if (!id.startsWith('SMOKE-')) continue;
    if (items[id]) {
      errors.push(`Duplicate smoke ID: ${id}`);
      continue;
    }
    items[id] = { status, evidence };
  }
  return items;
}

function candidateCheck(candidateReport, id) {
  return candidateReport?.checks?.find((check) => check.id === id);
}

export function verifySmokeReport({
  content,
  expectedCommit,
  expectedArtifactSha256,
  expectedSourceFingerprint,
  candidateReport,
  profile = 'public-release',
}) {
  const errors = [];
  const metadata = parseMetadata(content);
  const items = parseItems(content, errors);
  const isTestPlan = profile === 'test-plan';

  if (!['public-release', 'test-plan'].includes(profile)) {
    errors.push(`Unknown validation profile: ${profile}.`);
  }

  for (const field of METADATA_FIELDS) {
    if (isPlaceholder(metadata[field])) {
      errors.push(`${field} is required and cannot be a placeholder.`);
    }
  }
  if (metadata.Date && !/^\d{4}-\d{2}-\d{2}$/.test(metadata.Date)) {
    errors.push('Date must use YYYY-MM-DD.');
  }
  if (metadata['Commit SHA'] && !/^[a-f0-9]{40,64}$/i.test(metadata['Commit SHA'])) {
    errors.push('Commit SHA must be a full hexadecimal commit identifier.');
  }
  if (
    expectedCommit
    && metadata['Commit SHA']
    && metadata['Commit SHA'] !== expectedCommit
  ) {
    errors.push('Commit SHA does not match the candidate report.');
  }
  if (metadata['Artifact SHA-256'] && !/^[a-f0-9]{64}$/i.test(metadata['Artifact SHA-256'])) {
    errors.push('Artifact SHA-256 must be a 64-character hexadecimal checksum.');
  }
  if (
    expectedArtifactSha256
    && metadata['Artifact SHA-256']
    && metadata['Artifact SHA-256'].toLowerCase() !== expectedArtifactSha256.toLowerCase()
  ) {
    errors.push('Artifact SHA-256 does not match the verified candidate artifact.');
  }

  const requiredSmokeIds = isTestPlan
    ? REQUIRED_SMOKE_IDS.filter((id) => !DISTRIBUTION_TRUST_SMOKE_IDS.has(id))
    : REQUIRED_SMOKE_IDS;
  for (const id of requiredSmokeIds) {
    const item = items[id];
    if (!item) {
      errors.push(`Missing required smoke ID: ${id}`);
      continue;
    }
    if (!['PASS', 'FAIL', 'NOT_RUN'].includes(item.status)) {
      errors.push(`${id} has invalid status: ${item.status || '<blank>'}`);
    } else if (item.status !== 'PASS') {
      errors.push(`${id} must be PASS, got ${item.status}.`);
    }
    if (isPlaceholder(item.evidence)) {
      errors.push(`${id} requires evidence; blank or placeholder text is not proof.`);
    }
  }

  if (!candidateReport) {
    errors.push('Candidate report evidence is unavailable.');
  } else {
    if (candidateReport.schemaVersion !== 2) {
      errors.push('Candidate report must use schema version 2.');
    }
    if (!isTestPlan && candidateReport.status !== 'PASS') {
      errors.push(`Candidate report must be PASS, got ${candidateReport.status ?? '<missing>'}.`);
    }
    if (
      candidateReport.provenance?.mode !== 'normal'
      || candidateReport.provenance?.worktreeClean !== true
    ) {
      errors.push('Candidate JSON provenance must identify a normal clean release build.');
    }
    if (
      !isTestPlan
      && (
      candidateReport.release?.eligible !== true
      || candidateCheck(candidateReport, 'release.eligibility')?.status !== 'PASS'
      )
    ) {
      errors.push('Candidate JSON must be explicitly eligible for public release.');
    }
    if (candidateReport.commitSha !== expectedCommit) {
      errors.push('Candidate report commit does not match the release commit.');
    }
    if (
      expectedSourceFingerprint
      && candidateReport.provenance?.sourceFingerprint !== expectedSourceFingerprint
    ) {
      errors.push('Candidate report source fingerprint does not match the current release source.');
    }
    if (candidateReport.artifact?.sha256 !== expectedArtifactSha256) {
      errors.push('Candidate report checksum does not match the candidate artifact.');
    }
    if (isTestPlan) {
      for (const id of REQUIRED_TEST_PLAN_CANDIDATE_CHECK_IDS) {
        if (!candidateCheck(candidateReport, id)) {
          errors.push(`Missing required candidate check: ${id}.`);
        }
      }
      for (const check of candidateReport.checks ?? []) {
        if (
          !DISTRIBUTION_TRUST_CANDIDATE_CHECK_IDS.has(check.id)
          && check.status !== 'PASS'
        ) {
          errors.push(`Candidate check ${check.id ?? '<missing>'} must be PASS, got ${check.status ?? '<missing>'}.`);
        }
      }
    } else {
      if (
        candidateReport.signature?.kind !== 'DEVELOPER_ID'
        || candidateCheck(candidateReport, 'signature.verify')?.status !== 'PASS'
      ) {
        errors.push('Developer ID signature is not confirmed by candidate evidence.');
      }
      if (candidateCheck(candidateReport, 'gatekeeper.assess')?.status !== 'PASS') {
        errors.push('Gatekeeper PASS is not confirmed by candidate evidence.');
      }
      if (candidateCheck(candidateReport, 'notarization.validate')?.status !== 'PASS') {
        errors.push('Apple notarization PASS is not confirmed by candidate evidence.');
      }
    }
  }

  return {
    status: errors.length === 0 ? 'PASS' : 'FAIL',
    errors,
    metadata,
    items,
  };
}

function sha256(filePath) {
  return createHash('sha256').update(readFileSync(filePath)).digest('hex');
}

function requiredArgument(args, name) {
  const index = args.indexOf(name);
  if (index === -1 || !args[index + 1]) throw new Error(`Missing required argument: ${name}`);
  return args[index + 1];
}

function usage() {
  console.log('Usage: node scripts/verify-smoke-report.mjs --smoke <markdown> --candidate <json> --commit <sha> --source-fingerprint <sha256> [--profile public-release|test-plan]');
}

function main() {
  const args = process.argv.slice(2);
  if (args.includes('--help') || args.includes('-h')) {
    usage();
    return;
  }

  const smokePath = path.resolve(requiredArgument(args, '--smoke'));
  const candidatePath = path.resolve(requiredArgument(args, '--candidate'));
  const expectedCommit = requiredArgument(args, '--commit');
  const profile = args.includes('--profile')
    ? requiredArgument(args, '--profile')
    : 'public-release';
  const expectedSourceFingerprint = profile === 'test-plan'
    ? undefined
    : requiredArgument(args, '--source-fingerprint');
  const candidateReport = JSON.parse(readFileSync(candidatePath, 'utf8'));
  const artifactPath = path.isAbsolute(candidateReport.artifact?.path ?? '')
    ? candidateReport.artifact.path
    : path.resolve(path.dirname(candidatePath), candidateReport.artifact?.path ?? '');
  const externalErrors = [];
  let actualArtifactSha256 = '';
  if (!artifactPath || !existsSync(artifactPath)) {
    externalErrors.push(`Candidate artifact is missing: ${artifactPath || '<missing path>'}`);
  } else {
    actualArtifactSha256 = sha256(artifactPath);
    if (actualArtifactSha256 !== candidateReport.artifact?.sha256) {
      externalErrors.push('Candidate artifact checksum no longer matches candidate-report.json.');
    }
  }

  const verification = verifySmokeReport({
    content: readFileSync(smokePath, 'utf8'),
    expectedCommit,
    expectedArtifactSha256: actualArtifactSha256,
    expectedSourceFingerprint,
    candidateReport,
    profile,
  });
  verification.errors.unshift(...externalErrors);
  verification.status = verification.errors.length === 0 ? 'PASS' : 'FAIL';

  for (const error of verification.errors) console.error(`FAIL: ${error}`);
  const gateLabel = profile === 'test-plan' ? 'TEST PLAN GATE' : 'RELEASE GATE';
  console.log(`${gateLabel}: ${verification.status}`);
  if (verification.status !== 'PASS') process.exitCode = 1;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    main();
  } catch (error) {
    console.error(`FAIL: ${error.message}`);
    const profileIndex = process.argv.indexOf('--profile');
    const gateLabel = process.argv[profileIndex + 1] === 'test-plan'
      ? 'TEST PLAN GATE'
      : 'RELEASE GATE';
    console.log(`${gateLabel}: FAIL`);
    process.exitCode = 1;
  }
}
