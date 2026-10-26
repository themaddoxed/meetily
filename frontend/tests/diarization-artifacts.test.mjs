import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createRequire } from 'node:module';
import test from 'node:test';

const require = createRequire(import.meta.url);
const {
  DIARIZATION_ARTIFACT_MANIFEST,
  provisionDiarizationResources,
  verifyDiarizationArtifact,
} = require('../scripts/diarization-artifacts');

const frontendRoot = path.resolve(import.meta.dirname, '..');

test('pins both native architectures and the two production diarization models', () => {
  const names = DIARIZATION_ARTIFACT_MANIFEST.map((entry) => entry.name);
  assert.deepEqual(names, [
    'sherpa-onnx-v1.13.2-osx-arm64-static-lib.tar.bz2',
    'sherpa-onnx-v1.13.2-osx-x64-static-lib.tar.bz2',
    'pyannote-segmentation-3.0.onnx',
    '3dspeaker-eres2net-base-16k.onnx',
  ]);

  assert.deepEqual(
    DIARIZATION_ARTIFACT_MANIFEST
      .filter((entry) => entry.kind === 'native-archive')
      .map((entry) => entry.target),
    ['aarch64-apple-darwin', 'x86_64-apple-darwin'],
  );
  assert.deepEqual(
    DIARIZATION_ARTIFACT_MANIFEST
      .filter((entry) => entry.kind === 'model')
      .map((entry) => entry.resourcePath),
    [
      'diarization/models/pyannote-segmentation-3.0.onnx',
      'diarization/models/3dspeaker-eres2net-base-16k.onnx',
    ],
  );

  for (const entry of DIARIZATION_ARTIFACT_MANIFEST) {
    assert.match(entry.url, /^https:\/\//);
    assert.doesNotMatch(entry.url, /\/(?:main|master)\//);
    assert.match(entry.sha256, /^[a-f0-9]{64}$/);
    assert.ok(Number.isSafeInteger(entry.size) && entry.size > 0);
    assert.match(entry.license.id, /^(?:Apache-2\.0|MIT)$/);
    assert.match(entry.license.source, /^https:\/\//);
    assert.doesNotMatch(entry.license.source, /\/(?:main|master)(?:\/|$)/);
  }
});

test('ships the reviewed license texts and aggregate notice', () => {
  for (const name of [
    'NOTICE.md',
    'licenses/Apache-2.0.txt',
    'licenses/MIT-pyannote.txt',
  ]) {
    const filePath = path.join(frontendRoot, 'src-tauri', 'resources', 'diarization', name);
    assert.equal(existsSync(filePath), true, `missing ${name}`);
    assert.ok(readFileSync(filePath, 'utf8').trim().length > 100, `${name} is unexpectedly short`);
  }
});

test('verifies exact size and checksum before copying model resources', () => {
  const tempDir = mkdtempSync(path.join(os.tmpdir(), 'meetily-diarization-artifacts-'));
  const storeDir = path.join(tempDir, 'store');
  const resourcesDir = path.join(tempDir, 'resources');
  const model = Buffer.from('verified diarization model');
  const native = Buffer.from('verified native archive');
  const manifest = [
    {
      kind: 'native-archive',
      name: 'native.tar.bz2',
      target: 'aarch64-apple-darwin',
      size: native.length,
      sha256: createHash('sha256').update(native).digest('hex'),
    },
    {
      kind: 'model',
      name: 'model.onnx',
      resourcePath: 'diarization/models/model.onnx',
      size: model.length,
      sha256: createHash('sha256').update(model).digest('hex'),
    },
  ];

  try {
    assert.throws(
      () => provisionDiarizationResources({ storeDir, resourcesDir, manifest }),
      /incomplete.*native\.tar\.bz2/i,
    );

    require('node:fs').mkdirSync(storeDir, { recursive: true });
    writeFileSync(path.join(storeDir, manifest[0].name), native);
    writeFileSync(path.join(storeDir, manifest[1].name), model);
    provisionDiarizationResources({ storeDir, resourcesDir, manifest });

    const destination = path.join(resourcesDir, manifest[1].resourcePath);
    assert.deepEqual(readFileSync(destination), model);
    assert.equal(existsSync(path.join(resourcesDir, 'native.tar.bz2')), false);

    writeFileSync(path.join(storeDir, manifest[1].name), Buffer.from('corrupt'));
    assert.throws(
      () => verifyDiarizationArtifact(path.join(storeDir, manifest[1].name), manifest[1]),
      /size mismatch|sha-256 mismatch/i,
    );
  } finally {
    rmSync(tempDir, { recursive: true, force: true });
  }
});

test('Universal builds require the local sherpa archive store and bundle verified models', () => {
  const buildSource = readFileSync(
    path.join(frontendRoot, 'scripts', 'build-universal-macos.js'),
    'utf8',
  );
  const provisionSource = readFileSync(
    path.join(frontendRoot, 'scripts', 'provision-universal-artifacts.js'),
    'utf8',
  );
  const tauriConfig = JSON.parse(readFileSync(
    path.join(frontendRoot, 'src-tauri', 'tauri.conf.json'),
    'utf8',
  ));
  const universalConfig = JSON.parse(readFileSync(
    path.join(frontendRoot, 'src-tauri', 'tauri.universal.conf.json'),
    'utf8',
  ));

  assert.match(buildSource, /CARGO_NET_OFFLINE:\s*'true'/);
  assert.match(buildSource, /env\.SHERPA_ONNX_ARCHIVE_DIR\s*=\s*artifactStoreDir/);
  assert.match(buildSource, /provisionDiarizationResources/);
  assert.match(provisionSource, /DIARIZATION_ARTIFACT_MANIFEST/);
  assert.match(provisionSource, /provisionDiarizationResources/);
  assert.equal(tauriConfig.bundle.resources['resources/diarization/'], 'diarization/');
  assert.equal(universalConfig.bundle.resources['resources/diarization/'], 'diarization/');
});
