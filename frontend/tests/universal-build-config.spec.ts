import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import {
  chmodSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { expect, test } from '@playwright/test';

import packageJson from '../package.json';

test('defines the Universal macOS Tauri build command', () => {
  expect(packageJson.scripts['tauri:build:universal']).toBe(
    'node scripts/build-universal-macos.js',
  );
});

test('prepares both architecture-specific sidecars offline before bundling', () => {
  const scriptPath = path.resolve('scripts/build-universal-macos.js');
  const source = existsSync(scriptPath) ? readFileSync(scriptPath, 'utf8') : '';

  expect(source).toContain('aarch64-apple-darwin');
  expect(source).toContain('x86_64-apple-darwin');
  expect(source).toContain('llama-helper-${target}');
  expect(source).toContain("CARGO_NET_OFFLINE: 'true'");
  expect(source).toContain("MEETILY_FFMPEG_OFFLINE_REQUIRED: 'true'");
  expect(source).toContain('verifyUniversalFfmpegSidecars(binariesDir)');
  expect(source).toContain('prepareUniversalOnnxRuntime');
  expect(source).toContain('ORT_LIB_LOCATION');
  expect(source).toContain('provisionDiarizationResources');
  expect(source).toContain('SHERPA_ONNX_ARCHIVE_DIR');
  expect(source).toContain("['build', '--offline', '--release', '--package', 'llama-helper'");
  expect(source).toContain("'--config', 'src-tauri/tauri.universal.conf.json'");
  expect(source).toContain('llama-helper-universal-apple-darwin');
  expect(source).toContain('filesEqual(source, destination)');
  expect(source).toContain('atomicCreateUniversal');
  expect(source).toContain('verifyThinMachO(source, target)');
  expect(source).toContain('verifyUniversalMachO');
  expect(source).not.toContain('if (helperChanged || !existsSync(universalHelper))');
  expect(source).toContain("run('hdiutil', ['create'");
  expect(source).toContain("run('du', ['-sk', stagingDir]");
  expect(source).toContain("'-size', `${dmgSizeMb}m`");
  expect(source).toContain("'-format', 'UDZO'");
  expect(source).toContain('meetily_${version}_universal.dmg');
});

test('routes llama-helper acceleration by architecture', () => {
  const scriptPath = path.resolve('scripts/build-universal-macos.js');
  const { helperCargoArgsForTarget } = require(scriptPath);

  expect(helperCargoArgsForTarget('x86_64-apple-darwin')).toEqual([
    'build', '--offline', '--release', '--package', 'llama-helper',
    '--target', 'x86_64-apple-darwin', '--no-default-features',
  ]);
  expect(helperCargoArgsForTarget('aarch64-apple-darwin')).toEqual([
    'build', '--offline', '--release', '--package', 'llama-helper',
    '--target', 'aarch64-apple-darwin', '--no-default-features',
    '--features', 'metal',
  ]);
  expect(() => helperCargoArgsForTarget('unknown-target')).toThrow(
    'Unsupported llama-helper target: unknown-target',
  );
});

test('atomically replaces a stale Universal helper from freshly verified thin inputs', () => {
  const toolsPath = path.resolve('scripts/macos-sidecar-tools.js');
  expect(existsSync(toolsPath)).toBe(true);

  const { atomicCreateUniversal, verifyUniversalMachO } = require(toolsPath);
  const tempDir = mkdtempSync(path.join(os.tmpdir(), 'meetily-llama-stale-'));
  const armPath = path.join(tempDir, 'llama-helper-aarch64-apple-darwin');
  const intelPath = path.join(tempDir, 'llama-helper-x86_64-apple-darwin');
  const outputPath = path.join(tempDir, 'llama-helper-universal-apple-darwin');

  try {
    writeThinMachO(armPath, 0x0100000c);
    writeThinMachO(intelPath, 0x01000007);
    writeFileSync(outputPath, 'stale-corrupt-helper');
    chmodSync(outputPath, 0o755);

    let createCount = 0;
    atomicCreateUniversal({
      thinBinaries: [
        { path: armPath, target: 'aarch64-apple-darwin' },
        { path: intelPath, target: 'x86_64-apple-darwin' },
      ],
      outputPath,
      createFat: (_thinPaths: string[], temporaryPath: string) => {
        createCount += 1;
        writeFileSync(temporaryPath, makeFatMachO());
      },
    });

    expect(createCount).toBe(1);
    expect(readFileSync(outputPath)).toEqual(makeFatMachO());
    verifyUniversalMachO(outputPath);
  } finally {
    rmSync(tempDir, { recursive: true, force: true });
  }
});

test('does not replace an existing helper when the new fat output is corrupt', () => {
  const toolsPath = path.resolve('scripts/macos-sidecar-tools.js');
  expect(existsSync(toolsPath)).toBe(true);

  const { atomicCreateUniversal } = require(toolsPath);
  const tempDir = mkdtempSync(path.join(os.tmpdir(), 'meetily-llama-atomic-'));
  const armPath = path.join(tempDir, 'llama-helper-aarch64-apple-darwin');
  const intelPath = path.join(tempDir, 'llama-helper-x86_64-apple-darwin');
  const outputPath = path.join(tempDir, 'llama-helper-universal-apple-darwin');

  try {
    writeThinMachO(armPath, 0x0100000c);
    writeThinMachO(intelPath, 0x01000007);
    writeFileSync(outputPath, 'existing-helper');

    expect(() => atomicCreateUniversal({
      thinBinaries: [
        { path: armPath, target: 'aarch64-apple-darwin' },
        { path: intelPath, target: 'x86_64-apple-darwin' },
      ],
      outputPath,
      createFat: (_thinPaths: string[], temporaryPath: string) => {
        writeFileSync(temporaryPath, 'corrupt-new-helper');
      },
    })).toThrow('Universal Mach-O header');
    expect(readFileSync(outputPath, 'utf8')).toBe('existing-helper');
  } finally {
    rmSync(tempDir, { recursive: true, force: true });
  }
});

test('accepts bounded FAT32 and FAT64 sidecars in both byte orders', () => {
  const { verifyUniversalMachO } = require(path.resolve('scripts/macos-sidecar-tools.js'));
  const tempDir = mkdtempSync(path.join(os.tmpdir(), 'meetily-fat-valid-'));

  try {
    for (const fat64 of [false, true]) {
      for (const littleEndian of [false, true]) {
        const binaryPath = path.join(tempDir, `valid-${fat64}-${littleEndian}`);
        writeExecutable(binaryPath, makeFatMachO({ fat64, littleEndian }));
        expect(() => verifyUniversalMachO(binaryPath)).not.toThrow();
      }
    }
  } finally {
    rmSync(tempDir, { recursive: true, force: true });
  }
});

test('rejects malformed or overlapping Universal Mach-O slices', () => {
  const { verifyUniversalMachO } = require(path.resolve('scripts/macos-sidecar-tools.js'));
  const tempDir = mkdtempSync(path.join(os.tmpdir(), 'meetily-fat-invalid-'));
  const cases = [
    {
      name: 'truncated-table',
      expected: 'architecture table is truncated',
      mutate: () => {
        const binary = Buffer.alloc(20);
        binary.writeUInt32BE(0xcafebabe, 0);
        binary.writeUInt32BE(2, 4);
        return binary;
      },
    },
    {
      name: 'zero-size',
      expected: 'has an empty slice',
      mutate: () => {
        const binary = makeFatMachO();
        binary.writeUInt32BE(0, 20);
        return binary;
      },
    },
    {
      name: 'out-of-bounds',
      expected: 'slice is out of bounds',
      mutate: () => {
        const binary = makeFatMachO();
        binary.writeUInt32BE(binary.length - 4, 16);
        return binary;
      },
    },
    {
      name: 'inside-table',
      expected: 'slice begins before the complete architecture table',
      mutate: () => {
        const binary = makeFatMachO();
        binary.writeUInt32BE(8, 16);
        return binary;
      },
    },
    {
      name: 'overlap',
      expected: 'slices overlap',
      mutate: () => {
        const binary = makeFatMachO();
        binary.writeUInt32BE(52, 36);
        return binary;
      },
    },
    {
      name: 'wrong-nested-magic',
      expected: 'slice has invalid Mach-O magic',
      mutate: () => {
        const binary = makeFatMachO();
        binary.fill(0, 48, 52);
        return binary;
      },
    },
    {
      name: 'wrong-nested-cpu',
      expected: 'slice CPU type does not match',
      mutate: () => {
        const binary = makeFatMachO();
        binary.writeUInt32LE(0x01000007, 60);
        return binary;
      },
    },
  ];

  try {
    for (const testCase of cases) {
      const binaryPath = path.join(tempDir, testCase.name);
      writeExecutable(binaryPath, testCase.mutate());
      expect(() => verifyUniversalMachO(binaryPath)).toThrow(testCase.expected);
    }
  } finally {
    rmSync(tempDir, { recursive: true, force: true });
  }
});

test('rejects duplicate FAT CPU entries explicitly', () => {
  const { verifyUniversalMachO } = require(path.resolve('scripts/macos-sidecar-tools.js'));
  const tempDir = mkdtempSync(path.join(os.tmpdir(), 'meetily-fat-duplicate-'));
  const binaryPath = path.join(tempDir, 'duplicate-x86_64');

  try {
    const binary = makeFatMachO();
    binary.writeUInt32BE(0x01000007, 28);
    writeExecutable(binaryPath, binary);

    expect(() => verifyUniversalMachO(binaryPath))
      .toThrow('contains a duplicate CPU slice');
  } finally {
    rmSync(tempDir, { recursive: true, force: true });
  }
});

test('provisions verified Universal artifacts from a durable local store', () => {
  const provisionPath = path.resolve('scripts/provision-universal-artifacts.js');
  expect(existsSync(provisionPath)).toBe(true);

  const { provisionArtifacts } = require(provisionPath);
  const tempDir = mkdtempSync(path.join(os.tmpdir(), 'meetily-artifact-store-'));
  const storeDir = path.join(tempDir, 'store');
  const binariesDir = path.join(tempDir, 'binaries');
  const arm = makeThinMachO(0x0100000c);
  const intel = makeThinMachO(0x01000007);
  const manifest = [
    manifestEntry('ffmpeg-aarch64-apple-darwin', 'aarch64-apple-darwin', arm),
    manifestEntry('ffmpeg-x86_64-apple-darwin', 'x86_64-apple-darwin', intel),
  ];

  try {
    mkdirSync(storeDir, { recursive: true });
    writeExecutable(path.join(storeDir, manifest[0].name), arm);
    writeExecutable(path.join(storeDir, manifest[1].name), intel);

    provisionArtifacts({ storeDir, binariesDir, manifest });

    expect(readFileSync(path.join(binariesDir, manifest[0].name))).toEqual(arm);
    expect(readFileSync(path.join(binariesDir, manifest[1].name))).toEqual(intel);
  } finally {
    rmSync(tempDir, { recursive: true, force: true });
  }
});

test('rejects a missing or corrupt durable Universal artifact store', () => {
  const provisionPath = path.resolve('scripts/provision-universal-artifacts.js');
  expect(existsSync(provisionPath)).toBe(true);

  const { provisionArtifacts } = require(provisionPath);
  const tempDir = mkdtempSync(path.join(os.tmpdir(), 'meetily-artifact-store-bad-'));
  const storeDir = path.join(tempDir, 'store');
  const binariesDir = path.join(tempDir, 'binaries');
  const arm = makeThinMachO(0x0100000c);
  const manifest = [
    manifestEntry('ffmpeg-aarch64-apple-darwin', 'aarch64-apple-darwin', arm),
  ];

  try {
    expect(() => provisionArtifacts({ storeDir, binariesDir, manifest }))
      .toThrow('Local Universal artifact store is incomplete');
    mkdirSync(storeDir, { recursive: true });
    writeExecutable(path.join(storeDir, manifest[0].name), Buffer.from('corrupt'));
    expect(() => provisionArtifacts({ storeDir, binariesDir, manifest }))
      .toThrow('invalid Mach-O magic');
  } finally {
    rmSync(tempDir, { recursive: true, force: true });
  }
});

test('exposes an explicit seed command backed by git-common-dir', () => {
  const provisionPath = path.resolve('scripts/provision-universal-artifacts.js');
  const source = existsSync(provisionPath) ? readFileSync(provisionPath, 'utf8') : '';
  const { getInitSourceArgument } = require(provisionPath);

  expect(packageJson.scripts['artifacts:seed:universal']).toBe(
    'node scripts/provision-universal-artifacts.js seed',
  );
  expect(source).toContain("['rev-parse', '--git-common-dir']");
  expect(source).toContain("'local-artifacts', 'universal-macos', 'v1'");
  expect(getInitSourceArgument(['src-tauri/binaries'])).toBe('src-tauri/binaries');
  expect(getInitSourceArgument(['--', 'src-tauri/binaries'])).toBe('src-tauri/binaries');
});

test('offline FFmpeg preflight fails before the build when an input is missing', () => {
  const binariesDir = mkdtempSync(path.join(os.tmpdir(), 'meetily-ffmpeg-missing-'));

  try {
    const result = spawnSync(
      process.execPath,
      [path.resolve('scripts/verify-universal-ffmpeg.js'), binariesDir],
      { encoding: 'utf8' },
    );

    expect(result.status).toBe(1);
    expect(result.stderr).toContain(
      'Missing required offline FFmpeg sidecar: ffmpeg-aarch64-apple-darwin',
    );
  } finally {
    rmSync(binariesDir, { recursive: true, force: true });
  }
});

test('offline FFmpeg preflight rejects a corrupt target input', () => {
  const binariesDir = mkdtempSync(path.join(os.tmpdir(), 'meetily-ffmpeg-corrupt-'));
  const armPath = path.join(binariesDir, 'ffmpeg-aarch64-apple-darwin');
  const intelPath = path.join(binariesDir, 'ffmpeg-x86_64-apple-darwin');

  try {
    writeFileSync(armPath, Buffer.from([0xcf, 0xfa, 0xed, 0xfe, 0x0c, 0, 0, 1]));
    writeFileSync(intelPath, Buffer.from([0xcf, 0xfa, 0xed, 0xfe, 0x07, 0, 0, 1]));
    chmodSync(armPath, 0o755);
    chmodSync(intelPath, 0o755);

    const result = spawnSync(
      process.execPath,
      [path.resolve('scripts/verify-universal-ffmpeg.js'), binariesDir],
      { encoding: 'utf8' },
    );

    expect(result.status).toBe(1);
    expect(result.stderr).toContain(
      'SHA-256 mismatch for ffmpeg-aarch64-apple-darwin',
    );
  } finally {
    rmSync(binariesDir, { recursive: true, force: true });
  }
});

test('omits updater artifacts from base and Universal bundle configs', () => {
  const configPath = path.resolve('src-tauri/tauri.universal.conf.json');
  const config = existsSync(configPath)
    ? JSON.parse(readFileSync(configPath, 'utf8'))
    : {};
  const releaseConfig = JSON.parse(
    readFileSync(path.resolve('src-tauri/tauri.conf.json'), 'utf8'),
  );

  expect(config.bundle).not.toHaveProperty('createUpdaterArtifacts');
  expect(releaseConfig.bundle).not.toHaveProperty('createUpdaterArtifacts');
  expect(releaseConfig.bundle.macOS.minimumSystemVersion).toBe('14.2');
  if (config.bundle.macOS?.minimumSystemVersion !== undefined) {
    expect(config.bundle.macOS.minimumSystemVersion).toBe('14.2');
  }
});

test('creates the Universal FFmpeg sidecar after both target binaries exist', () => {
  const source = readFileSync(
    path.resolve('src-tauri/build/ffmpeg.rs'),
    'utf8',
  );

  expect(source).toContain('ffmpeg-universal-apple-darwin');
  expect(source).toContain('ensure_universal_macos_binary');
  expect(source).toContain('Command::new("lipo")');
  expect(source).toContain('verify_ffmpeg_binary(&binary_path, &target)');
  expect(source).toContain('MEETILY_FFMPEG_OFFLINE_REQUIRED');
  expect(source).toContain('Offline Universal build requires a verified FFmpeg sidecar');
});

function makeThinMachO(cpuType: number): Buffer {
  const header = Buffer.alloc(8);
  header.writeUInt32LE(0xfeedfacf, 0);
  header.writeUInt32LE(cpuType, 4);
  return header;
}

function writeThinMachO(filePath: string, cpuType: number): void {
  writeExecutable(filePath, makeThinMachO(cpuType));
}

function writeExecutable(filePath: string, contents: Buffer): void {
  writeFileSync(filePath, contents);
  chmodSync(filePath, 0o755);
}

function makeFatMachO(
  { fat64 = false, littleEndian = false } = {},
): Buffer {
  const entrySize = fat64 ? 32 : 20;
  const tableEnd = 8 + 2 * entrySize;
  const fat = Buffer.alloc(tableEnd + 16);
  const writeUInt32 = littleEndian
    ? fat.writeUInt32LE.bind(fat)
    : fat.writeUInt32BE.bind(fat);
  const writeUInt64 = littleEndian
    ? fat.writeBigUInt64LE.bind(fat)
    : fat.writeBigUInt64BE.bind(fat);

  writeUInt32(fat64 ? 0xcafebabf : 0xcafebabe, 0);
  writeUInt32(2, 4);
  for (const [index, cpuType] of [0x01000007, 0x0100000c].entries()) {
    const base = 8 + index * entrySize;
    const offset = tableEnd + index * 8;
    writeUInt32(cpuType, base);
    writeUInt32(0, base + 4);
    if (fat64) {
      writeUInt64(BigInt(offset), base + 8);
      writeUInt64(8n, base + 16);
      writeUInt32(0, base + 24);
      writeUInt32(0, base + 28);
    } else {
      writeUInt32(offset, base + 8);
      writeUInt32(8, base + 12);
      writeUInt32(0, base + 16);
    }
  }
  makeThinMachO(0x01000007).copy(fat, tableEnd);
  makeThinMachO(0x0100000c).copy(fat, tableEnd + 8);
  return fat;
}

function manifestEntry(name: string, target: string, contents: Buffer) {
  return {
    name,
    target,
    sha256: createHash('sha256').update(contents).digest('hex'),
  };
}
