#!/usr/bin/env node
/* eslint-disable @typescript-eslint/no-require-imports -- CommonJS build helper. */

const { createHash } = require('crypto');
const {
  chmodSync,
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  renameSync,
  rmSync,
  statSync,
} = require('fs');
const path = require('path');

const SHERPA_LICENSE_SOURCE = 'https://github.com/k2-fsa/sherpa-onnx/blob/v1.13.2/LICENSE';
const PYANNOTE_REVISION = '340b52f1f5cd12d45a30fa284691417eaad2ff92';
const PYANNOTE_REPOSITORY = 'https://huggingface.co/csukuangfj/sherpa-onnx-pyannote-segmentation-3-0';
const THREE_D_SPEAKER_REVISION = '065629c313eaf1a01c65c640c46d77e61e9607b4';

const DIARIZATION_ARTIFACT_MANIFEST = [
  {
    kind: 'native-archive',
    name: 'sherpa-onnx-v1.13.2-osx-arm64-static-lib.tar.bz2',
    target: 'aarch64-apple-darwin',
    url: 'https://github.com/k2-fsa/sherpa-onnx/releases/download/v1.13.2/sherpa-onnx-v1.13.2-osx-arm64-static-lib.tar.bz2',
    size: 18_686_919,
    sha256: 'e2d704b01c392970ee7fb90d7e74fd854528a172de0381228e987b79ac479f8e',
    license: { id: 'Apache-2.0', source: SHERPA_LICENSE_SOURCE },
  },
  {
    kind: 'native-archive',
    name: 'sherpa-onnx-v1.13.2-osx-x64-static-lib.tar.bz2',
    target: 'x86_64-apple-darwin',
    url: 'https://github.com/k2-fsa/sherpa-onnx/releases/download/v1.13.2/sherpa-onnx-v1.13.2-osx-x64-static-lib.tar.bz2',
    size: 18_402_047,
    sha256: 'aba062391bd847bae6b839fd474f444442d1f45071f80ca0c429ec0fb23f9c52',
    license: { id: 'Apache-2.0', source: SHERPA_LICENSE_SOURCE },
  },
  {
    kind: 'model',
    name: 'pyannote-segmentation-3.0.onnx',
    resourcePath: 'diarization/models/pyannote-segmentation-3.0.onnx',
    url: `${PYANNOTE_REPOSITORY}/resolve/${PYANNOTE_REVISION}/model.onnx`,
    size: 5_992_778,
    sha256: 'fed22097bca974bad329a930b60865703766ff89f05fa09060bf6fd44e92e319',
    license: {
      id: 'MIT',
      source: `${PYANNOTE_REPOSITORY}/blob/${PYANNOTE_REVISION}/LICENSE`,
    },
  },
  {
    kind: 'model',
    name: '3dspeaker-eres2net-base-16k.onnx',
    resourcePath: 'diarization/models/3dspeaker-eres2net-base-16k.onnx',
    url: 'https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-recongition-models/3dspeaker_speech_eres2net_base_sv_zh-cn_3dspeaker_16k.onnx',
    size: 39_593_761,
    sha256: '1a331345f04805badbb495c775a6ddffcdd1a732567d5ec8b3d5749e3c7a5e4b',
    license: {
      id: 'Apache-2.0',
      source: `https://github.com/modelscope/3D-Speaker/blob/${THREE_D_SPEAKER_REVISION}/LICENSE`,
    },
  },
];

function verifyDiarizationArtifact(filePath, entry) {
  if (!existsSync(filePath) || !statSync(filePath).isFile()) {
    throw new Error(`Local diarization artifact store is incomplete: ${filePath}`);
  }
  const actualSize = statSync(filePath).size;
  if (actualSize !== entry.size) {
    throw new Error(
      `Size mismatch for ${entry.name}: expected ${entry.size}, got ${actualSize}`,
    );
  }
  const actualSha256 = createHash('sha256').update(readFileSync(filePath)).digest('hex');
  if (actualSha256 !== entry.sha256) {
    throw new Error(
      `SHA-256 mismatch for ${entry.name}: expected ${entry.sha256}, got ${actualSha256}`,
    );
  }
}

function provisionDiarizationResources({
  storeDir,
  resourcesDir,
  manifest = DIARIZATION_ARTIFACT_MANIFEST,
}) {
  for (const entry of manifest) {
    verifyDiarizationArtifact(path.join(storeDir, entry.name), entry);
  }

  const provisioned = [];
  for (const entry of manifest.filter((candidate) => candidate.kind === 'model')) {
    const sourcePath = path.join(storeDir, entry.name);
    const destinationPath = path.join(resourcesDir, entry.resourcePath);
    const temporaryPath = `${destinationPath}.tmp-${process.pid}-${Date.now()}`;
    mkdirSync(path.dirname(destinationPath), { recursive: true });
    try {
      copyFileSync(sourcePath, temporaryPath);
      chmodSync(temporaryPath, 0o644);
      verifyDiarizationArtifact(temporaryPath, entry);
      renameSync(temporaryPath, destinationPath);
      verifyDiarizationArtifact(destinationPath, entry);
      provisioned.push(destinationPath);
    } finally {
      rmSync(temporaryPath, { force: true });
    }
  }
  return provisioned;
}

module.exports = {
  DIARIZATION_ARTIFACT_MANIFEST,
  provisionDiarizationResources,
  verifyDiarizationArtifact,
};
