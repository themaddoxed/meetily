#!/usr/bin/env node

const { createHash } = require('crypto');
const { spawnSync } = require('child_process');
const {
  chmodSync,
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  renameSync,
  rmSync,
  writeFileSync,
} = require('fs');
const path = require('path');

const { verifyThinMachO } = require('./macos-sidecar-tools');
// eslint-disable-next-line @typescript-eslint/no-require-imports -- This build script is CommonJS.
const {
  ONNX_RUNTIME_MANIFEST,
  verifyThinOnnxArchive,
} = require('./universal-onnx-runtime');
const { FFMPEG_MANIFEST } = require('./verify-universal-ffmpeg');
const {
  DIARIZATION_ARTIFACT_MANIFEST,
  provisionDiarizationResources,
  verifyDiarizationArtifact,
} = require('./diarization-artifacts');

function verifyArtifact(filePath, entry) {
  if (entry.kind === 'native-archive' || entry.kind === 'model') {
    verifyDiarizationArtifact(filePath, entry);
    return;
  }
  if (entry.kind === 'static-archive') {
    verifyThinOnnxArchive(filePath, entry);
    return;
  }
  verifyThinMachO(filePath, entry.target);
  const actualSha256 = createHash('sha256').update(readFileSync(filePath)).digest('hex');
  if (actualSha256 !== entry.sha256) {
    throw new Error(
      `SHA-256 mismatch for ${entry.name}: expected ${entry.sha256}, got ${actualSha256}`,
    );
  }
}

function copyArtifacts({ sourceDir, destinationDir, manifest, missingMessage }) {
  for (const entry of manifest) {
    const sourcePath = path.join(sourceDir, entry.name);
    if (!existsSync(sourcePath)) {
      throw new Error(`${missingMessage}: ${sourcePath}`);
    }
    verifyArtifact(sourcePath, entry);
  }

  mkdirSync(destinationDir, { recursive: true });
  for (const entry of manifest) {
    const sourcePath = path.join(sourceDir, entry.name);
    const destinationPath = path.join(destinationDir, entry.name);
    const temporaryPath = `${destinationPath}.tmp-${process.pid}-${Date.now()}`;
    try {
      copyFileSync(sourcePath, temporaryPath);
      chmodSync(temporaryPath, 0o755);
      verifyArtifact(temporaryPath, entry);
      renameSync(temporaryPath, destinationPath);
      verifyArtifact(destinationPath, entry);
    } finally {
      rmSync(temporaryPath, { force: true });
    }
  }
}

function provisionArtifacts({ storeDir, binariesDir, manifest = FFMPEG_MANIFEST }) {
  copyArtifacts({
    sourceDir: storeDir,
    destinationDir: binariesDir,
    manifest,
    missingMessage: 'Local Universal artifact store is incomplete',
  });
}

function initializeArtifactStore({
  sourceDir,
  storeDir,
  manifest = [
    ...FFMPEG_MANIFEST,
    ...ONNX_RUNTIME_MANIFEST.map((entry) => ({ ...entry, kind: 'static-archive' })),
    ...DIARIZATION_ARTIFACT_MANIFEST,
  ],
}) {
  copyArtifacts({
    sourceDir,
    destinationDir: storeDir,
    manifest,
    missingMessage: 'Universal artifact seed source is incomplete',
  });
  const manifestPath = path.join(storeDir, 'manifest.json');
  writeFileSync(manifestPath, `${JSON.stringify({ version: 1, artifacts: manifest }, null, 2)}\n`);
}

function getArtifactStoreDir(workspaceRoot) {
  const result = spawnSync('git', ['rev-parse', '--git-common-dir'], {
    cwd: workspaceRoot,
    encoding: 'utf8',
    stdio: 'pipe',
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    throw new Error(result.stderr || 'Unable to discover git common directory');
  }

  const commonDirValue = result.stdout.trim();
  const commonDir = path.isAbsolute(commonDirValue)
    ? commonDirValue
    : path.resolve(workspaceRoot, commonDirValue);
  return path.join(commonDir, 'local-artifacts', 'universal-macos', 'v1');
}

function getInitSourceArgument(argumentsAfterCommand) {
  return argumentsAfterCommand.find((argument) => argument !== '--');
}

function main() {
  const command = process.argv[2];
  const frontendRoot = path.resolve(__dirname, '..');
  const workspaceRoot = path.resolve(frontendRoot, '..');
  const storeDir = getArtifactStoreDir(workspaceRoot);
  const binariesDir = path.join(frontendRoot, 'src-tauri', 'binaries');
  const resourcesDir = path.join(frontendRoot, 'src-tauri', 'resources');

  if (command === 'seed') {
    provisionArtifacts({ storeDir, binariesDir });
    for (const entry of ONNX_RUNTIME_MANIFEST) {
      verifyThinOnnxArchive(path.join(storeDir, entry.name), entry);
    }
    provisionDiarizationResources({ storeDir, resourcesDir });
    console.log(`Universal artifacts provisioned from ${storeDir}`);
    return;
  }

  if (command === 'init') {
    const sourceArgument = getInitSourceArgument(process.argv.slice(3));
    if (!sourceArgument) {
      throw new Error('Usage: provision-universal-artifacts.js init <source-directory>');
    }
    initializeArtifactStore({
      sourceDir: path.resolve(process.cwd(), sourceArgument),
      storeDir,
    });
    console.log(`Universal artifact store initialized at ${storeDir}`);
    return;
  }

  throw new Error('Usage: provision-universal-artifacts.js <init|seed> [source-directory]');
}

if (require.main === module) {
  try {
    main();
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}

module.exports = {
  getArtifactStoreDir,
  getInitSourceArgument,
  initializeArtifactStore,
  provisionArtifacts,
  verifyArtifact,
};
