import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { createRequire } from 'node:module';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';

const require = createRequire(import.meta.url);
const modulePath = path.resolve('scripts/universal-onnx-runtime.js');

function manifestEntry(name, target, contents) {
  return {
    name,
    target,
    sha256: createHash('sha256').update(contents).digest('hex'),
  };
}

test('pins both offline ONNX Runtime static archives', () => {
  const { ONNX_RUNTIME_MANIFEST } = require(modulePath);

  assert.deepEqual(
    ONNX_RUNTIME_MANIFEST.map(({ name, target }) => ({ name, target })),
    [
      { name: 'onnxruntime-aarch64-apple-darwin.a', target: 'aarch64-apple-darwin' },
      { name: 'onnxruntime-x86_64-apple-darwin.a', target: 'x86_64-apple-darwin' },
    ],
  );
  for (const entry of ONNX_RUNTIME_MANIFEST) {
    assert.match(entry.sha256, /^[a-f0-9]{64}$/);
  }
});

test('creates and verifies an atomic Universal ONNX library for Cargo', () => {
  const { prepareUniversalOnnxRuntime } = require(modulePath);
  const tempDir = mkdtempSync(path.join(os.tmpdir(), 'meetily-onnx-universal-'));
  const storeDir = path.join(tempDir, 'store');
  const outputRoot = path.join(tempDir, 'output');
  const arm = Buffer.from('arm64-static-archive');
  const intel = Buffer.from('x86-static-archive');
  const manifest = [
    manifestEntry('onnxruntime-aarch64-apple-darwin.a', 'aarch64-apple-darwin', arm),
    manifestEntry('onnxruntime-x86_64-apple-darwin.a', 'x86_64-apple-darwin', intel),
  ];
  const verifiedTargets = [];

  try {
    mkdirSync(storeDir, { recursive: true });
    writeFileSync(path.join(storeDir, manifest[0].name), arm);
    writeFileSync(path.join(storeDir, manifest[1].name), intel);

    const result = prepareUniversalOnnxRuntime({
      storeDir,
      outputRoot,
      manifest,
      verifyThin: (filePath, entry) => {
        assert.equal(
          createHash('sha256').update(readFileSync(filePath)).digest('hex'),
          entry.sha256,
        );
        verifiedTargets.push(entry.target);
      },
      createFat: (thinPaths, outputPath) => {
        assert.deepEqual(thinPaths, manifest.map((entry) => path.join(storeDir, entry.name)));
        writeFileSync(outputPath, 'verified-universal-archive');
      },
      verifyUniversal: (filePath) => {
        assert.equal(readFileSync(filePath, 'utf8'), 'verified-universal-archive');
      },
    });

    assert.equal(result, outputRoot);
    assert.deepEqual(verifiedTargets, manifest.map((entry) => entry.target));
    assert.equal(
      readFileSync(path.join(outputRoot, 'lib', 'libonnxruntime.a'), 'utf8'),
      'verified-universal-archive',
    );
    assert.equal(existsSync(path.join(outputRoot, 'lib', '.libonnxruntime.a.tmp')), false);
  } finally {
    rmSync(tempDir, { recursive: true, force: true });
  }
});

test('does not replace a prior ONNX library when new Universal verification fails', () => {
  const { prepareUniversalOnnxRuntime } = require(modulePath);
  const tempDir = mkdtempSync(path.join(os.tmpdir(), 'meetily-onnx-atomic-'));
  const storeDir = path.join(tempDir, 'store');
  const outputRoot = path.join(tempDir, 'output');
  const libDir = path.join(outputRoot, 'lib');
  const manifest = [
    manifestEntry('arm.a', 'aarch64-apple-darwin', Buffer.from('arm')),
    manifestEntry('intel.a', 'x86_64-apple-darwin', Buffer.from('intel')),
  ];

  try {
    mkdirSync(storeDir, { recursive: true });
    mkdirSync(libDir, { recursive: true });
    writeFileSync(path.join(storeDir, 'arm.a'), 'arm');
    writeFileSync(path.join(storeDir, 'intel.a'), 'intel');
    writeFileSync(path.join(libDir, 'libonnxruntime.a'), 'known-good');

    assert.throws(
      () => prepareUniversalOnnxRuntime({
        storeDir,
        outputRoot,
        manifest,
        verifyThin: () => {},
        createFat: (_thinPaths, outputPath) => writeFileSync(outputPath, 'corrupt'),
        verifyUniversal: () => { throw new Error('missing arm64 slice'); },
      }),
      /missing arm64 slice/,
    );
    assert.equal(readFileSync(path.join(libDir, 'libonnxruntime.a'), 'utf8'), 'known-good');
  } finally {
    rmSync(tempDir, { recursive: true, force: true });
  }
});

test('Universal build keeps Cargo offline and points ort-sys at the verified fat library', () => {
  const source = readFileSync(path.resolve('scripts/build-universal-macos.js'), 'utf8');

  assert.match(source, /prepareUniversalOnnxRuntime/);
  assert.match(source, /ORT_LIB_LOCATION/);
  assert.match(source, /CARGO_NET_OFFLINE:\s*'true'/);
});
