#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import {
  existsSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readlinkSync,
  readdirSync,
  rmSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import diarizationArtifacts from './diarization-artifacts.js';

const { DIARIZATION_ARTIFACT_MANIFEST } = diarizationArtifacts;

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const frontendRoot = path.resolve(scriptDirectory, '..');
const repositoryRoot = path.resolve(frontendRoot, '..');
const TEXT_EXTENSIONS = new Set(['.cjs', '.css', '.html', '.js', '.json', '.mjs', '.txt']);
const EXECUTABLES = ['meetily', 'llama-helper', 'ffmpeg'];
const UNIVERSAL_ARCHITECTURES = ['arm64', 'x86_64'];
const DIARIZATION_RUNTIME_MARKERS = [
  'pyannote-segmentation-3.0.onnx',
  '3dspeaker-eres2net-base-16k.onnx',
  'sherpa_onnx_1_13_2',
];
const AUDITED_THIRD_PARTY_BUILD_PREFIXES = {
  meetily: [
    `/${'Users'}/runner/work/ort-artifacts/ort-artifacts/`,
    `/${'Users'}/runner/work/onnxruntime-build/onnxruntime-build/`,
    `/${'Users'}/runner/work/sherpa-onnx/sherpa-onnx/`,
  ],
  ffmpeg: [`/${'Users'}/Shared/ffmpeg/`],
};
const DANGEROUS_ENTITLEMENTS = [
  'com.apple.security.get-task-allow',
  'com.apple.security.cs.disable-library-validation',
  'com.apple.security.cs.allow-unsigned-executable-memory',
  'com.apple.security.cs.allow-dyld-environment-variables',
];
const DIARIZATION_NOTICE_RESOURCES = [
  'diarization/NOTICE.md',
  'diarization/licenses/Apache-2.0.txt',
  'diarization/licenses/MIT-pyannote.txt',
];

function result(id, status, detail) {
  return { id, status, detail };
}

function hasSuccessfulEvidence(evidence) {
  return evidence && evidence.status === 0;
}

function commandOutput(evidence) {
  return `${evidence?.stdout ?? ''}\n${evidence?.stderr ?? ''}`.trim();
}

export function classifySignature(details) {
  if (/Authority=Developer ID Application:/i.test(details)) return 'DEVELOPER_ID';
  if (/Signature=adhoc|ad[ -]?hoc/i.test(details)) return 'AD_HOC';
  return 'UNSIGNED';
}

function hasHardenedRuntime(details) {
  return /flags=.*\bruntime\b/i.test(details);
}

function parseEntitlements(details) {
  const entitlements = {};
  const entryPattern = /<key>\s*([^<]+?)\s*<\/key>\s*(?:<(true|false)\s*\/>|<string>([\s\S]*?)<\/string>)/g;
  for (const match of details.matchAll(entryPattern)) {
    entitlements[match[1]] = match[2] ? match[2] === 'true' : match[3];
  }
  return entitlements;
}

function entitlementMismatches(actual, required) {
  const mismatches = Object.entries(required ?? {}).flatMap(([key, value]) => (
    actual[key] === value
      ? []
      : [`${key}: expected ${JSON.stringify(value)}, got ${actual[key] === undefined ? '<missing>' : JSON.stringify(actual[key])}`]
  ));
  for (const key of DANGEROUS_ENTITLEMENTS) {
    if (actual[key] === true) {
      mismatches.push(`${key}: dangerous entitlement must not be enabled`);
    }
  }
  return mismatches;
}

function updateFingerprintField(fingerprint, value) {
  const content = Buffer.isBuffer(value) ? value : Buffer.from(value);
  const length = Buffer.allocUnsafe(8);
  length.writeBigUInt64BE(BigInt(content.length));
  fingerprint.update(length);
  fingerprint.update(content);
}

export function bundleFingerprint(appPath) {
  const contentsPath = path.join(appPath, 'Contents');
  if (!existsSync(contentsPath) || !statSync(contentsPath).isDirectory()) {
    throw new Error('App bundle Contents directory is unavailable.');
  }

  const fingerprint = createHash('sha256');
  const visit = (directory) => {
    const entries = readdirSync(directory, { withFileTypes: true })
      .sort((left, right) => (
        left.name === right.name ? 0 : left.name < right.name ? -1 : 1
      ));
    for (const entry of entries) {
      const entryPath = path.join(directory, entry.name);
      const relativePath = path.relative(appPath, entryPath);
      const mode = (lstatSync(entryPath).mode & 0o7777).toString(8);

      if (entry.isDirectory()) {
        updateFingerprintField(fingerprint, 'directory');
        updateFingerprintField(fingerprint, relativePath);
        updateFingerprintField(fingerprint, mode);
        visit(entryPath);
      } else if (entry.isSymbolicLink()) {
        updateFingerprintField(fingerprint, 'link');
        updateFingerprintField(fingerprint, relativePath);
        updateFingerprintField(fingerprint, mode);
        updateFingerprintField(fingerprint, readlinkSync(entryPath));
      } else if (entry.isFile()) {
        updateFingerprintField(fingerprint, 'file');
        updateFingerprintField(fingerprint, relativePath);
        updateFingerprintField(fingerprint, mode);
        updateFingerprintField(fingerprint, readFileSync(entryPath));
      } else {
        throw new Error(`Unsupported app bundle entry: ${relativePath}`);
      }
    }
  };

  // Include embedded signatures as well as executables and resources so the
  // report binds the DMG to the exact app bytes that were verified.
  visit(contentsPath);
  return fingerprint.digest('hex');
}

export function selectDmgAppBundleEntry(entries, productName) {
  const expectedName = `${productName}.app`;
  const appEntries = entries
    .filter((entry) => entry.isDirectory() && entry.name.toLowerCase().endsWith('.app'));
  if (
    appEntries.length !== 1
    || appEntries[0].name.toLowerCase() !== expectedName.toLowerCase()
  ) {
    return null;
  }
  return appEntries[0].name;
}

function verifyMetadata(actual, expected) {
  const mismatches = [];
  for (const key of ['productName', 'version', 'bundleIdentifier', 'executableName', 'iconFile']) {
    if (actual?.[key] !== expected[key]) {
      mismatches.push(`${key}: expected ${expected[key]}, got ${actual?.[key] ?? '<missing>'}`);
    }
  }
  return mismatches;
}

function verifyExecutables(appPath) {
  const missing = [];
  for (const executable of EXECUTABLES) {
    const executablePath = path.join(appPath, 'Contents', 'MacOS', executable);
    if (
      !existsSync(executablePath)
      || !statSync(executablePath).isFile()
      || (statSync(executablePath).mode & 0o111) === 0
    ) {
      missing.push(executable);
    }
  }
  return missing;
}

function verifyResources(appPath, iconFile, templates, diarizationArtifacts) {
  const missing = [];
  const resources = path.join(appPath, 'Contents', 'Resources');
  if (!iconFile || path.basename(iconFile) !== iconFile
    || !existsSync(path.join(resources, iconFile))) {
    missing.push(iconFile || '<configured .icns icon>');
  }
  for (const template of templates) {
    if (!existsSync(path.join(resources, 'templates', template))) {
      missing.push(`templates/${template}`);
    }
  }
  for (const resource of DIARIZATION_NOTICE_RESOURCES) {
    if (!existsSync(path.join(resources, resource))) {
      missing.push(resource);
    }
  }
  for (const entry of diarizationArtifacts.filter(({ kind }) => kind === 'model')) {
    const modelPath = path.join(resources, entry.resourcePath);
    if (!existsSync(modelPath) || !statSync(modelPath).isFile()) {
      missing.push(entry.resourcePath);
      continue;
    }
    const actualSize = statSync(modelPath).size;
    const actualSha256 = sha256(modelPath);
    if (actualSize !== entry.size || actualSha256 !== entry.sha256) {
      missing.push(`${entry.resourcePath} (checksum or size mismatch)`);
    }
  }
  return missing;
}

function verifyArchitectures(architectures) {
  const missing = [];
  for (const executable of EXECUTABLES) {
    const actual = new Set(architectures?.[executable] ?? []);
    for (const architecture of UNIVERSAL_ARCHITECTURES) {
      if (!actual.has(architecture)) {
        missing.push(`${executable} missing ${architecture}`);
      }
    }
  }
  return missing;
}

function verifyDiarizationRuntimeSlices(runtimeSlices) {
  return UNIVERSAL_ARCHITECTURES.filter((architecture) => runtimeSlices?.[architecture] !== true);
}

function walkTextFiles(root) {
  if (!existsSync(root)) return [];
  const files = [];
  const pending = [root];
  while (pending.length > 0) {
    const current = pending.pop();
    for (const entry of readdirSync(current, { withFileTypes: true })) {
      const entryPath = path.join(current, entry.name);
      if (entry.isDirectory()) {
        pending.push(entryPath);
      } else if (
        entry.isFile()
        && TEXT_EXTENSIONS.has(path.extname(entry.name).toLowerCase())
        && statSync(entryPath).size <= 5_000_000
      ) {
        files.push(entryPath);
      }
    }
  }
  return files.sort();
}

function scanExecutableDeveloperPaths(appPath, repositoryPath) {
  const findings = [];
  const auditedPaths = new Set();
  const userMarker = Buffer.from('/Users/');
  const repositoryNeedle = repositoryPath ? Buffer.from(repositoryPath) : null;

  for (const executable of EXECUTABLES) {
    const executablePath = path.join(appPath, 'Contents', 'MacOS', executable);
    if (!existsSync(executablePath) || !statSync(executablePath).isFile()) continue;

    const content = readFileSync(executablePath);
    const relativePath = `Contents/MacOS/${executable}`;
    const allowedPrefixes = AUDITED_THIRD_PARTY_BUILD_PREFIXES[executable] ?? [];
    let offset = 0;
    let foundUnapprovedUserPath = false;
    while (offset < content.length) {
      const markerIndex = content.indexOf(userMarker, offset);
      if (markerIndex === -1) break;
      const usernameStart = markerIndex + userMarker.length;
      const usernameEnd = content.indexOf(0x2f, usernameStart);
      if (usernameEnd !== -1 && usernameEnd - usernameStart <= 128) {
        const username = content.subarray(usernameStart, usernameEnd).toString('ascii');
        if (/^[A-Za-z0-9._-]+$/.test(username)) {
          const approvedPrefix = allowedPrefixes.find((prefix) => (
            content.subarray(markerIndex, markerIndex + Buffer.byteLength(prefix))
              .equals(Buffer.from(prefix))
          ));
          if (approvedPrefix) {
            auditedPaths.add(`${relativePath} -> ${approvedPrefix}`);
          } else {
            findings.push(`${relativePath}: developer-home path (/${'Users'}/${username}/)`);
            foundUnapprovedUserPath = true;
            break;
          }
        }
      }
      offset = markerIndex + userMarker.length;
    }
    if (
      !foundUnapprovedUserPath
      && repositoryNeedle
      && content.includes(repositoryNeedle)
    ) {
      findings.push(`${relativePath}: repository path`);
    }
  }
  return { findings, auditedPaths: [...auditedPaths].sort() };
}

function scanBundle(appPath, repositoryPath) {
  const executableScan = scanExecutableDeveloperPaths(appPath, repositoryPath);
  const findings = [...executableScan.findings];
  const credentialPattern = /\b(?:api[_-]?key|access[_-]?token|auth[_-]?token|client[_-]?secret|password|secret)\b\s*[:=]\s*["'][A-Za-z0-9_+./=-]{20,}["']/i;
  const developerHomePattern = /(?:^|[^A-Za-z])\/Users\/[^/\s]+\//;
  const e2ePattern = /__emitTauriEvent|NEXT_PUBLIC_E2E_TESTING|E2EBootstrap/;

  for (const filePath of walkTextFiles(path.join(appPath, 'Contents'))) {
    const content = readFileSync(filePath, 'utf8');
    const relativePath = path.relative(appPath, filePath);
    if (
      developerHomePattern.test(content)
      || (repositoryPath && content.includes(repositoryPath))
    ) {
      findings.push(`${relativePath}: developer-home path`);
    }
    if (e2ePattern.test(content)) {
      findings.push(`${relativePath}: E2E bootstrap marker`);
    }
    if (credentialPattern.test(content)) {
      findings.push(`${relativePath}: credential literal`);
    }
  }
  return { findings, auditedPaths: executableScan.auditedPaths };
}

function sha256(filePath) {
  return createHash('sha256').update(readFileSync(filePath)).digest('hex');
}

export function verifyCandidate(input) {
  const checks = [];
  const provenance = input.provenance ?? {};
  const eligibilityErrors = [];
  if (provenance.mode !== 'normal') {
    eligibilityErrors.push(`mode must be normal, got ${provenance.mode ?? '<missing>'}`);
  }
  if (provenance.worktreeClean !== true) {
    eligibilityErrors.push('worktree must be clean');
  }
  if (!/^[A-Za-z0-9._-]+$/.test(provenance.runId ?? '')) {
    eligibilityErrors.push('runId is missing or invalid');
  }
  if (!/^[a-f0-9]{64}$/i.test(provenance.sourceFingerprint ?? '')) {
    eligibilityErrors.push('sourceFingerprint must be a SHA-256 value');
  }
  checks.push(result(
    'release.eligibility',
    eligibilityErrors.length === 0 ? 'PASS' : 'FAIL',
    eligibilityErrors.length === 0
      ? 'Candidate provenance is a normal build from a clean identified source.'
      : eligibilityErrors.join('; '),
  ));

  const metadataMismatches = verifyMetadata(input.metadata, input.expected);
  checks.push(result(
    'bundle.metadata',
    metadataMismatches.length === 0 ? 'PASS' : 'FAIL',
    metadataMismatches.length === 0 ? 'Bundle metadata matches release config.' : metadataMismatches.join('; '),
  ));

  const missingExecutables = verifyExecutables(input.appPath);
  checks.push(result(
    'bundle.executables',
    missingExecutables.length === 0 ? 'PASS' : 'FAIL',
    missingExecutables.length === 0
      ? 'Main executable and required sidecars are present and executable.'
      : `Missing or non-executable: ${missingExecutables.join(', ')}`,
  ));

  const missingResources = verifyResources(
    input.appPath,
    input.expected.iconFile,
    input.expected.templates ?? [],
    input.expected.diarizationArtifacts ?? DIARIZATION_ARTIFACT_MANIFEST,
  );
  checks.push(result(
    'bundle.resources',
    missingResources.length === 0 ? 'PASS' : 'FAIL',
    missingResources.length === 0
      ? 'Icon, templates, diarization licenses, and exact offline models are present.'
      : `Missing resources: ${missingResources.join(', ')}`,
  ));

  const missingArchitectures = verifyArchitectures(input.architectures);
  checks.push(result(
    'bundle.architectures',
    missingArchitectures.length === 0 ? 'PASS' : 'FAIL',
    missingArchitectures.length === 0
      ? 'All executables contain arm64 and x86_64 slices.'
      : missingArchitectures.join('; '),
  ));

  const missingDiarizationRuntimeSlices = verifyDiarizationRuntimeSlices(
    input.diarizationRuntimeSlices,
  );
  checks.push(result(
    'bundle.diarization-runtime',
    missingDiarizationRuntimeSlices.length === 0 ? 'PASS' : 'FAIL',
    missingDiarizationRuntimeSlices.length === 0
      ? 'Pinned offline diarization runtime markers are present in both executable slices.'
      : `Diarization runtime missing from: ${missingDiarizationRuntimeSlices.join(', ')}`,
  ));

  const bundleScan = scanBundle(input.appPath, input.repositoryPath);
  checks.push(result(
    'bundle.static-scan',
    bundleScan.findings.length === 0 ? 'PASS' : 'FAIL',
    bundleScan.findings.length === 0
      ? bundleScan.auditedPaths.length === 0
        ? 'No developer paths, E2E hooks, or high-confidence credential literals found.'
        : `No unapproved developer paths found. Audited third-party build paths: ${bundleScan.auditedPaths.join('; ')}`
      : bundleScan.findings.join('; '),
  ));

  const signatureDetails = commandOutput(input.evidence?.signatureDetails);
  const signatureKind = classifySignature(signatureDetails);
  const signatureVerification = input.evidence?.signatureVerification;
  const signatureIntegrityStatus = !signatureVerification
    ? 'NOT_RUN'
    : hasSuccessfulEvidence(signatureVerification) ? 'PASS' : 'FAIL';
  const signatureIntegrityDetail = !signatureVerification
    ? 'codesign integrity evidence is unavailable.'
    : commandOutput(signatureVerification) || `exit ${signatureVerification.status}`;
  checks.push(result(
    'signature.integrity',
    signatureIntegrityStatus,
    signatureIntegrityDetail,
  ));
  // Retained for compatibility with existing smoke-report evidence.
  checks.push(result(
    'signature.verify',
    signatureIntegrityStatus,
    signatureIntegrityDetail,
  ));

  const signatureTrustStatus = !input.evidence?.signatureDetails
    ? 'NOT_RUN'
    : hasSuccessfulEvidence(input.evidence.signatureDetails) && signatureKind === 'DEVELOPER_ID'
      ? 'PASS'
      : 'FAIL';
  checks.push(result(
    'signature.trust',
    signatureTrustStatus,
    signatureTrustStatus === 'NOT_RUN'
      ? 'codesign identity evidence is unavailable.'
      : `${signatureKind}: ${signatureDetails || `exit ${input.evidence.signatureDetails.status}`}`,
  ));

  const hardenedRuntimeStatus = !input.evidence?.signatureDetails
    ? 'NOT_RUN'
    : hasSuccessfulEvidence(input.evidence.signatureDetails) && hasHardenedRuntime(signatureDetails)
      ? 'PASS'
      : 'FAIL';
  checks.push(result(
    'signature.hardened-runtime',
    hardenedRuntimeStatus,
    hardenedRuntimeStatus === 'NOT_RUN'
      ? 'codesign runtime evidence is unavailable.'
      : hardenedRuntimeStatus === 'PASS'
        ? 'Packaged app signature has the hardened runtime flag.'
        : 'Packaged app signature does not confirm the hardened runtime flag.',
  ));

  const entitlementsEvidence = input.evidence?.entitlements;
  const actualEntitlements = parseEntitlements(commandOutput(entitlementsEvidence));
  const entitlementErrors = entitlementMismatches(
    actualEntitlements,
    input.expected.requiredEntitlements,
  );
  const entitlementsStatus = !entitlementsEvidence
    ? 'NOT_RUN'
    : hasSuccessfulEvidence(entitlementsEvidence) && entitlementErrors.length === 0
      ? 'PASS'
      : 'FAIL';
  checks.push(result(
    'signature.entitlements',
    entitlementsStatus,
    entitlementsStatus === 'NOT_RUN'
      ? 'Embedded entitlements evidence is unavailable.'
      : entitlementsStatus === 'PASS'
        ? 'Packaged app contains all required entitlements.'
        : entitlementErrors.length > 0
          ? entitlementErrors.join('; ')
          : commandOutput(entitlementsEvidence) || `exit ${entitlementsEvidence.status}`,
  ));

  const gatekeeper = input.evidence?.gatekeeper;
  const gatekeeperStatus = !gatekeeper
    ? 'NOT_RUN'
    : hasSuccessfulEvidence(gatekeeper) ? 'PASS' : 'FAIL';
  checks.push(result(
    'gatekeeper.assess',
    gatekeeperStatus,
    gatekeeper ? commandOutput(gatekeeper) || `exit ${gatekeeper.status}` : 'Gatekeeper evidence is unavailable.',
  ));

  const notarization = input.evidence?.notarization;
  const notarizationStatus = !gatekeeper || !notarization
    ? 'NOT_RUN'
    : hasSuccessfulEvidence(notarization) ? 'PASS' : 'FAIL';
  checks.push(result(
    'notarization.validate',
    notarizationStatus,
    notarizationStatus === 'NOT_RUN'
      ? 'Notarization was not verified because required evidence is unavailable.'
      : commandOutput(notarization) || `exit ${notarization.status}`,
  ));

  const dmgVerification = input.evidence?.dmgVerification;
  const dmgStatus = !dmgVerification
    ? 'NOT_RUN'
    : hasSuccessfulEvidence(dmgVerification) ? 'PASS' : 'FAIL';
  checks.push(result(
    'dmg.structure',
    dmgStatus,
    dmgVerification
      ? commandOutput(dmgVerification) || `exit ${dmgVerification.status}`
      : 'DMG verification evidence is unavailable.',
  ));
  // Retained for compatibility with existing candidate reports and tooling.
  checks.push(result(
    'dmg.verify',
    dmgStatus,
    dmgVerification
      ? commandOutput(dmgVerification) || `exit ${dmgVerification.status}`
      : 'DMG verification evidence is unavailable.',
  ));

  const dmgSignatureDetails = commandOutput(input.evidence?.dmgSignatureDetails);
  const dmgSignatureKind = classifySignature(dmgSignatureDetails);
  const dmgSignatureVerification = input.evidence?.dmgSignatureVerification;
  const dmgIntegrityStatus = !dmgSignatureVerification
    ? 'NOT_RUN'
    : hasSuccessfulEvidence(dmgSignatureVerification) ? 'PASS' : 'FAIL';
  checks.push(result(
    'dmg.signature.integrity',
    dmgIntegrityStatus,
    dmgIntegrityStatus === 'NOT_RUN'
      ? 'DMG codesign integrity evidence is unavailable.'
      : commandOutput(dmgSignatureVerification) || `exit ${dmgSignatureVerification.status}`,
  ));

  const dmgTrustStatus = !input.evidence?.dmgSignatureDetails
    ? 'NOT_RUN'
    : hasSuccessfulEvidence(input.evidence.dmgSignatureDetails) && dmgSignatureKind === 'DEVELOPER_ID'
      ? 'PASS'
      : 'FAIL';
  checks.push(result(
    'dmg.signature.trust',
    dmgTrustStatus,
    dmgTrustStatus === 'NOT_RUN'
      ? 'DMG codesign identity evidence is unavailable.'
      : `${dmgSignatureKind}: ${dmgSignatureDetails || `exit ${input.evidence.dmgSignatureDetails.status}`}`,
  ));

  const dmgGatekeeper = input.evidence?.dmgGatekeeper;
  const dmgGatekeeperStatus = !dmgGatekeeper
    ? 'NOT_RUN'
    : hasSuccessfulEvidence(dmgGatekeeper) ? 'PASS' : 'FAIL';
  checks.push(result(
    'dmg.gatekeeper',
    dmgGatekeeperStatus,
    dmgGatekeeper
      ? commandOutput(dmgGatekeeper) || `exit ${dmgGatekeeper.status}`
      : 'DMG Gatekeeper evidence is unavailable.',
  ));

  const dmgNotarization = input.evidence?.dmgNotarization;
  const dmgNotarizationStatus = !dmgNotarization
    ? 'NOT_RUN'
    : hasSuccessfulEvidence(dmgNotarization) ? 'PASS' : 'FAIL';
  checks.push(result(
    'dmg.notarization',
    dmgNotarizationStatus,
    dmgNotarization
      ? commandOutput(dmgNotarization) || `exit ${dmgNotarization.status}`
      : 'DMG notarization evidence is unavailable.',
  ));

  const dmgBundleEvidence = input.evidence?.dmgBundleFingerprint;
  let dmgBundleStatus = 'FAIL';
  let dmgBundleDetail = 'Mounted DMG app fingerprint does not match the verified app.';
  if (!dmgBundleEvidence || !input.bundleFingerprint) {
    dmgBundleStatus = 'NOT_RUN';
    dmgBundleDetail = 'Mounted DMG app fingerprint evidence is unavailable.';
  } else if (!hasSuccessfulEvidence(dmgBundleEvidence)) {
    dmgBundleDetail = dmgBundleEvidence.detail || `DMG app inspection exited ${dmgBundleEvidence.status}.`;
  } else if (dmgBundleEvidence.fingerprint === input.bundleFingerprint) {
    dmgBundleStatus = 'PASS';
    dmgBundleDetail = 'Mounted DMG app matches the verified app bundle fingerprint.';
  }
  checks.push(result(
    'dmg.bundle-match',
    dmgBundleStatus,
    dmgBundleDetail,
  ));

  const artifactExists = existsSync(input.dmgPath) && statSync(input.dmgPath).isFile();
  const releaseEligible = checks.every((check) => check.status === 'PASS') && artifactExists;
  const report = {
    schemaVersion: 2,
    generatedAt: new Date().toISOString(),
    status: releaseEligible ? 'PASS' : 'FAIL',
    commitSha: input.commitSha,
    provenance: {
      mode: provenance.mode,
      worktreeClean: provenance.worktreeClean,
      runId: provenance.runId,
      sourceFingerprint: provenance.sourceFingerprint,
    },
    release: {
      eligible: releaseEligible,
    },
    artifact: {
      path: input.dmgPath,
      sha256: artifactExists ? sha256(input.dmgPath) : null,
      sizeBytes: artifactExists ? statSync(input.dmgPath).size : null,
      embeddedBundleFingerprint: dmgBundleEvidence?.fingerprint ?? null,
    },
    bundle: {
      path: input.appPath,
      fingerprint: input.bundleFingerprint,
      metadata: input.metadata,
      architectures: input.architectures,
      diarizationRuntimeSlices: input.diarizationRuntimeSlices,
    },
    signature: {
      kind: signatureKind,
    },
    dmgSignature: {
      kind: dmgSignatureKind,
    },
    checks,
  };
  return report;
}

export function writeCandidateReport(report, outputPath) {
  mkdirSync(path.dirname(outputPath), { recursive: true });
  writeFileSync(outputPath, `${JSON.stringify(report, null, 2)}\n`);
}

function runCommand(command, args) {
  const commandResult = spawnSync(command, args, { encoding: 'utf8' });
  if (commandResult.error) return undefined;
  return {
    status: commandResult.status ?? 1,
    stdout: commandResult.stdout ?? '',
    stderr: commandResult.stderr ?? '',
  };
}

function requiredArgument(args, name) {
  const index = args.indexOf(name);
  if (index === -1 || !args[index + 1]) {
    throw new Error(`Missing required argument: ${name}`);
  }
  return args[index + 1];
}

function inspectMetadata(appPath) {
  const plistPath = path.join(appPath, 'Contents', 'Info.plist');
  const inspection = runCommand('plutil', ['-convert', 'json', '-o', '-', plistPath]);
  if (!hasSuccessfulEvidence(inspection)) {
    throw new Error(`Unable to read bundle metadata: ${commandOutput(inspection)}`);
  }
  const plist = JSON.parse(inspection.stdout);
  return {
    productName: plist.CFBundleName ?? plist.CFBundleDisplayName,
    version: plist.CFBundleShortVersionString,
    bundleIdentifier: plist.CFBundleIdentifier,
    executableName: plist.CFBundleExecutable,
    iconFile: plist.CFBundleIconFile,
  };
}

function inspectArchitectures(appPath) {
  return Object.fromEntries(EXECUTABLES.map((executable) => {
    const executablePath = path.join(appPath, 'Contents', 'MacOS', executable);
    const inspection = runCommand('lipo', ['-archs', executablePath]);
    const architectures = hasSuccessfulEvidence(inspection)
      ? inspection.stdout.trim().split(/\s+/).filter(Boolean)
      : [];
    return [executable, architectures];
  }));
}

function inspectDiarizationRuntimeSlices(appPath) {
  const executablePath = path.join(appPath, 'Contents', 'MacOS', 'meetily');
  const temporaryRoot = mkdtempSync(path.join(os.tmpdir(), 'meetily-diarization-slices-'));
  try {
    return Object.fromEntries(UNIVERSAL_ARCHITECTURES.map((architecture) => {
      const thinPath = path.join(temporaryRoot, `meetily-${architecture}`);
      const extraction = runCommand('lipo', [
        executablePath,
        '-thin',
        architecture,
        '-output',
        thinPath,
      ]);
      if (!hasSuccessfulEvidence(extraction) || !existsSync(thinPath)) {
        return [architecture, false];
      }
      const content = readFileSync(thinPath);
      return [
        architecture,
        DIARIZATION_RUNTIME_MARKERS.every((marker) => content.includes(Buffer.from(marker))),
      ];
    }));
  } finally {
    rmSync(temporaryRoot, { recursive: true, force: true });
  }
}

function inspectRequiredEntitlements(entitlementsPath) {
  const inspection = runCommand('plutil', [
    '-convert',
    'json',
    '-o',
    '-',
    entitlementsPath,
  ]);
  if (!hasSuccessfulEvidence(inspection)) {
    throw new Error(`Unable to read required entitlements: ${commandOutput(inspection)}`);
  }
  return JSON.parse(inspection.stdout);
}

function inspectDmgBundleFingerprint(dmgPath, productName) {
  const temporaryRoot = mkdtempSync(path.join(os.tmpdir(), 'meetily-dmg-'));
  const mountPoint = path.join(temporaryRoot, 'mount');
  mkdirSync(mountPoint);
  let attached = false;
  let detached = true;
  let inspection = {
    status: 1,
    detail: 'DMG app inspection did not complete.',
  };

  try {
    const attach = runCommand('hdiutil', [
      'attach',
      '-readonly',
      '-nobrowse',
      '-mountpoint',
      mountPoint,
      dmgPath,
    ]);
    if (!attach) {
      inspection.detail = 'hdiutil attach is unavailable.';
    } else if (!hasSuccessfulEvidence(attach)) {
      inspection.detail = commandOutput(attach) || `hdiutil attach exited ${attach.status}.`;
    } else {
      attached = true;
      detached = false;
      const expectedName = `${productName}.app`;
      const appEntryName = selectDmgAppBundleEntry(
        readdirSync(mountPoint, { withFileTypes: true }),
        productName,
      );
      if (!appEntryName) {
        inspection.detail = `DMG must contain exactly one root app named ${expectedName}.`;
      } else {
        inspection = {
          status: 0,
          fingerprint: bundleFingerprint(path.join(mountPoint, appEntryName)),
          detail: `Mounted DMG contains ${appEntryName}.`,
        };
      }
    }
  } catch (error) {
    inspection = {
      status: 1,
      detail: `DMG app inspection failed: ${error.message}`,
    };
  } finally {
    if (attached) {
      const detach = runCommand('hdiutil', ['detach', mountPoint]);
      if (hasSuccessfulEvidence(detach)) {
        detached = true;
      } else {
        const forcedDetach = runCommand('hdiutil', ['detach', '-force', mountPoint]);
        detached = hasSuccessfulEvidence(forcedDetach);
        inspection = {
          ...inspection,
          status: 1,
          detail: detached
            ? 'Normal DMG detach failed; forced cleanup succeeded.'
            : `DMG detach failed; manual cleanup may be required: ${commandOutput(forcedDetach) || commandOutput(detach) || 'no command evidence'}`,
        };
      }
    }

    if (!attached || detached) {
      try {
        rmSync(temporaryRoot, { recursive: true, force: true });
      } catch (error) {
        inspection = {
          ...inspection,
          status: 1,
          detail: `DMG temporary directory cleanup failed: ${error.message}`,
        };
      }
    }
  }

  return inspection;
}

function usage() {
  console.log('Usage: node scripts/verify-release-candidate.mjs --app <path> --dmg <path> --commit <sha> --mode <normal|diagnostic> --worktree-clean <true|false> --run-id <id> --source-fingerprint <sha256> --output <json>');
}

function main() {
  const args = process.argv.slice(2);
  if (args.includes('--help') || args.includes('-h')) {
    usage();
    return;
  }

  const appPath = path.resolve(requiredArgument(args, '--app'));
  const dmgPath = path.resolve(requiredArgument(args, '--dmg'));
  const commitSha = requiredArgument(args, '--commit');
  const mode = requiredArgument(args, '--mode');
  if (!['normal', 'diagnostic'].includes(mode)) {
    throw new Error(`Invalid --mode: ${mode}`);
  }
  const worktreeCleanValue = requiredArgument(args, '--worktree-clean');
  if (!['true', 'false'].includes(worktreeCleanValue)) {
    throw new Error('--worktree-clean must be true or false.');
  }
  const runId = requiredArgument(args, '--run-id');
  const sourceFingerprint = requiredArgument(args, '--source-fingerprint');
  const outputPath = path.resolve(requiredArgument(args, '--output'));
  const packageJson = JSON.parse(readFileSync(path.join(frontendRoot, 'package.json'), 'utf8'));
  const tauriConfig = JSON.parse(readFileSync(path.join(frontendRoot, 'src-tauri', 'tauri.conf.json'), 'utf8'));
  const templatesDirectory = path.join(frontendRoot, 'src-tauri', 'templates');
  const templates = readdirSync(templatesDirectory)
    .filter((name) => name.endsWith('.json'))
    .sort();
  const configuredIcon = tauriConfig.bundle?.icon
    ?.find((name) => name.endsWith('.icns'));
  if (!configuredIcon) {
    throw new Error('Tauri release config does not declare an .icns icon.');
  }
  const entitlementsFile = tauriConfig.bundle?.macOS?.entitlements;
  if (!entitlementsFile) {
    throw new Error('Tauri release config does not declare macOS entitlements.');
  }
  const requiredEntitlements = inspectRequiredEntitlements(
    path.join(frontendRoot, 'src-tauri', entitlementsFile),
  );
  const verifiedBundleFingerprint = bundleFingerprint(appPath);
  const dmgBundleFingerprint = inspectDmgBundleFingerprint(
    dmgPath,
    tauriConfig.productName,
  );

  const report = verifyCandidate({
    appPath,
    dmgPath,
    commitSha,
    provenance: {
      mode,
      worktreeClean: worktreeCleanValue === 'true',
      runId,
      sourceFingerprint,
    },
    repositoryPath: repositoryRoot,
    bundleFingerprint: verifiedBundleFingerprint,
    expected: {
      productName: tauriConfig.productName,
      version: packageJson.version,
      bundleIdentifier: tauriConfig.identifier,
      executableName: tauriConfig.productName,
      iconFile: path.basename(configuredIcon),
      templates,
      requiredEntitlements,
    },
    metadata: inspectMetadata(appPath),
    architectures: inspectArchitectures(appPath),
    diarizationRuntimeSlices: inspectDiarizationRuntimeSlices(appPath),
    evidence: {
      signatureDetails: runCommand('codesign', ['-dv', '--verbose=4', appPath]),
      signatureVerification: runCommand('codesign', ['--verify', '--deep', '--strict', '--verbose=2', appPath]),
      entitlements: runCommand('codesign', ['-d', '--entitlements', '-', '--xml', appPath]),
      gatekeeper: runCommand('spctl', ['--assess', '--type', 'execute', '--verbose=4', appPath]),
      notarization: runCommand('xcrun', ['stapler', 'validate', appPath]),
      dmgVerification: runCommand('hdiutil', ['verify', dmgPath]),
      dmgSignatureDetails: runCommand('codesign', ['-dv', '--verbose=4', dmgPath]),
      dmgSignatureVerification: runCommand('codesign', ['--verify', '--verbose=2', dmgPath]),
      dmgGatekeeper: runCommand('spctl', [
        '--assess',
        '--type',
        'open',
        '--context',
        'context:primary-signature',
        '--verbose=4',
        dmgPath,
      ]),
      dmgNotarization: runCommand('xcrun', ['stapler', 'validate', dmgPath]),
      dmgBundleFingerprint,
    },
  });

  writeCandidateReport(report, outputPath);
  for (const check of report.checks) {
    console.log(`${check.status} ${check.id}: ${check.detail}`);
  }
  console.log(`Candidate report: ${outputPath}`);
  if (report.status !== 'PASS') process.exitCode = 1;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    main();
  } catch (error) {
    console.error(`Candidate verification failed: ${error.message}`);
    process.exitCode = 1;
  }
}
