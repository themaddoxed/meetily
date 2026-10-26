import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';
import { scanPlatformSurface } from '../scripts/platform-surface.mjs';

function fixture() {
  return {
    'Cargo.toml': '[workspace]\nresolver = "2"',
    'frontend/src-tauri/Cargo.toml': '[target.\'cfg(target_os = "macos")\'.dependencies]\ncidre = "0.15"',
    'llama-helper/Cargo.toml': '[features]\ndefault=[]\nmetal=[]',
    'frontend/package.json': JSON.stringify({ scripts: { dev: 'next dev', build: 'next build' } }),
    'frontend/src-tauri/tauri.conf.json': JSON.stringify({ bundle: { targets: ['app', 'dmg'] } }),
    '.github/workflows/ci.yml': 'name: CI\non: push\njobs:\n  check:\n    runs-on: macos-15\n    steps:\n      - run: cargo test --locked\n',
  };
}

test('allows macOS builds, provider endpoints and development Python tools', () => {
  const sources = {
    ...fixture(),
    'frontend/src/provider.ts': 'const url = "https://provider.example/api/windows"; const local = "http://localhost:11434";',
    'scripts/inject_transcript.py': 'print("dev fixture")',
    'frontend/scripts/build.js': 'const targets = ["aarch64-apple-darwin", "x86_64-apple-darwin"];',
  };
  const report = scanPlatformSurface(sources);
  assert.deepEqual(report.findings, []);
  assert.ok(report.excludedPaths.some((item) => item.path === 'scripts/inject_transcript.py'));
});

const violations = [
  ['platform.path', 'frontend/src-tauri/binaries/ffmpeg.exe', ''],
  ['platform.cargo-target', 'frontend/src-tauri/Cargo.toml', '[target.\'cfg(windows)\'.dependencies]\nwindows="0.61"'],
  ['platform.workflow-runner', '.github/workflows/ci.yml', 'jobs:\n  build:\n    runs-on: windows-latest'],
  ['platform.workflow-runner', '.github/workflows/ci.yml', 'jobs:\n  build:\n    strategy:\n      matrix:\n        runner: [macos-15, ubuntu-latest]\n    runs-on: ${{ matrix.runner }}'],
  ['platform.workflow-runner', '.github/workflows/ci.yml', 'jobs:\n  build:\n    runs-on: ${{ inputs.platform }}'],
  ['platform.workflow-condition', '.github/workflows/ci.yml', "jobs:\n  build:\n    runs-on: macos-15\n    if: runner.os == 'Linux'"],
  ['platform.target', 'frontend/scripts/build.js', 'const target = "aarch64-unknown-linux-gnu";'],
  ['platform.condition', 'frontend/src/runtime.ts', "if (process.platform === 'win32') { run(); }"],
  ['platform.condition', 'frontend/src-tauri/src/new.rs', '#[cfg(all(feature = "capture", target_os = "linux"))]\nfn capture() {}'],
  ['platform.bundle', 'frontend/src-tauri/tauri.conf.json', JSON.stringify({ bundle: { targets: ['app', 'deb'] } })],
  ['platform.docs', 'README.md', '## Supported platforms\nWorks on macOS, Windows and Linux.'],
  ['legacy.backend-tree', 'backend/app/main.py', 'import fastapi'],
  ['legacy.launch-surface', 'frontend/src/runtime.ts', "import { spawn } from 'node:child_process'; spawn('python3', ['app/main.py']);"],
  ['legacy.python-runtime', 'frontend/src-tauri/resources/server.py', 'import fastapi'],
  ['platform.cargo-target', '.cargo/config.toml', '[build]\ntarget="wasm32-unknown-unknown"'],
  ['platform.cargo-target', 'frontend/src-tauri/Cargo.toml', '[target.\'cfg(not(target_os = "macos"))\'.dependencies]\nother="1"'],
  ['platform.condition', 'frontend/src-tauri/src/new.rs', '#[cfg(all(any(feature = "capture"), target_os = "linux"))]\nfn capture() {}'],
  ['platform.condition', 'frontend/src/runtime.ts', 'const native = process.platform === "win32" ? 1 : 2;'],
  ['platform.condition', 'frontend/src/runtime.ts', 'if (os.platform() === "win32") { run(); }'],
  ['platform.condition', 'frontend/src/runtime.ts', 'switch (process.platform) { case "win32": run(); }'],
  ['platform.parse', 'frontend/src/runtime.ts', 'if (process.platform === "win32" {'],
  ['platform.parse', 'frontend/package.json', 'null'],
  ['platform.parse', 'Cargo.toml', '[workspace'],
  ['platform.parse', '.github/workflows/ci.yml', 'jobs: [invalid'],
  ['platform.parse', 'frontend/package.json', '{'],
];
for (const [code, filePath, content] of violations) {
  test(`rejects ${code} in ${filePath}: ${content.slice(0, 30)}`, () => {
    const report = scanPlatformSurface({ ...fixture(), [filePath]: content });
    assert.ok(report.findings.some((item) => item.code === code && (item.path === filePath || item.path === 'backend/')), JSON.stringify(report.findings));
  });
}

test('comments and historical evidence are excluded from executable conditions', () => {
  const sources = {
    ...fixture(),
    'frontend/src/runtime.ts': "// if (process.platform === 'win32') {}\nconst text = 'Windows';",
    'frontend/src-tauri/src/new.rs': '// #[cfg(target_os = "linux")]\nfn capture() {}',
    'docs/fork/old-evidence.md': '## Windows\nHistorical build evidence only.',
  };
  assert.deepEqual(scanPlatformSurface(sources).findings, []);
});

test('resolves macOS matrix include runners and fails closed on missing config', () => {
  const sources = fixture();
  sources['.github/workflows/ci.yml'] = 'jobs:\n  build:\n    strategy:\n      matrix:\n        include:\n          - runner: macos-15\n          - runner: macos-15-intel\n    runs-on: ${{ matrix.runner }}';
  assert.deepEqual(scanPlatformSurface(sources).findings, []);
  delete sources['frontend/package.json'];
  assert.ok(scanPlatformSurface(sources).findings.some((item) => item.code === 'platform.missing-config'));
});

const cliPath = fileURLToPath(new URL('../scripts/platform-surface.mjs', import.meta.url));
test('CLI reports actual PASS/FAIL exits for clean and injected tracked/untracked files', () => {
  const root = mkdtempSync(path.join(os.tmpdir(), 'meetily-platform-'));
  try {
    execFileSync('git', ['-c', 'core.hooksPath=/dev/null', 'init', '--quiet', root]);
    for (const [filePath, content] of Object.entries(fixture())) {
      mkdirSync(path.dirname(path.join(root, filePath)), { recursive: true });
      writeFileSync(path.join(root, filePath), content);
    }
    const run = () => spawnSync(process.execPath, [cliPath, '--root', root, '--json'], { encoding: 'utf8' });
    let result = run();
    assert.equal(result.status, 0, result.stderr);
    assert.equal(JSON.parse(result.stdout).status, 'PASS');
    mkdirSync(path.join(root, 'frontend/src-tauri/binaries'), { recursive: true });
    writeFileSync(path.join(root, 'frontend/src-tauri/binaries/installer.exe'), '');
    for (const tracked of [false, true]) {
      if (tracked) execFileSync('git', ['-c', 'core.hooksPath=/dev/null', '-C', root, 'add', '.']);
      result = run();
      assert.equal(result.status, 1, result.stderr);
      const report = JSON.parse(result.stdout);
      assert.equal(report.status, 'FAIL');
      assert.ok(report.findings.some((item) => item.path.endsWith('installer.exe')));
    }
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
test('CLI reports operational errors separately from policy failures', () => {
  const root = mkdtempSync(path.join(os.tmpdir(), 'meetily-platform-error-'));
  try {
    const result = spawnSync(process.execPath, [cliPath, '--root', root, '--json'], { encoding: 'utf8' });
    assert.equal(result.status, 2);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
