#!/usr/bin/env node

const { spawnSync } = require('child_process');
const {
  chmodSync,
  copyFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  statSync,
  symlinkSync,
} = require('fs');
const os = require('os');
const path = require('path');

const { version } = require('../package.json');
const {
  atomicCreateUniversal,
  verifyThinMachO,
  verifyUniversalMachO,
} = require('./macos-sidecar-tools');
// eslint-disable-next-line @typescript-eslint/no-require-imports -- This build script is CommonJS.
const { getArtifactStoreDir } = require('./provision-universal-artifacts');
const { provisionDiarizationResources } = require('./diarization-artifacts');
// eslint-disable-next-line @typescript-eslint/no-require-imports -- This build script is CommonJS.
const { prepareUniversalOnnxRuntime } = require('./universal-onnx-runtime');
const { verifyUniversalFfmpegSidecars } = require('./verify-universal-ffmpeg');

const targets = ['aarch64-apple-darwin', 'x86_64-apple-darwin'];
const frontendRoot = path.resolve(__dirname, '..');
const workspaceRoot = path.resolve(frontendRoot, '..');
const binariesDir = path.join(frontendRoot, 'src-tauri', 'binaries');
const artifactStoreDir = getArtifactStoreDir(workspaceRoot);
const env = {
  ...process.env,
  CARGO_NET_OFFLINE: 'true',
  MEETILY_FFMPEG_OFFLINE_REQUIRED: 'true',
};

// Release binaries must not retain the workstation username or checkout path
// in panic messages and native __FILE__ strings. Keep stable synthetic paths
// so diagnostics remain useful without leaking the developer machine layout.
const remappedHome = '/build/meetily-home';
const remappedSource = '/build/meetily-source';
const rustPathRemaps = [
  `--remap-path-prefix=${workspaceRoot}=${remappedSource}`,
  `--remap-path-prefix=${os.homedir()}=${remappedHome}`,
];
const nativePathRemaps = [
  `-ffile-prefix-map=${workspaceRoot}=${remappedSource}`,
  `-fdebug-prefix-map=${workspaceRoot}=${remappedSource}`,
  `-ffile-prefix-map=${os.homedir()}=${remappedHome}`,
  `-fdebug-prefix-map=${os.homedir()}=${remappedHome}`,
];
env.RUSTFLAGS = [process.env.RUSTFLAGS, ...rustPathRemaps].filter(Boolean).join(' ');
env.CFLAGS = [process.env.CFLAGS, ...nativePathRemaps].filter(Boolean).join(' ');
env.CXXFLAGS = [process.env.CXXFLAGS, ...nativePathRemaps].filter(Boolean).join(' ');

function run(command, args, cwd = workspaceRoot, captureOutput = false) {
  const result = spawnSync(command, args, {
    cwd,
    env,
    encoding: 'utf8',
    stdio: captureOutput ? 'pipe' : 'inherit',
  });

  if (result.error) {
    throw result.error;
  }

  if (result.status !== 0) {
    if (captureOutput && result.stderr) {
      process.stderr.write(result.stderr);
    }
    throw new Error(`${command} failed with exit code ${result.status ?? 1}`);
  }

  return result.stdout || '';
}

function filesEqual(left, right) {
  if (!existsSync(right) || statSync(left).size !== statSync(right).size) {
    return false;
  }

  return readFileSync(left).equals(readFileSync(right));
}

function helperCargoArgsForTarget(target) {
  if (target !== 'aarch64-apple-darwin' && target !== 'x86_64-apple-darwin') {
    throw new Error(`Unsupported llama-helper target: ${target}`);
  }

  const cargoArgs = ['build', '--offline', '--release', '--package', 'llama-helper',
    '--target', target, '--no-default-features'];
  if (target === 'aarch64-apple-darwin') {
    cargoArgs.push('--features', 'metal');
  }
  return cargoArgs;
}

function main() {
if (process.platform !== 'darwin') {
  console.error('Universal macOS builds must run on macOS.');
  process.exit(1);
}

const fullXcodeDeveloperDir = '/Applications/Xcode.app/Contents/Developer';
if (!env.DEVELOPER_DIR && existsSync(fullXcodeDeveloperDir)) {
  env.DEVELOPER_DIR = fullXcodeDeveloperDir;
}

const installedTargets = new Set(
  run('rustup', ['target', 'list', '--installed'], workspaceRoot, true)
    .trim()
    .split(/\s+/),
);
const missingTargets = targets.filter((target) => !installedTargets.has(target));

if (missingTargets.length > 0) {
  console.error(`Missing Rust targets: ${missingTargets.join(', ')}`);
  console.error(`Install them with: rustup target add ${missingTargets.join(' ')}`);
  process.exit(1);
}

verifyUniversalFfmpegSidecars(binariesDir);
console.log('Universal FFmpeg offline preflight passed.');

env.ORT_LIB_LOCATION = prepareUniversalOnnxRuntime({
  storeDir: artifactStoreDir,
  outputRoot: path.join(workspaceRoot, 'target', 'universal-onnxruntime'),
});
console.log(`Universal ONNX Runtime offline preflight passed: ${env.ORT_LIB_LOCATION}`);

const diarizationResources = provisionDiarizationResources({
  storeDir: artifactStoreDir,
  resourcesDir: path.join(frontendRoot, 'src-tauri', 'resources'),
});
env.SHERPA_ONNX_ARCHIVE_DIR = artifactStoreDir;
console.log(
  `Diarization offline preflight passed: ${diarizationResources.length} model resources`,
);

mkdirSync(binariesDir, { recursive: true });
const helperBinaries = [];

for (const target of targets) {
  console.log(`Building llama-helper for ${target}...`);
  run('cargo', helperCargoArgsForTarget(target));

  const source = path.join(
    workspaceRoot,
    'target',
    target,
    'release',
    'llama-helper',
  );
  const destination = path.join(binariesDir, `llama-helper-${target}`);

  if (!existsSync(source)) {
    console.error(`Built llama-helper was not found at ${source}`);
    process.exit(1);
  }
  verifyThinMachO(source, target);

  if (!filesEqual(source, destination)) {
    copyFileSync(source, destination);
    chmodSync(destination, 0o755);
  }
  verifyThinMachO(destination, target);
  helperBinaries.push({ path: destination, target });
}

const universalHelper = path.join(
  binariesDir,
  'llama-helper-universal-apple-darwin',
);
atomicCreateUniversal({ thinBinaries: helperBinaries, outputPath: universalHelper });

console.log('Building Universal Meetily bundle...');
run('pnpm', [
  'tauri', 'build',
  '--target', 'universal-apple-darwin',
  '--bundles', 'app',
  '--config', 'src-tauri/tauri.universal.conf.json',
], frontendRoot);

const bundleRoot = path.join(
  workspaceRoot,
  'target',
  'universal-apple-darwin',
  'release',
  'bundle',
);
const appPath = path.join(bundleRoot, 'macos', 'meetily.app');
const dmgDir = path.join(bundleRoot, 'dmg');
const dmgPath = path.join(dmgDir, `meetily_${version}_universal.dmg`);

if (!existsSync(appPath)) {
  throw new Error(`Universal app bundle was not found at ${appPath}`);
}

for (const executable of ['meetily', 'llama-helper', 'ffmpeg']) {
  verifyUniversalMachO(path.join(appPath, 'Contents', 'MacOS', executable));
}
console.log('Universal app executable verification passed.');

mkdirSync(dmgDir, { recursive: true });
const stagingDir = mkdtempSync(path.join(os.tmpdir(), 'meetily-dmg-'));

try {
  run('ditto', [appPath, path.join(stagingDir, 'Meetily.app')], frontendRoot);
  symlinkSync('/Applications', path.join(stagingDir, 'Applications'));
  const stagingSizeKb = Number(
    run('du', ['-sk', stagingDir], frontendRoot, true).trim().split(/\s+/)[0],
  );
  const dmgSizeMb = Math.ceil((stagingSizeKb / 1024) * 1.25 + 32);
  run('hdiutil', ['create',
    '-volname', 'Meetily',
    '-srcfolder', stagingDir,
    '-size', `${dmgSizeMb}m`,
    '-ov', '-format', 'UDZO',
    dmgPath,
  ], frontendRoot);
} finally {
  rmSync(stagingDir, { recursive: true, force: true });
}

console.log(`Universal DMG created at ${dmgPath}`);
}

if (require.main === module) {
  main();
}

module.exports = { helperCargoArgsForTarget };
