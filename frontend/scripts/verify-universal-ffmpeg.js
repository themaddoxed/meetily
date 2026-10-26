#!/usr/bin/env node

const { createHash } = require('crypto');
const { existsSync, readFileSync, statSync } = require('fs');
const path = require('path');

const { verifyThinMachO } = require('./macos-sidecar-tools');

const FFMPEG_MANIFEST = [
  {
    name: 'ffmpeg-aarch64-apple-darwin',
    target: 'aarch64-apple-darwin',
    sha256: '77d2c853f431318d55ec02676d9b2f185ebfdddb9f7677a251fbe453affe025a',
  },
  {
    name: 'ffmpeg-x86_64-apple-darwin',
    target: 'x86_64-apple-darwin',
    sha256: '430d60fbf419dab28daee9b679e7929a31ee9bae53f6e42e8ae26b725584290f',
  },
];

function verifyUniversalFfmpegSidecars(binariesDir) {
  for (const entry of FFMPEG_MANIFEST) {
    const { name } = entry;
    const binaryPath = path.join(binariesDir, name);

    if (!existsSync(binaryPath) || !statSync(binaryPath).isFile()) {
      throw new Error(`Missing required offline FFmpeg sidecar: ${name}`);
    }

    verifyThinMachO(binaryPath, entry.target);
    const binary = readFileSync(binaryPath);
    const actualSha256 = createHash('sha256').update(binary).digest('hex');
    if (actualSha256 !== entry.sha256) {
      throw new Error(
        `SHA-256 mismatch for ${name}: expected ${entry.sha256}, got ${actualSha256}`,
      );
    }
  }
}

if (require.main === module) {
  const binariesDir = process.argv[2];
  if (!binariesDir) {
    console.error('Usage: verify-universal-ffmpeg.js <binaries-directory>');
    process.exitCode = 1;
  } else {
    try {
      verifyUniversalFfmpegSidecars(path.resolve(binariesDir));
      console.log('Universal FFmpeg offline preflight passed.');
    } catch (error) {
      console.error(error.message);
      process.exitCode = 1;
    }
  }
}

module.exports = { FFMPEG_MANIFEST, verifyUniversalFfmpegSidecars };
