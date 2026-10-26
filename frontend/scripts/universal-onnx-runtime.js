#!/usr/bin/env node
/* eslint-disable @typescript-eslint/no-require-imports -- CommonJS module consumed by existing build scripts. */

const { createHash } = require('crypto');
const { spawnSync } = require('child_process');
const {
  existsSync,
  mkdirSync,
  readFileSync,
  renameSync,
  rmSync,
  statSync,
} = require('fs');
const path = require('path');

const ONNX_RUNTIME_MANIFEST = [
  {
    name: 'onnxruntime-aarch64-apple-darwin.a',
    target: 'aarch64-apple-darwin',
    sha256: 'e5c83560aa9e88afa39d9dca9fb5f5a767e28adb5458d1c36fe0357131b6af8b',
  },
  {
    name: 'onnxruntime-x86_64-apple-darwin.a',
    target: 'x86_64-apple-darwin',
    sha256: 'f41370c27dd380419c6cbdfe4f78238613a49506c8f491c0cb07084edac9d561',
  },
];

const TARGET_ARCHITECTURES = {
  'aarch64-apple-darwin': 'arm64',
  'x86_64-apple-darwin': 'x86_64',
};

function runLipo(args) {
  const result = spawnSync('xcrun', ['lipo', ...args], {
    encoding: 'utf8',
    stdio: 'pipe',
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    throw new Error(result.stderr || `xcrun lipo failed with exit code ${result.status ?? 1}`);
  }
  return result.stdout.trim();
}

function readArchitectures(filePath) {
  return runLipo(['-archs', filePath]).split(/\s+/).filter(Boolean);
}

function verifySha256(filePath, entry) {
  if (!existsSync(filePath) || !statSync(filePath).isFile()) {
    throw new Error(`Missing required offline ONNX Runtime archive: ${entry.name}`);
  }
  const actualSha256 = createHash('sha256').update(readFileSync(filePath)).digest('hex');
  if (actualSha256 !== entry.sha256) {
    throw new Error(
      `SHA-256 mismatch for ${entry.name}: expected ${entry.sha256}, got ${actualSha256}`,
    );
  }
}

function verifyThinOnnxArchive(filePath, entry) {
  verifySha256(filePath, entry);
  const expectedArchitecture = TARGET_ARCHITECTURES[entry.target];
  const architectures = readArchitectures(filePath);
  if (!expectedArchitecture || architectures.length !== 1
    || architectures[0] !== expectedArchitecture) {
    throw new Error(
      `${entry.name} must contain only ${expectedArchitecture || entry.target}; found ${architectures.join(', ') || 'none'}`,
    );
  }
}

function verifyUniversalOnnxArchive(filePath) {
  const architectures = new Set(readArchitectures(filePath));
  if (architectures.size !== 2 || !architectures.has('arm64') || !architectures.has('x86_64')) {
    throw new Error(
      `Universal ONNX Runtime archive must contain arm64 and x86_64; found ${[...architectures].join(', ') || 'none'}`,
    );
  }
}

function createUniversalArchive(thinPaths, outputPath) {
  runLipo(['-create', ...thinPaths, '-output', outputPath]);
}

function prepareUniversalOnnxRuntime({
  storeDir,
  outputRoot,
  manifest = ONNX_RUNTIME_MANIFEST,
  verifyThin = verifyThinOnnxArchive,
  createFat = createUniversalArchive,
  verifyUniversal = verifyUniversalOnnxArchive,
}) {
  const thinPaths = manifest.map((entry) => path.join(storeDir, entry.name));
  for (const [index, entry] of manifest.entries()) {
    verifyThin(thinPaths[index], entry);
  }

  const libDir = path.join(outputRoot, 'lib');
  const outputPath = path.join(libDir, 'libonnxruntime.a');
  const temporaryPath = `${outputPath}.tmp-${process.pid}-${Date.now()}`;
  mkdirSync(libDir, { recursive: true });

  try {
    createFat(thinPaths, temporaryPath);
    verifyUniversal(temporaryPath);
    renameSync(temporaryPath, outputPath);
    verifyUniversal(outputPath);
  } finally {
    rmSync(temporaryPath, { force: true });
  }

  return outputRoot;
}

module.exports = {
  ONNX_RUNTIME_MANIFEST,
  prepareUniversalOnnxRuntime,
  verifySha256,
  verifyThinOnnxArchive,
  verifyUniversalOnnxArchive,
};
