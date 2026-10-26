import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import {
  scanTextFile,
  validateProductionConfig,
} from '../scripts/release-policy.mjs';

const policyModule = await import('../scripts/release-policy.mjs');

test('exports the updater-disabled policy interface', async () => {
  assert.equal(typeof policyModule.validateUpdaterDisabled, 'function');
});

const { validateUpdaterDisabled } = policyModule;

test('exports the macOS Rust surface policy interface', async () => {
  assert.equal(typeof policyModule.validateMacosRustSurface, 'function');
});

const { validateMacosRustSurface } = policyModule;

test('exports the macOS audio surface policy interface', async () => {
  assert.equal(typeof policyModule.validateMacosAudioSurface, 'function');
});

test('exports the legacy-runtime absence policy interface', async () => {
  assert.equal(typeof policyModule.validateLegacyRuntimeAbsent, 'function');
});

test('repository policy reports each macOS audio violation exactly once', () => {
  const libPath = 'frontend/src-tauri/src/lib.rs';
  const macosPath = 'frontend/src-tauri/src/audio/capture/macos.rs';
  const libSource = readFileSync(new URL('../src-tauri/src/lib.rs', import.meta.url), 'utf8');
  const macosSource = readFileSync(
    new URL('../src-tauri/src/audio/capture/macos.rs', import.meta.url),
    'utf8',
  );
  const cases = [
    {
      expectedCode: 'audio.lifecycle-global',
      sourceFiles: { [libPath]: `${libSource}\nstatic RECORDING_FLAG: AtomicBool = AtomicBool::new(false);` },
    },
    {
      expectedCode: 'audio.capture-adapter-count',
      sourceFiles: { [macosPath]: macosSource.replace('pub struct MacosCaptureBackend', 'pub struct RemovedCaptureBackend') },
    },
  ];

  for (const { expectedCode, sourceFiles } of cases) {
    const matches = policyModule.validateRepository({ sourceFiles })
      .filter(({ code }) => code === expectedCode);
    assert.equal(matches.length, 1, `expected one ${expectedCode} finding`);
  }
});

function cleanLegacyRuntimeFixture() {
  return {
    trackedFiles: new Set([
      'scripts/inject_transcript.py',
      'frontend/src-tauri/tests/fixtures/legacy_fastapi_schema.sql',
      'frontend/src-tauri/src/database/commands.rs',
      'frontend/src-tauri/src/api/api.rs',
      'frontend/src-tauri/tauri.conf.json',
    ]),
    sourceFiles: {
      'scripts/inject_transcript.py': 'print("developer-only transcript import")',
      'frontend/src-tauri/tests/fixtures/legacy_fastapi_schema.sql': '-- historical FastAPI schema fixture',
      'frontend/src-tauri/src/database/commands.rs': [
        'pub async fn detect_legacy_database(selected_path: String) {',
        'const path = PathBuf::from(selected_path);',
        'let backend_db = path.join("backend").join("meeting_minutes.db");',
        'if backend_db.exists() { return; }',
        '}',
      ].join('\n'),
      'frontend/src-tauri/src/api/api.rs': [
        'const OLLAMA_URL: &str = "http://localhost:11434";',
        'let custom_provider_url = configured_openai_compatible_endpoint;',
      ].join('\n'),
      'frontend/src-tauri/src/database/legacy_fastapi.rs': 'const CONTEXT: &str = "historical FastAPI Python schema";',
      'frontend/src-tauri/src/audio/decoder.rs': 'use std::process::Command; let ffmpeg_path = find_ffmpeg_path().unwrap(); let inspect = |ffmpeg_path: &Path| ffmpeg_path.exists(); inspect(&ffmpeg_path); let command = Command::new(&ffmpeg_path);',
      'frontend/src-tauri/src/audio/incremental_saver.rs': 'use std::process::Command; fn verify(path: &Path) { let ffmpeg = find_ffmpeg_path().unwrap(); let _child = Command::new(&ffmpeg).arg(path).spawn(); }',
      'frontend/src-tauri/src/safe_shell.rs': 'let command = Command::new("sh").args(["-c", "echo ok"]);',
      'frontend/src-tauri/src/local_command.rs': 'struct Command; impl Command { fn new(_: &str) {} } fn run() { Command::new("python3"); }',
      'frontend/src-tauri/src/imported_local_command.rs': 'use crate::safe::Command; fn run() { Command::new("python3"); }',
      'frontend/src-tauri/src/scoped_local_command.rs': 'use std::process::Command; fn run() { struct Command; impl Command { fn new(_: &str) {} } Command::new("python3"); }',
      'frontend/src-tauri/src/scoped_local_use.rs': 'use std::process::Command; fn run() { use crate::safe::Command; Command::new("python3"); }',
      'frontend/src-tauri/src/scoped_local_process.rs': 'use std::process; fn run() { mod process { pub struct Command; impl Command { pub fn new(_: &str) {} } } process::Command::new("python3"); }',
      'frontend/src-tauri/src/scoped_grouped_local_use.rs': 'use std::process::Command; fn run() { use crate::{safe::Command}; Command::new("python3"); }',
      'frontend/src-tauri/src/scoped_grouped_local_process.rs': 'use std::process; fn run() { use crate::{process}; process::Command::new("python3"); }',
      'frontend/src/components/DatabaseImport/HomebrewDatabaseDetector.tsx': 'const HELP = "Install Python 3 with Homebrew";',
      'frontend/src/runtime.ts': [
        '// spawn("python3", ["app/main.py"]);',
        'const runner = { spawn() {} }; runner.spawn("python3", ["app/main.py"]);',
        'function spawn() {} spawn("python3", ["app/main.py"]);',
        'const childProcess = { spawn() {} }; childProcess.spawn("python3", ["app/main.py"]);',
        'import { spawn as importedSpawn } from "node:child_process";',
        'function runLocal(importedSpawn) { importedSpawn("python3", ["app/main.py"]); }',
        'function runBlock() { const importedSpawn = runner.spawn; importedSpawn("python3", ["app/main.py"]); }',
        'try { runner.run(); } catch (importedSpawn) { importedSpawn("python3", ["app/main.py"]); }',
        'import * as importedChildProcess from "node:child_process";',
        'function runNamespace() { const importedChildProcess = runner; importedChildProcess.spawn("python3", ["app/main.py"]); }',
        'function runLocalAlias(importedSpawn) { const launch = importedSpawn; launch("python3", ["app/main.py"]); }',
        'function runLocalNamespace(importedChildProcess) { const launch = importedChildProcess.spawn; launch("python3", ["app/main.py"]); }',
        'function resetToLocal(launch) { launch = require("node:child_process").spawn; launch = runner.spawn; launch("python3", ["app/main.py"]); }',
        'function callBeforeRealAssignment() { let launch = runner.spawn; launch("python3", ["app/main.py"]); launch = require("node:child_process").spawn; }',
        'for (const importedSpawn of runners) { importedSpawn("python3", ["app/main.py"]); }',
        'for (const importedChildProcess of runners) { importedChildProcess.spawn("python3", ["app/main.py"]); }',
        'function resetThroughForOf() { let importedSpawn = require("node:child_process").spawn; for (importedSpawn of runners) { importedSpawn("python3", ["app/main.py"]); } }',
        'function resetThroughForIn() { let importedSpawn = require("node:child_process").spawn; for (importedSpawn in runners) { importedSpawn("python3", ["app/main.py"]); } }',
        'function keepTruthyLocal() { let launch = () => {}; launch ||= require("node:child_process").spawn; launch("python3", ["app/main.py"]); }',
        'import("node:child_process").then(({ spawn }) => { { const spawn = runner.spawn; spawn("python3", ["app/main.py"]); } });',
      ].join('\n'),
    },
    packageJson: { scripts: { dev: 'next dev', build: 'next build' } },
    tauriConfig: {
      app: { security: { csp: "connect-src 'self' http://localhost:11434 https:" } },
      bundle: {
        externalBin: ['binaries/llama-helper'],
        resources: { 'binaries/ffmpeg-*': 'ffmpeg' },
      },
    },
    workflowFiles: {
      '.github/workflows/build.yml': '# FastAPI Python backend historical note\nrun: cargo build --workspace',
    },
  };
}

test('legacy runtime absence policy accepts exact dev, provider, and migration exceptions', () => {
  assert.deepEqual(policyModule.validateLegacyRuntimeAbsent(cleanLegacyRuntimeFixture()), []);
});

test('legacy runtime absence policy reports every prohibited product surface', () => {
  const cases = [
    ['legacy.backend-tree', (fixture) => {
      fixture.trackedFiles.add('backend/app/main.py');
      fixture.sourceFiles['backend/app/main.py'] = 'from fastapi import FastAPI';
    }],
    ['legacy.backend-gitlink', (fixture) => {
      fixture.trackedFiles.add('.gitmodules');
      fixture.sourceFiles['.gitmodules'] = '[submodule "backend/whisper.cpp"]\npath = backend/whisper.cpp';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.packageJson.scripts.startBackend = 'python -m uvicorn backend.app.main:app';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.packageJson.scripts.startApp = 'python app/main.py';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.packageJson.scripts.startApp = 'python -m app.main';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.packageJson.scripts.startApp = './app/main.py';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/main.rs'] = 'Command::new("python").arg("app/main.py");';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/main.rs'] = 'Command::new("/usr/bin/python3").arg("app/main.py");';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/main.rs'] = 'Command::new("env").arg("python3").arg("app/main.py");';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/main.rs'] = 'Command::new("sh").arg("-c").arg("python3 app/main.py");';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/main.rs'] = 'let mut cmd = Command::new("sh"); cmd.arg("-c").arg("python3 app/main.py");';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/main.rs'] = 'let mut cmd = Command::new("sh"); cmd.args(["-c", "python3 app/main.py"]);';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/main.rs'] = 'let mut cmd = Command::new("sh"); cmd.args(["-c", "echo ready; ./app/main.py"]);';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/main.rs'] = 'let mut cmd = Command::new("env"); cmd.arg("./app/main.py");';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/main.rs'] = 'Command::new("sh").arg("-c").arg("python3 app/main.py; echo ok");';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/main.rs'] = 'Command::new("sh").args(["-c", "python3 app/main.py"]);';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/main.rs'] = 'Command::new("env").args(["python3", "app/main.py"]);';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/main.rs'] = 'const PYTHON_CONST: &str = "python3"; Command::new(PYTHON_CONST).arg("app/main.py");';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/main.rs'] = 'Command::new(runtime_executable).arg("app/main.py");';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/main.rs'] = 'Command::new(ffmpeg_or_python).arg("app/main.py");';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/main.rs'] = 'Command::new("app/main.py");';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/main.rs'] = 'use std::process::Command as ProcessCommand; ProcessCommand::new("python3").arg("app/main.py");';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/main.rs'] = 'use std::process::Command; Command :: new("python3").arg("app/main.py");';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/audio/decoder.rs'] = 'use std::process::Command; let mut ffmpeg_path = find_ffmpeg_path().unwrap(); ffmpeg_path = "python3".into(); Command::new(&ffmpeg_path).arg("app/main.py");';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/audio/decoder.rs'] = 'use std::process::Command; let ffmpeg_path = if legacy { "python3".into() } else { find_ffmpeg_path().unwrap() }; Command::new(&ffmpeg_path);';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/audio/decoder.rs'] = 'use std::process::Command; let ffmpeg_path = choose_python_or(find_ffmpeg_path().unwrap()); Command::new(&ffmpeg_path);';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/audio/decoder.rs'] = 'use std::process::Command; let mut ffmpeg_path = find_ffmpeg_path().unwrap(); ffmpeg_path.set_file_name("python3"); Command::new(&ffmpeg_path);';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/audio/decoder.rs'] = 'use std::process::Command; let ffmpeg_path = find_ffmpeg_path().unwrap(); fn run(ffmpeg_path: PathBuf) { Command::new(&ffmpeg_path); }';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/main.rs'] = 'mod helper { struct Command; } use std::process::*; Command::new("python3").arg("app/main.py");';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/main.rs'] = 'use std::{fs, process::Command}; Command::new("python3").arg("app/main.py");';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/main.rs'] = 'use std::process; process::Command::new("python3").arg("app/main.py");';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/main.rs'] = 'use std::process as proc; proc::Command::new("python3").arg("app/main.py");';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/main.rs'] = 'use tokio::process as proc; proc::Command::new("python3").arg("app/main.py");';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/main.rs'] = 'use std::{fs, process}; process::Command::new("python3").arg("app/main.py");';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/main.rs'] = 'use std::{process as proc}; proc::Command::new("python3").arg("app/main.py");';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/main.rs'] = 'use std::process::{self}; process::Command::new("python3").arg("app/main.py");';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/main.rs'] = 'use tokio::{sync, process}; process::Command::new("python3").arg("app/main.py");';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/main.rs'] = 'use std::{fs, process::Command as Cmd}; Cmd::new("python3").arg("app/main.py");';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/main.rs'] = 'use std::process::Command; fn run() { { struct Command; } Command::new("python3").arg("app/main.py"); }';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/main.rs'] = 'use ::std::process::Command; Command::new("python3").arg("app/main.py");';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/main.rs'] = 'use {std::process::Command}; Command::new("python3").arg("app/main.py");';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/audio/decoder.rs'] = 'use std::process::Command; let ffmpeg_path = find_ffmpeg_path().unwrap(); let launch = |ffmpeg_path: PathBuf| Command::new(ffmpeg_path); launch("python3".into());';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/audio/decoder.rs'] = 'use std::process::Command; let mut ffmpeg_path = find_ffmpeg_path().unwrap(); std::mem::replace(&mut ffmpeg_path, "python3".into()); Command::new(&ffmpeg_path);';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/audio/decoder.rs'] = 'use std::process::Command; let mut ffmpeg_path = find_ffmpeg_path().unwrap(); ffmpeg_path.as_mut_os_string().clear(); ffmpeg_path.as_mut_os_string().push("python3"); Command::new(&ffmpeg_path);';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src/runtime.ts'] = 'import { spawn as launch } from "node:child_process"; launch("python3", ["app/main.py"]);';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src/runtime.ts'] = 'import * as childProcess from "node:child_process"; childProcess["spawn"]("python3", ["app/main.py"]);';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src/runtime.ts'] = 'const childProcess = require("node:child_process"); childProcess.spawn("python3", ["app/main.py"]);';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src/runtime.ts'] = 'const launch = require("child_process").spawn; launch("python3", ["app/main.py"]);';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src/runtime.ts'] = 'require("child_process").spawn("python3", ["app/main.py"]);';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src/runtime.ts'] = 'import { execFileSync } from "node:child_process"; execFileSync("python3", ["app/main.py"]);';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src/runtime.ts'] = 'let launch; launch = require("child_process").spawn; launch("python3", ["app/main.py"]);';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src/runtime.ts'] = 'const { spawn } = await import("node:child_process"); spawn("python3", ["app/main.py"]);';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src/runtime.ts'] = 'function run() { const localSpawn = require("node:child_process").spawn; localSpawn("python3", ["app/main.py"]); }';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src/runtime.ts'] = 'function run() { const localProcess = require("node:child_process"); localProcess.spawn("python3", ["app/main.py"]); }';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src/runtime.ts'] = 'const childProcess = await import("node:child_process"); childProcess.spawn("python3", ["app/main.py"]);';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src/runtime.ts'] = '(await import("node:child_process")).spawn("python3", ["app/main.py"]);';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src/runtime.ts'] = 'import("node:child_process").then(({ spawn }) => spawn("python3", ["app/main.py"]));';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src/runtime.ts'] = 'import("node:child_process").then((cp) => cp.spawn("python3", ["app/main.py"]));';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src/runtime.ts'] = 'let launch = require("node:child_process").spawn; launch("python3", ["app/main.py"]); launch = runner.spawn;';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src/runtime.ts'] = 'let launch; if (ready) { launch = require("node:child_process").spawn; } launch("python3", ["app/main.py"]);';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src/runtime.ts'] = 'import { spawn } from "node:child_process"; spawn("python3", ["app/main.py"]); function later() { const spawn = runner.spawn; }';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src/runtime.ts'] = 'import * as cp from "node:child_process"; cp.spawn("python3", ["app/main.py"]); function later() { const cp = runner; }';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src/runtime.ts'] = 'import("node:child_process").then((cp) => { const launch = cp.spawn; launch("python3", ["app/main.py"]); });';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src/runtime.ts'] = 'import("node:child_process").then((cp) => { const { spawn: launch } = cp; launch("python3", ["app/main.py"]); });';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src/runtime.ts'] = 'import("node:child_process").then((cp) => { const launch = cp; launch.spawn("python3", ["app/main.py"]); });';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src/runtime.ts'] = 'let launch; ({ spawn: launch } = require("node:child_process")); launch("python3", ["app/main.py"]);';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src/runtime.ts'] = 'let launch; [launch] = [require("node:child_process").spawn]; launch("python3", ["app/main.py"]);';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src/runtime.ts'] = 'let launch; launch ||= require("node:child_process").spawn; launch("python3", ["app/main.py"]);';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src/runtime.ts'] = 'const cp = require("node:child_process"); const launch = cp.spawn.bind(cp); launch("python3", ["app/main.py"]);';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src/runtime.ts'] = 'const cp = require("node:child_process"); const launch = ready ? cp.spawn : runner.spawn; launch("python3", ["app/main.py"]);';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src/runtime.ts'] = 'let launch; import("node:child_process").then((cp) => (launch = cp.spawn)); launch("python3", ["app/main.py"]);';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src/runtime.ts'] = 'import { spawn as launch } from "node:child_process"; function run(launch) { launch = require("node:child_process").spawn; launch("python3", ["app/main.py"]); }';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src/runtime.ts'] = 'import { spawn as launch } from "node:child_process"; try { work(); } catch (launch) { launch = require("node:child_process").spawn; launch("python3", ["app/main.py"]); }';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src/runtime.ts'] = 'import { spawn as launch } from "node:child_process"; function run(launch) { if (ready) { launch = require("node:child_process").spawn; launch("python3", ["app/main.py"]); } }';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src/runtime.ts'] = 'import { exec } from "node:child_process"; exec("sh -c python3 app/main.py");';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src/runtime.ts'] = 'import { spawn } from "node:child_process"; spawn("env", ["./app/main.py"]);';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src/runtime.ts'] = 'import { spawn } from "node:child_process"; spawn(runtimeExecutable, ["app/main.py"]);';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.sourceFiles['frontend/src/runtime.ts'] = 'import { spawn } from "node:child_process"; spawn("app/main.py");';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.workflowFiles['.github/workflows/build.yml'] = '# harmless comment\nrun: python3 app/main.py';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.workflowFiles['.github/workflows/build.yml'] = 'run: ./app/main.py';
    }],
    ['legacy.launch-surface', (fixture) => {
      fixture.workflowFiles['.github/workflows/build.yml'] = 'jobs:\n  test:\n    steps:\n      - shell: python\n        run: import backend.app.main';
    }],
    ['legacy.local-port', (fixture) => {
      fixture.sourceFiles['frontend/src/runtime.ts'] = 'const legacy = "http://localhost:5167";';
    }],
    ['legacy.http-orchestration', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/api/api.rs'] += '\npub async fn test_backend_connection() { reqwest::get(APP_SERVER_URL).await; }';
    }],
    ['legacy.bundle-runtime', (fixture) => {
      fixture.tauriConfig.bundle.resources['../backend/**'] = 'backend/';
    }],
    ['legacy.bundle-runtime', (fixture) => {
      fixture.tauriConfig.bundle.resources['payload.py'] = 'payload.py';
    }],
    ['legacy.bundle-runtime', (fixture) => {
      fixture.tauriConfig.bundle.resources['payload.pyc'] = 'payload.pyc';
    }],
    ['legacy.bundle-runtime', (fixture) => {
      fixture.tauriConfig.bundle.resources['payload.pyz'] = 'payload.pyz';
    }],
    ['legacy.bundle-runtime', (fixture) => {
      fixture.tauriConfig.bundle.externalBin.push('bin/python3');
    }],
    ['legacy.backend-path', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/api/api.rs'] += '\nlet runtime = root.join("backend").join("server.py");';
    }],
    ['legacy.backend-path', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/api/api.rs'] += '\nlet runtime = PathBuf::from("backend").join("server.py");';
    }],
    ['legacy.backend-path', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/api/api.rs'] += '\nlet runtime = PathBuf::from("backend/server.py");';
    }],
    ['legacy.backend-path', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/api/api.rs'] += '\npath.push("backend");';
    }],
    ['legacy.backend-path', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/database/commands.rs'] = 'let runtime = root.join("backend").join("meeting_minutes.db").join("server.py");';
    }],
    ['legacy.backend-path', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/database/commands.rs'] = 'let runtime = (root.join("backend").join("meeting_minutes.db")).join("server.py");';
    }],
    ['legacy.backend-path', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/database/commands.rs'] = 'let runtime = root.join("backend").join("meeting_minutes.db").with_extension("py");';
    }],
    ['legacy.backend-path', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/database/commands.rs'] = 'let backend_db = path.join("backend").join("meeting_minutes.db"); backend_db.push("server.py");';
    }],
    ['legacy.backend-path', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/database/commands.rs'] = [
        'pub async fn detect_legacy_database(selected_path: String) {',
        'let path = PathBuf::from(selected_path);',
        'let backend_db = path.join("backend").join("meeting_minutes.db");',
        'if backend_db.exists() { return; }',
        '}',
        'fn launch_runtime(path: PathBuf) {',
        'let backend_db = path.join("backend").join("meeting_minutes.db");',
        'run(backend_db);',
        '}',
      ].join('\n');
    }],
    ['legacy.backend-path', (fixture) => {
      fixture.sourceFiles['frontend/src-tauri/src/database/commands.rs'] = cleanLegacyRuntimeFixture()
        .sourceFiles['frontend/src-tauri/src/database/commands.rs']
        .replace('if backend_db.exists() { return; }', 'if backend_db.exists() { run(backend_db); }');
    }],
  ];

  for (const [caseIndex, [expectedCode, mutate]] of cases.entries()) {
    const fixture = cleanLegacyRuntimeFixture();
    mutate(fixture);
    for (const [filePath, content] of Object.entries(fixture.sourceFiles)) {
      if (filePath.endsWith('.rs')
        && /\bCommand\s*::/.test(content)
        && !/\buse\b[^;]*\bCommand\b/s.test(content)
        && !/\buse\s+(?:std|tokio)\s*::\s*process\s*::\s*\*/.test(content)
        && !/\b(?:struct|enum|union|trait|type)\s+Command\b/.test(content)) {
        fixture.sourceFiles[filePath] = `use std::process::Command;\n${content}`;
      }
    }
    const matches = policyModule.validateLegacyRuntimeAbsent(fixture)
      .filter(({ code }) => code === expectedCode);
    assert.equal(matches.length, 1, `case ${caseIndex}: expected one ${expectedCode} finding`);
  }
});

function cleanMacosAudioFixture() {
  return {
    existingPaths: new Set([
      'frontend/src-tauri/src/audio/recording_session.rs',
      'frontend/src-tauri/src/audio/recording_session_state.rs',
      'frontend/src-tauri/src/audio/capture/backend.rs',
      'frontend/src-tauri/src/audio/capture/macos.rs',
      'frontend/src-tauri/src/audio/permissions.rs',
      'frontend/src/hooks/usePermissionCheck.ts',
    ]),
    sourceFiles: {
      'frontend/src-tauri/src/audio/recording_session.rs': [
        'pub struct RecordingSession { action_gate: tokio::sync::Mutex<()> }',
        'impl RecordingSession {',
        'pub async fn start(&self) {} pub async fn pause(&self) {}',
        'pub async fn resume(&self) {} pub async fn stop(&self) {}',
        'pub async fn recover(&self) {} pub async fn get(&self) {}',
        'pub async fn get_active(&self) {}',
        '}',
      ].join('\n'),
      'frontend/src-tauri/src/audio/capture/backend.rs': [
        'const CAPTURE_CONTROL_MAX_MESSAGES: usize = 8;',
        'let (control_tx, control_rx) = mpsc::sync_channel(CAPTURE_CONTROL_MAX_MESSAGES);',
        'let (startup_tx, startup_rx) = mpsc::sync_channel(1);',
        'let (runtime_event_tx, runtime_event_rx) = mpsc::sync_channel(CAPTURE_CONTROL_MAX_MESSAGES);',
        'pub struct CaptureStopped { closed_sources: BTreeSet<TrackKind> }',
      ].join('\n'),
      'frontend/src-tauri/src/audio/capture/macos.rs': [
        'pub const MICROPHONE_RECOVERY_MAX_ATTEMPTS: u8 = 3;',
        'pub const MICROPHONE_RECOVERY_WINDOW: Duration = Duration::from_secs(3);',
        'pub struct MacosCaptureBackend;',
        'fn runtime_event_for_setup_error(error: CoreAudioSetupError) -> CaptureRuntimeEvent {',
        'match error {',
        'CoreAudioSetupError::PermissionDenied(reason) => CaptureRuntimeEvent::SystemAudioDegraded { reason },',
        'CoreAudioSetupError::DeviceUnavailable(reason) => CaptureRuntimeEvent::SystemAudioDegraded { reason },',
        'CoreAudioSetupError::CaptureFailed(reason) => CaptureRuntimeEvent::SystemAudioDegraded { reason },',
        '}',
        '}',
      ].join('\n'),
      'frontend/src-tauri/src/audio/permissions.rs': 'pub struct AudioPermissionCoordinator;',
      'frontend/src/hooks/usePermissionCheck.ts': "invoke('get_audio_permission_snapshot');",
      'frontend/src-tauri/src/lib.rs': '.manage(audio::recording_session::RecordingSession::new())',
      'frontend/src/services/recordingService.ts': "invoke('start_recording'); invoke('stop_recording');",
      'frontend/src/components/RecordingControls.tsx': 'recordingService.stop(sessionId);',
      'frontend/src-tauri/src/audio/incremental_saver.rs': '".tracks" "microphone" "system" "microphone.mp4" "system.mp4" join(".checkpoints") "audio.mp4"',
    },
  };
}

test('accepts one managed owner, one bounded macOS adapter, and service-only invokes', () => {
  assert.deepEqual(policyModule.validateMacosAudioSurface(cleanMacosAudioFixture()), []);
});

for (const [name, mutate, expectedCode] of [
  ['rejects a second production capture adapter', (fixture) => { fixture.sourceFiles['frontend/src-tauri/src/audio/capture/other.rs'] = 'pub struct MacosCaptureBackend;'; }, 'audio.capture-adapter-count'],
  ['rejects a legacy recording global', (fixture) => { fixture.sourceFiles['frontend/src-tauri/src/lib.rs'] += '\nstatic RECORDING_FLAG: AtomicBool = AtomicBool::new(false);'; }, 'audio.lifecycle-global'],
  ['rejects an unsafe capture Send shim', (fixture) => { fixture.sourceFiles['frontend/src-tauri/src/audio/capture/macos.rs'] += '\nunsafe impl Send for MacosCaptureBackend {}'; }, 'audio.unsafe-send-sync'],
  ['rejects direct UI recording invoke', (fixture) => { fixture.sourceFiles['frontend/src/components/RecordingControls.tsx'] = "invoke('pause_recording')"; }, 'audio.direct-ui-invoke'],
  ['rejects unbounded source channels', (fixture) => { fixture.sourceFiles['frontend/src-tauri/src/audio/capture/backend.rs'] += '\nstd::sync::mpsc::channel::<TrackChunk>();'; }, 'audio.unbounded-channel'],
  ['rejects unbounded Tokio source channels', (fixture) => { fixture.sourceFiles['frontend/src-tauri/src/audio/capture/backend.rs'] += '\ntokio::sync::mpsc::unbounded_channel::<TrackChunk>();'; }, 'audio.unbounded-channel'],
  ['rejects permission inference from device enumeration', (fixture) => { fixture.sourceFiles['frontend/src/hooks/usePermissionCheck.ts'] += "\ninvoke('get_audio_devices');"; }, 'audio.permission-device-inference'],
  ['rejects an unbounded runtime-event channel', (fixture) => { fixture.sourceFiles['frontend/src-tauri/src/audio/capture/backend.rs'] = fixture.sourceFiles['frontend/src-tauri/src/audio/capture/backend.rs'].replace('let (runtime_event_tx, runtime_event_rx) = mpsc::sync_channel(CAPTURE_CONTROL_MAX_MESSAGES);', 'let (runtime_event_tx, runtime_event_rx) = mpsc::channel();'); }, 'audio.unbounded-runtime-event'],
  ['rejects a missing microphone recovery bound', (fixture) => { fixture.sourceFiles['frontend/src-tauri/src/audio/capture/macos.rs'] = fixture.sourceFiles['frontend/src-tauri/src/audio/capture/macos.rs'].replace('MICROPHONE_RECOVERY_MAX_ATTEMPTS: u8 = 3', 'MICROPHONE_RECOVERY_MAX_ATTEMPTS: u8 = 4'); }, 'audio.missing-microphone-recovery-bound'],
]) {
  test(name, () => {
    const fixture = cleanMacosAudioFixture();
    mutate(fixture);
    assert.equal(policyModule.validateMacosAudioSurface(fixture)[0]?.code, expectedCode);
  });
}

const audioFindingCodes = (fixture) => new Set(
  policyModule.validateMacosAudioSurface(fixture).map(({ code }) => code),
);

test('macOS audio policy accepts a qualified typed bounded runtime-event channel', () => {
  const fixture = cleanMacosAudioFixture();
  fixture.sourceFiles['frontend/src-tauri/src/audio/capture/backend.rs'] = [
    'const CAPTURE_CONTROL_MAX_MESSAGES: usize = 8;',
    'let (control_tx, control_rx) = std::sync::mpsc::sync_channel(CAPTURE_CONTROL_MAX_MESSAGES);',
    'let (startup_tx, startup_rx) = std::sync::mpsc::sync_channel(1);',
    'let (runtime_event_tx, runtime_event_rx): (std::sync::mpsc::SyncSender<CaptureRuntimeEvent>, std::sync::mpsc::Receiver<CaptureRuntimeEvent>) = std::sync::mpsc::sync_channel::<CaptureRuntimeEvent>(CAPTURE_CONTROL_MAX_MESSAGES);',
    'pub struct CaptureStopped { closed_sources: BTreeSet<TrackKind> }',
  ].join('\n');
  assert.equal(audioFindingCodes(fixture).has('audio.unbounded-runtime-event'), false);
});

test('macOS audio policy accepts mut and ref tuple bindings for a bounded runtime-event channel', () => {
  const fixture = cleanMacosAudioFixture();
  fixture.sourceFiles['frontend/src-tauri/src/audio/capture/backend.rs'] = [
    'const CAPTURE_CONTROL_MAX_MESSAGES: usize = 8;',
    'let (control_tx, control_rx) = std::sync::mpsc::sync_channel(CAPTURE_CONTROL_MAX_MESSAGES);',
    'let (startup_tx, startup_rx) = std::sync::mpsc::sync_channel(1);',
    'let (mut runtime_event_tx, ref runtime_event_rx): (std::sync::mpsc::SyncSender<CaptureRuntimeEvent>, std::sync::mpsc::Receiver<CaptureRuntimeEvent>) = std::sync::mpsc::sync_channel::<CaptureRuntimeEvent>(CAPTURE_CONTROL_MAX_MESSAGES);',
    'pub struct CaptureStopped { closed_sources: BTreeSet<TrackKind> }',
  ].join('\n');
  assert.equal(audioFindingCodes(fixture).has('audio.unbounded-runtime-event'), false);
});

test('macOS audio policy rejects an actual unbounded runtime binding despite decoys', () => {
  const fixture = cleanMacosAudioFixture();
  fixture.sourceFiles['frontend/src-tauri/src/audio/capture/backend.rs'] = [
    'const CAPTURE_CONTROL_MAX_MESSAGES: usize = 8;',
    'let (control_tx, control_rx) = mpsc::sync_channel(CAPTURE_CONTROL_MAX_MESSAGES);',
    'let (startup_tx, startup_rx) = mpsc::sync_channel(1);',
    'let (runtime_event_tx, runtime_event_rx) = std::sync::mpsc::channel::<CaptureRuntimeEvent>();',
    'const DECOY: &str = "let (runtime_event_tx, runtime_event_rx) = mpsc::sync_channel(CAPTURE_CONTROL_MAX_MESSAGES);";',
    '// let (runtime_event_tx, runtime_event_rx) = mpsc::sync_channel(CAPTURE_CONTROL_MAX_MESSAGES);',
    'pub struct CaptureStopped { closed_sources: BTreeSet<TrackKind> }',
  ].join('\n');
  assert.equal(audioFindingCodes(fixture).has('audio.unbounded-runtime-event'), true);
});

test('macOS audio policy rejects any unbounded runtime binding beside a safe one', () => {
  const fixture = cleanMacosAudioFixture();
  fixture.sourceFiles['frontend/src-tauri/src/audio/capture/backend.rs'] += [
    '',
    'if fallback {',
    'let (runtime_event_tx, runtime_event_rx) = std::sync::mpsc::channel::<CaptureRuntimeEvent>();',
    '}',
  ].join('\n');
  assert.equal(audioFindingCodes(fixture).has('audio.unbounded-runtime-event'), true);
});

test('macOS audio policy rejects an unbounded mut tuple binding beside a safe one', () => {
  const fixture = cleanMacosAudioFixture();
  fixture.sourceFiles['frontend/src-tauri/src/audio/capture/backend.rs'] += [
    '',
    'if fallback {',
    'let (mut runtime_event_tx, ref runtime_event_rx) = std::sync::mpsc::channel::<CaptureRuntimeEvent>();',
    '}',
  ].join('\n');
  assert.equal(audioFindingCodes(fixture).has('audio.unbounded-runtime-event'), true);
});

test('macOS audio policy accepts equivalent recovery constant syntax', () => {
  const fixture = cleanMacosAudioFixture();
  fixture.sourceFiles['frontend/src-tauri/src/audio/capture/macos.rs'] = fixture.sourceFiles['frontend/src-tauri/src/audio/capture/macos.rs']
    .replace('pub const MICROPHONE_RECOVERY_MAX_ATTEMPTS: u8 = 3;', 'pub(crate) const MICROPHONE_RECOVERY_MAX_ATTEMPTS: u8 = 3_u8;')
    .replace('pub const MICROPHONE_RECOVERY_WINDOW: Duration = Duration::from_secs(3);', 'pub(super) const MICROPHONE_RECOVERY_WINDOW: std::time::Duration = std::time::Duration::from_secs(3_u64);');
  assert.equal(audioFindingCodes(fixture).has('audio.missing-microphone-recovery-bound'), false);
});

test('macOS audio policy rejects recovery bounds supplied only by comments or strings', () => {
  const fixture = cleanMacosAudioFixture();
  fixture.sourceFiles['frontend/src-tauri/src/audio/capture/macos.rs'] = fixture.sourceFiles['frontend/src-tauri/src/audio/capture/macos.rs']
    .replace('pub const MICROPHONE_RECOVERY_MAX_ATTEMPTS: u8 = 3;', 'const ATTEMPT_DECOY: &str = "pub const MICROPHONE_RECOVERY_MAX_ATTEMPTS: u8 = 3;";')
    .replace('pub const MICROPHONE_RECOVERY_WINDOW: Duration = Duration::from_secs(3);', 'const WINDOW_DECOY: &str = "pub const MICROPHONE_RECOVERY_WINDOW: Duration = Duration::from_secs(3);";');
  assert.equal(audioFindingCodes(fixture).has('audio.missing-microphone-recovery-bound'), true);
});

test('macOS audio policy ignores a permission command that is never invoked', () => {
  const fixture = cleanMacosAudioFixture();
  fixture.sourceFiles['frontend/src/hooks/usePermissionCheck.ts'] = [
    "const migrationNote = 'get_audio_devices';",
    "// invoke('get_audio_devices');",
    "invoke('get_audio_permission_snapshot');",
  ].join('\n');
  assert.equal(audioFindingCodes(fixture).has('audio.permission-device-inference'), false);
});

test('macOS audio policy rejects a statically computed permission-device invoke', () => {
  const fixture = cleanMacosAudioFixture();
  fixture.sourceFiles['frontend/src/hooks/usePermissionCheck.ts'] = [
    "const deviceSuffix = 'devices';",
    'const command = `get_audio_${deviceSuffix}`;',
    'invoke(command);',
  ].join('\n');
  assert.equal(audioFindingCodes(fixture).has('audio.permission-device-inference'), true);
});

test('macOS audio policy resolves a dangerous command in lexical scope at its invoke site', () => {
  const fixture = cleanMacosAudioFixture();
  fixture.sourceFiles['frontend/src/hooks/usePermissionCheck.ts'] = [
    "const command = 'get_audio_devices';",
    'invoke(command);',
    '{',
    "const command = 'get_audio_permission_snapshot';",
    'invoke(command);',
    '}',
  ].join('\n');
  assert.equal(audioFindingCodes(fixture).has('audio.permission-device-inference'), true);
});

test('macOS audio policy keeps a safe invoke separate from a later same-name shadow', () => {
  const fixture = cleanMacosAudioFixture();
  fixture.sourceFiles['frontend/src/hooks/usePermissionCheck.ts'] = [
    "const command = 'get_audio_permission_snapshot';",
    'invoke(command);',
    '{',
    "const command = 'get_audio_devices';",
    'void command;',
    '}',
  ].join('\n');
  assert.equal(audioFindingCodes(fixture).has('audio.permission-device-inference'), false);
});

test('macOS audio policy checks every system setup PermissionDenied arm', () => {
  const fixture = cleanMacosAudioFixture();
  fixture.sourceFiles['frontend/src-tauri/src/audio/capture/macos.rs'] += [
    '',
    'fn second_system_setup(error: CoreAudioSetupError) -> Result<(), CaptureCause> {',
    'match error {',
    'CoreAudioSetupError::PermissionDenied(reason) => Err(CaptureCause::PermissionDenied(reason)),',
    'CoreAudioSetupError::DeviceUnavailable(_) => Ok(()),',
    'CoreAudioSetupError::CaptureFailed(_) => Ok(()),',
    '}',
    '}',
  ].join('\n');
  assert.equal(audioFindingCodes(fixture).has('audio.system-failure-blocks-recording'), true);
});

test('macOS audio policy follows a local denied helper from a system setup arm', () => {
  const fixture = cleanMacosAudioFixture();
  fixture.sourceFiles['frontend/src-tauri/src/audio/capture/macos.rs'] += [
    '',
    'fn whole_capture_denial(reason: String) -> CaptureCause {',
    'CaptureCause::PermissionDenied(reason)',
    '}',
    'fn indirect_system_setup(error: CoreAudioSetupError) -> Result<(), CaptureCause> {',
    'match error {',
    'CoreAudioSetupError::PermissionDenied(reason) => Err(whole_capture_denial(reason)),',
    'CoreAudioSetupError::DeviceUnavailable(_) => Ok(()),',
    'CoreAudioSetupError::CaptureFailed(_) => Ok(()),',
    '}',
    '}',
  ].join('\n');
  assert.equal(audioFindingCodes(fixture).has('audio.system-failure-blocks-recording'), true);
});

test('macOS audio policy accepts a local degradation helper from a system setup arm', () => {
  const fixture = cleanMacosAudioFixture();
  fixture.sourceFiles['frontend/src-tauri/src/audio/capture/macos.rs'] += [
    '',
    'fn system_degradation(reason: String) -> CaptureRuntimeEvent {',
    'CaptureRuntimeEvent::SystemAudioDegraded { reason }',
    '}',
    'fn indirect_system_setup(error: CoreAudioSetupError) -> CaptureRuntimeEvent {',
    'match error {',
    'CoreAudioSetupError::PermissionDenied(reason) => system_degradation(reason),',
    'CoreAudioSetupError::DeviceUnavailable(reason) => system_degradation(reason),',
    'CoreAudioSetupError::CaptureFailed(reason) => system_degradation(reason),',
    '}',
    '}',
  ].join('\n');
  assert.equal(audioFindingCodes(fixture).has('audio.system-failure-blocks-recording'), false);
});

test('macOS audio policy ignores blocking system-error arms in comments and strings', () => {
  const fixture = cleanMacosAudioFixture();
  fixture.sourceFiles['frontend/src-tauri/src/audio/capture/macos.rs'] = [
    'const ARM_DECOY: &str = "CoreAudioSetupError::PermissionDenied(reason) => CaptureCause::PermissionDenied(reason),";',
    '// CoreAudioSetupError::PermissionDenied(reason) => CaptureCause::PermissionDenied(reason),',
    fixture.sourceFiles['frontend/src-tauri/src/audio/capture/macos.rs'],
  ].join('\n');
  assert.equal(audioFindingCodes(fixture).has('audio.system-failure-blocks-recording'), false);
});

test('macOS audio policy rejects an exact whole-capture mutation of the real adapter', () => {
  const fixture = cleanMacosAudioFixture();
  const realMacos = readFileSync(new URL('../src-tauri/src/audio/capture/macos.rs', import.meta.url), 'utf8');
  const safeArm = [
    '            CoreAudioSetupError::PermissionDenied(reason) => {',
    '                format!("permission denied: {reason}")',
    '            }',
  ].join('\n');
  const blockingArm = [
    '            CoreAudioSetupError::PermissionDenied(reason) => {',
    '                return Err(CaptureCause::PermissionDenied(reason));',
    '            }',
  ].join('\n');
  assert.equal(realMacos.split(safeArm).length - 1, 1, 'expected one real setup arm to mutate');
  fixture.sourceFiles['frontend/src-tauri/src/audio/capture/macos.rs'] = realMacos.replace(safeArm, blockingArm);
  assert.equal(audioFindingCodes(fixture).has('audio.system-failure-blocks-recording'), true);
});

function cleanMacosRustSurfaceFixture() {
  return {
    workspaceCargoToml: '[workspace]\nresolver = "2"\nmembers = ["frontend/src-tauri", "llama-helper"]\n',
    appCargoToml: [
      '[package]',
      'name = "meetily"',
      '',
      '[target.\'cfg(target_os = "macos")\'.dependencies]',
      'whisper-rs = { version = "0.13.2", features = ["raw-api"] }',
      '',
      '[target.\'cfg(all(target_os = "macos", target_arch = "aarch64"))\'.dependencies]',
      'whisper-rs = { version = "0.13.2", features = ["metal", "coreml"] }',
      '',
      '[build-dependencies]',
      'tauri-build = "2"',
      'zip = "2"',
      '',
    ].join('\n'),
    llamaCargoToml: [
      '[package]',
      'name = "llama-helper"',
      '',
      '[features]',
      'default = []',
      'metal = ["llama-cpp-2/metal"]',
      '',
    ].join('\n'),
    packageJson: {
      scripts: {
        'tauri:dev': 'tauri dev',
        'tauri:build': 'tauri build',
        'tauri:build:universal': 'node scripts/build-universal-macos.js',
      },
    },
    sourceFiles: {
      'frontend/scripts/build-universal-macos.js': [
        "const targets = ['aarch64-apple-darwin', 'x86_64-apple-darwin'];",
        'function helperCargoArgsForTarget(target) {',
        "  const cargoArgs = ['build', '--offline', '--release', '--package', 'llama-helper', '--target', target, '--no-default-features'];",
        "  if (target === 'aarch64-apple-darwin') cargoArgs.push('--features', 'metal');",
        '  return cargoArgs;',
        '}',
        'for (const target of targets) {',
        "  run('cargo', helperCargoArgsForTarget(target));",
        '}',
      ].join('\n'),
      'frontend/src-tauri/build.rs': 'println!("cargo:rustc-link-lib=framework=AppKit");',
      'frontend/src-tauri/build/ffmpeg.rs': 'const TARGETS: [&str; 2] = ["aarch64-apple-darwin", "x86_64-apple-darwin"]; // ZIP only',
      'frontend/src-tauri/src/audio/ffmpeg.rs': 'const EXECUTABLE: &str = "ffmpeg";',
      'frontend/src-tauri/src/summary/summary_engine/sidecar.rs': 'const TARGET: &str = "aarch64-apple-darwin";',
      'frontend/src-tauri/src/console_utils/console_utils.rs': 'Command::new("osascript");',
      'frontend/src-tauri/src/api/api.rs': 'Command::new("open");',
      'frontend/src-tauri/src/database/commands.rs': 'Command::new("open");',
      'frontend/src-tauri/src/whisper_engine/commands.rs': 'Command::new("open");',
      'frontend/src-tauri/src/parakeet_engine/commands.rs': 'Command::new("open");',
      'frontend/src-tauri/src/lib.rs': '.plugin(tauri_plugin_single_instance::init(|_, _, _| {}))',
      'frontend/src-tauri/src/main.rs': 'fn main() { meetily_lib::run() }',
    },
  };
}

test('accepts the architecture-selected macOS Rust/build contract', () => {
  assert.deepEqual(validateMacosRustSurface(cleanMacosRustSurfaceFixture()), []);
});

const macosRustSurfaceCases = [
  {
    name: 'requires Cargo resolver 2 or 3',
    mutate: (input) => { input.workspaceCargoToml = '[workspace]\nresolver = "1"\n'; },
    expected: { code: 'rust.workspace-resolver', path: 'Cargo.toml' },
  },
  {
    name: 'rejects a direct Windows dependency target table',
    mutate: (input) => { input.appCargoToml += '\n[target.\'cfg(windows)\'.dependencies]\nwindows = "0.61"\n'; },
    expected: { code: 'rust.non-macos-target', path: 'frontend/src-tauri/Cargo.toml' },
  },
  {
    name: 'rejects a direct Linux dependency target table',
    mutate: (input) => { input.appCargoToml += '\n[target.\'cfg(target_os = "linux")\'.dependencies]\nlibc = "0.2"\n'; },
    expected: { code: 'rust.non-macos-target', path: 'frontend/src-tauri/Cargo.toml' },
  },
  {
    name: 'rejects an unreviewed non-macOS target table',
    mutate: (input) => { input.appCargoToml += '\n[target.\'cfg(target_os = "freebsd")\'.dependencies]\nlibc = "0.2"\n'; },
    expected: { code: 'rust.non-macos-target', path: 'frontend/src-tauri/Cargo.toml' },
  },
  {
    name: 'rejects an unreviewed target build-dependency table',
    mutate: (input) => { input.appCargoToml += '\n[target.\'cfg(target_os = "freebsd")\'.build-dependencies]\ncc = "1"\n'; },
    expected: { code: 'rust.non-macos-target', path: 'frontend/src-tauri/Cargo.toml' },
  },
  {
    name: 'rejects an unreviewed target dev-dependency table',
    mutate: (input) => { input.appCargoToml += '\n[target.\'cfg(target_os = "freebsd")\'.dev-dependencies]\ntempfile = "3"\n'; },
    expected: { code: 'rust.non-macos-target', path: 'frontend/src-tauri/Cargo.toml' },
  },
  ...['cuda', 'vulkan', 'hipblas', 'openblas', 'openmp', 'metal', 'coreml', 'platform-default']
    .map((feature) => ({
      name: `rejects the manual app feature ${feature}`,
      mutate: (input) => { input.appCargoToml += `\n[features]\n${feature} = []\n`; },
      expected: { code: 'rust.manual-acceleration-feature', path: 'frontend/src-tauri/Cargo.toml' },
    })),
  {
    name: 'rejects Metal on the common macOS Whisper dependency',
    mutate: (input) => { input.appCargoToml = input.appCargoToml.replace('["raw-api"]', '["raw-api", "metal"]'); },
    expected: { code: 'rust.intel-whisper-acceleration', path: 'frontend/src-tauri/Cargo.toml' },
  },
  {
    name: 'rejects CoreML on the common macOS Whisper dependency',
    mutate: (input) => { input.appCargoToml = input.appCargoToml.replace('["raw-api"]', '["raw-api", "coreml"]'); },
    expected: { code: 'rust.intel-whisper-acceleration', path: 'frontend/src-tauri/Cargo.toml' },
  },
  {
    name: 'requires the pinned common Whisper source version',
    mutate: (input) => { input.appCargoToml = input.appCargoToml.replace('version = "0.13.2", features = ["raw-api"]', 'version = "0.13.1", features = ["raw-api"]'); },
    expected: { code: 'rust.whisper-source', path: 'frontend/src-tauri/Cargo.toml' },
  },
  {
    name: 'requires the pinned arm64 Whisper source version',
    mutate: (input) => { input.appCargoToml = input.appCargoToml.replace('version = "0.13.2", features = ["metal", "coreml"]', 'version = "0.13.1", features = ["metal", "coreml"]'); },
    expected: { code: 'rust.whisper-source', path: 'frontend/src-tauri/Cargo.toml' },
  },
  {
    name: 'requires raw-api on the common macOS Whisper dependency',
    mutate: (input) => { input.appCargoToml = input.appCargoToml.replace('features = ["raw-api"]', 'features = []'); },
    expected: { code: 'rust.arm-whisper-acceleration', path: 'frontend/src-tauri/Cargo.toml' },
  },
  {
    name: 'requires Metal and CoreML on the arm64 Whisper dependency',
    mutate: (input) => { input.appCargoToml = input.appCargoToml.replace('["metal", "coreml"]', '["metal"]'); },
    expected: { code: 'rust.arm-whisper-acceleration', path: 'frontend/src-tauri/Cargo.toml' },
  },
  ...['tar', 'xz2', 'which'].map((dependency) => ({
    name: `rejects the non-macOS build dependency ${dependency}`,
    mutate: (input) => { input.appCargoToml += `${dependency} = "1"\n`; },
    expected: { code: 'rust.non-macos-build-dependency', path: 'frontend/src-tauri/Cargo.toml' },
  })),
  ...['cuda', 'vulkan'].map((feature) => ({
    name: `rejects the helper feature ${feature}`,
    mutate: (input) => { input.llamaCargoToml += `${feature} = []\n`; },
    expected: { code: 'rust.helper-feature', path: 'llama-helper/Cargo.toml' },
  })),
  {
    name: 'rejects a non-empty helper default feature list',
    mutate: (input) => { input.llamaCargoToml = input.llamaCargoToml.replace('default = []', 'default = ["metal"]'); },
    expected: { code: 'rust.helper-feature', path: 'llama-helper/Cargo.toml' },
  },
  {
    name: 'rejects a tracked obsolete GPU wrapper',
    mutate: (input) => { input.sourceFiles['frontend/scripts/tauri-auto.js'] = 'process.argv'; },
    expected: { code: 'rust.manual-gpu-script', path: 'frontend/scripts/tauri-auto.js' },
  },
  {
    name: 'rejects a package script that selects Cargo features',
    mutate: (input) => { input.packageJson.scripts['tauri:build:metal'] = 'tauri build --features metal'; },
    expected: { code: 'rust.manual-gpu-script', path: 'frontend/package.json' },
  },
  {
    name: 'rejects Metal routed to the Intel helper',
    mutate: (input) => {
      input.sourceFiles['frontend/scripts/build-universal-macos.js'] = input.sourceFiles['frontend/scripts/build-universal-macos.js']
        .replace(
          "target === 'aarch64-apple-darwin'",
          "target === 'aarch64-apple-darwin' || target === 'x86_64-apple-darwin'",
        );
    },
    expected: { code: 'rust.universal-helper-routing', path: 'frontend/scripts/build-universal-macos.js' },
  },
  {
    name: 'rejects a Universal helper build that omits arm64 Metal',
    mutate: (input) => { input.sourceFiles['frontend/scripts/build-universal-macos.js'] = "const targets = ['aarch64-apple-darwin', 'x86_64-apple-darwin']; const helperArgs = ['build', '--no-default-features'];"; },
    expected: { code: 'rust.universal-helper-routing', path: 'frontend/scripts/build-universal-macos.js' },
  },
  {
    name: 'rejects a Universal helper build satisfied only by a comment decoy',
    mutate: (input) => {
      input.sourceFiles['frontend/scripts/build-universal-macos.js'] = [
        'function helperCargoArgsForTarget(target) {',
        "  return ['build', '--offline', '--release', '--package', 'llama-helper', '--target', target, '--no-default-features'];",
        '}',
        '// aarch64-apple-darwin --features metal',
        "const cargoArgs = helperCargoArgsForTarget('aarch64-apple-darwin');",
        "run('cargo', cargoArgs);",
      ].join('\n');
    },
    expected: { code: 'rust.universal-helper-routing', path: 'frontend/scripts/build-universal-macos.js' },
  },
  {
    name: 'rejects a Universal helper build satisfied only by a string decoy',
    mutate: (input) => {
      input.sourceFiles['frontend/scripts/build-universal-macos.js'] = [
        'function helperCargoArgsForTarget(target) {',
        "  const decoy = 'aarch64-apple-darwin --features metal';",
        "  return ['build', '--offline', '--release', '--package', 'llama-helper', '--target', target, '--no-default-features'];",
        '}',
        "const cargoArgs = helperCargoArgsForTarget('aarch64-apple-darwin');",
        "run('cargo', cargoArgs);",
      ].join('\n');
    },
    expected: { code: 'rust.universal-helper-routing', path: 'frontend/scripts/build-universal-macos.js' },
  },
  {
    name: 'rejects swapped Intel and arm64 helper acceleration branches',
    mutate: (input) => {
      input.sourceFiles['frontend/scripts/build-universal-macos.js'] = input.sourceFiles['frontend/scripts/build-universal-macos.js']
        .replace("target === 'aarch64-apple-darwin'", "target === 'x86_64-apple-darwin'");
    },
    expected: { code: 'rust.universal-helper-routing', path: 'frontend/scripts/build-universal-macos.js' },
  },
  {
    name: 'rejects a helper whose result is built for a constant target',
    mutate: (input) => {
      input.sourceFiles['frontend/scripts/build-universal-macos.js'] = input.sourceFiles['frontend/scripts/build-universal-macos.js']
        .replace(
          "run('cargo', helperCargoArgsForTarget(target));",
          "run('cargo', helperCargoArgsForTarget('x86_64-apple-darwin'));",
        );
    },
    expected: { code: 'rust.universal-helper-routing', path: 'frontend/scripts/build-universal-macos.js' },
  },
  {
    name: 'rejects a dead helper invocation decoy beside a CPU-only build loop',
    mutate: (input) => {
      input.sourceFiles['frontend/scripts/build-universal-macos.js'] = input.sourceFiles['frontend/scripts/build-universal-macos.js']
        .replace(
          "  run('cargo', helperCargoArgsForTarget(target));",
          "  run('cargo', ['build', '--offline', '--release', '--package', 'llama-helper', '--target', target, '--no-default-features']);",
        )
        .concat([
          '',
          'function unusedHelperBuild(target) {',
          '  const cargoArgs = helperCargoArgsForTarget(target);',
          "  run('cargo', cargoArgs);",
          '}',
        ].join('\n'));
    },
    expected: { code: 'rust.universal-helper-routing', path: 'frontend/scripts/build-universal-macos.js' },
  },
  ...[
    ['frontend/src-tauri/build.rs', 'let target = "x86_64-pc-windows-msvc";'],
    ['frontend/src-tauri/build/ffmpeg.rs', 'extract_tar_xz(archive);'],
    ['frontend/src-tauri/src/audio/ffmpeg.rs', 'let executable = "ffmpeg.exe";'],
    ['frontend/src-tauri/src/summary/summary_engine/sidecar.rs', 'let target = "x86_64-unknown-linux-gnu";'],
    ['frontend/src-tauri/build.rs', 'if which::which("nvidia-smi").is_ok() {}'],
  ].map(([filePath, source]) => ({
    name: `rejects non-macOS build source in ${filePath}`,
    mutate: (input) => { input.sourceFiles[filePath] = source; },
    expected: { code: 'rust.non-macos-build-source', path: filePath },
  })),
];

for (const { name, mutate, expected } of macosRustSurfaceCases) {
  test(name, () => {
    const input = cleanMacosRustSurfaceFixture();
    mutate(input);
    assert.deepEqual(
      validateMacosRustSurface(input).map(({ code, path }) => ({ code, path })),
      [expected],
    );
  });
}

test('ignores forbidden words in semantic Cargo comments and package metadata strings', () => {
  const input = cleanMacosRustSurfaceFixture();
  input.workspaceCargoToml += '# resolver = "1" for old examples\n';
  input.appCargoToml += '\n[package.metadata.notes]\ntext = "[features] cuda = [] and windows are historical examples"\n';
  input.llamaCargoToml += '# cuda = ["llama-cpp-2/cuda"]\n';

  assert.deepEqual(validateMacosRustSurface(input), []);
});

function cleanUpdaterPolicyFixture() {
  return {
    packageJson: { dependencies: {} },
    cargoToml: '[package]\nname = "meetily"\n\n[dependencies]\ntauri = "2"\n',
    cargoLock: 'version = 4\n\n[[package]]\nname = "tauri"\nversion = "2.0.0"\n',
    sourceFiles: {},
    tauriConfig: {
      app: { security: { capabilities: [{ permissions: ['core:default'] }] } },
      bundle: {},
    },
    universalConfig: { bundle: { resources: {} } },
    capabilityFiles: {},
    trackedFiles: new Set(),
    workflowFiles: {},
  };
}

const updaterSurfaceCases = [
  {
    name: 'rejects the frontend updater dependency',
    mutate: (input) => { input.packageJson.dependencies['@tauri-apps/plugin-updater'] = '2.0.0'; },
    expected: { code: 'updater.frontend-dependency', path: 'frontend/package.json' },
  },
  {
    name: 'rejects the frontend process dependency',
    mutate: (input) => { input.packageJson.dependencies['@tauri-apps/plugin-process'] = '2.0.0'; },
    expected: { code: 'process.frontend-dependency', path: 'frontend/package.json' },
  },
  ...[
    ['optionalDependencies', '@tauri-apps/plugin-updater', 'updater.frontend-dependency'],
    ['optionalDependencies', '@tauri-apps/plugin-process', 'process.frontend-dependency'],
    ['devDependencies', '@tauri-apps/plugin-updater', 'updater.frontend-dependency'],
    ['devDependencies', '@tauri-apps/plugin-process', 'process.frontend-dependency'],
    ['peerDependencies', '@tauri-apps/plugin-updater', 'updater.frontend-dependency'],
    ['peerDependencies', '@tauri-apps/plugin-process', 'process.frontend-dependency'],
  ].map(([field, dependency, code]) => ({
    name: `rejects ${dependency} in ${field}`,
    mutate: (input) => { input.packageJson[field] = { [dependency]: '2.0.0' }; },
    expected: { code, path: 'frontend/package.json' },
  })),
  {
    name: 'rejects an npm updater alias in production dependencies',
    mutate: (input) => { input.packageJson.dependencies.legacy = 'npm:@tauri-apps/plugin-updater@2.10.1'; },
    expected: { code: 'updater.frontend-dependency', path: 'frontend/package.json' },
  },
  {
    name: 'rejects an npm process alias in optional dependencies',
    mutate: (input) => { input.packageJson.optionalDependencies = { restart: 'npm:@tauri-apps/plugin-process@^2' }; },
    expected: { code: 'process.frontend-dependency', path: 'frontend/package.json' },
  },
  ...[
    ['bundledDependencies', '@tauri-apps/plugin-updater', 'updater.frontend-dependency'],
    ['bundledDependencies', '@tauri-apps/plugin-process', 'process.frontend-dependency'],
    ['bundleDependencies', '@tauri-apps/plugin-updater', 'updater.frontend-dependency'],
    ['bundleDependencies', '@tauri-apps/plugin-process', 'process.frontend-dependency'],
  ].map(([field, dependency, code]) => ({
    name: `rejects ${dependency} in ${field}`,
    mutate: (input) => { input.packageJson[field] = [dependency]; },
    expected: { code, path: 'frontend/package.json' },
  })),
  {
    name: 'rejects the root Cargo updater dependency',
    mutate: (input) => { input.cargoToml += 'tauri-plugin-updater = "2"\n'; },
    expected: { code: 'updater.cargo-dependency', path: 'frontend/src-tauri/Cargo.toml' },
  },
  {
    name: 'rejects the root Cargo process dependency',
    mutate: (input) => { input.cargoToml += 'tauri-plugin-process = "2"\n'; },
    expected: { code: 'process.cargo-dependency', path: 'frontend/src-tauri/Cargo.toml' },
  },
  {
    name: 'rejects a double-quoted root Cargo updater dependency key',
    mutate: (input) => { input.cargoToml += '"tauri-plugin-updater" = "2"\n'; },
    expected: { code: 'updater.cargo-dependency', path: 'frontend/src-tauri/Cargo.toml' },
  },
  {
    name: 'rejects a single-quoted root Cargo process dependency key',
    mutate: (input) => { input.cargoToml += "'tauri-plugin-process' = \"2\"\n"; },
    expected: { code: 'process.cargo-dependency', path: 'frontend/src-tauri/Cargo.toml' },
  },
  {
    name: 'rejects an unquoted target-specific Cargo updater dependency',
    mutate: (input) => { input.cargoToml += '\n[target.aarch64-apple-darwin.dependencies]\ntauri-plugin-updater = "2"\n'; },
    expected: { code: 'updater.cargo-dependency', path: 'frontend/src-tauri/Cargo.toml' },
  },
  {
    name: 'rejects an unquoted target-specific Cargo process dependency',
    mutate: (input) => { input.cargoToml += '\n[target.aarch64-apple-darwin.dependencies]\ntauri-plugin-process = "2"\n'; },
    expected: { code: 'process.cargo-dependency', path: 'frontend/src-tauri/Cargo.toml' },
  },
  {
    name: 'rejects a double-quoted target-specific Cargo updater dependency key',
    mutate: (input) => { input.cargoToml += '\n[target.aarch64-apple-darwin.dependencies]\n"tauri-plugin-updater" = "2"\n'; },
    expected: { code: 'updater.cargo-dependency', path: 'frontend/src-tauri/Cargo.toml' },
  },
  {
    name: 'rejects a single-quoted target-specific Cargo process dependency key',
    mutate: (input) => { input.cargoToml += "\n[target.aarch64-apple-darwin.dependencies]\n'tauri-plugin-process' = \"2\"\n"; },
    expected: { code: 'process.cargo-dependency', path: 'frontend/src-tauri/Cargo.toml' },
  },
  {
    name: 'rejects a quoted cfg target-specific Cargo updater dependency',
    mutate: (input) => { input.cargoToml += '\n[target.\'cfg(target_os = "macos")\'.dependencies]\ntauri-plugin-updater = "2"\n'; },
    expected: { code: 'updater.cargo-dependency', path: 'frontend/src-tauri/Cargo.toml' },
  },
  {
    name: 'rejects a quoted cfg target-specific Cargo process dependency',
    mutate: (input) => { input.cargoToml += '\n[target.\'cfg(target_os = "macos")\'.dependencies]\ntauri-plugin-process = "2"\n'; },
    expected: { code: 'process.cargo-dependency', path: 'frontend/src-tauri/Cargo.toml' },
  },
  {
    name: 'rejects a single-quoted Cargo updater key in a quoted cfg target table',
    mutate: (input) => { input.cargoToml += "\n[target.'cfg(target_os = \"macos\")'.dependencies]\n'tauri-plugin-updater' = \"2\"\n"; },
    expected: { code: 'updater.cargo-dependency', path: 'frontend/src-tauri/Cargo.toml' },
  },
  {
    name: 'rejects a double-quoted Cargo process key in a quoted cfg target table',
    mutate: (input) => { input.cargoToml += '\n[target.\'cfg(target_os = "macos")\'.dependencies]\n"tauri-plugin-process" = "2"\n'; },
    expected: { code: 'process.cargo-dependency', path: 'frontend/src-tauri/Cargo.toml' },
  },
  {
    name: 'rejects a dependency in a whitespace-separated Cargo target table',
    mutate: (input) => { input.cargoToml += '\n[target . aarch64-apple-darwin . dependencies]\ntauri-plugin-updater = "2"\n'; },
    expected: { code: 'updater.cargo-dependency', path: 'frontend/src-tauri/Cargo.toml' },
  },
  {
    name: 'rejects a dependency in a quoted root Cargo dependency table',
    mutate: (input) => { input.cargoToml = input.cargoToml.replace('[dependencies]', '["dependencies"]') + 'tauri-plugin-process = "2"\n'; },
    expected: { code: 'process.cargo-dependency', path: 'frontend/src-tauri/Cargo.toml' },
  },
  {
    name: 'rejects a root Cargo dependency subtable',
    mutate: (input) => { input.cargoToml += '\n[dependencies."tauri-plugin-updater"]\nversion = "2"\n'; },
    expected: { code: 'updater.cargo-dependency', path: 'frontend/src-tauri/Cargo.toml' },
  },
  {
    name: 'rejects a target Cargo dependency subtable',
    mutate: (input) => { input.cargoToml += '\n[target.\'cfg(target_os = "macos")\'.dependencies.\'tauri-plugin-process\']\nversion = "2"\n'; },
    expected: { code: 'process.cargo-dependency', path: 'frontend/src-tauri/Cargo.toml' },
  },
  {
    name: 'rejects an aliased Cargo updater dependency',
    mutate: (input) => { input.cargoToml += 'updater = { package = "tauri-plugin-updater", version = "2" }\n'; },
    expected: { code: 'updater.cargo-dependency', path: 'frontend/src-tauri/Cargo.toml' },
  },
  {
    name: 'rejects an aliased Cargo process dependency subtable',
    mutate: (input) => { input.cargoToml += '\n[dependencies.restart]\npackage = "tauri-plugin-process"\nversion = "2"\n'; },
    expected: { code: 'process.cargo-dependency', path: 'frontend/src-tauri/Cargo.toml' },
  },
  {
    name: 'rejects a root dotted Cargo updater dependency assignment',
    mutate: (input) => { input.cargoToml = 'dependencies.tauri-plugin-updater.version = "2"\n\n[package]\nname = "meetily"\n'; },
    expected: { code: 'updater.cargo-dependency', path: 'frontend/src-tauri/Cargo.toml' },
  },
  {
    name: 'rejects a target dotted Cargo process dependency assignment',
    mutate: (input) => { input.cargoToml = 'target.aarch64-apple-darwin.dependencies.tauri-plugin-process.version = "2"\n' + input.cargoToml; },
    expected: { code: 'process.cargo-dependency', path: 'frontend/src-tauri/Cargo.toml' },
  },
  {
    name: 'rejects an updater dependency in a root inline Cargo map',
    mutate: (input) => { input.cargoToml = 'dependencies = { tauri-plugin-updater = "2" }\n\n[package]\nname = "meetily"\n'; },
    expected: { code: 'updater.cargo-dependency', path: 'frontend/src-tauri/Cargo.toml' },
  },
  {
    name: 'rejects a process dependency in a target inline Cargo map',
    mutate: (input) => { input.cargoToml = 'target.aarch64-apple-darwin.dependencies = { tauri-plugin-process = "2" }\n' + input.cargoToml; },
    expected: { code: 'process.cargo-dependency', path: 'frontend/src-tauri/Cargo.toml' },
  },
  {
    name: 'rejects a unicode-escaped basic quoted Cargo dependency key',
    mutate: (input) => { input.cargoToml += '"tauri-plugin-\\u0075pdater" = "2"\n'; },
    expected: { code: 'updater.cargo-dependency', path: 'frontend/src-tauri/Cargo.toml' },
  },
  {
    name: 'rejects a unicode-escaped Cargo package alias in a dependency subtable',
    mutate: (input) => { input.cargoToml += '\n[dependencies.restart]\npackage = "tauri-plugin-\\u0070rocess"\nversion = "2"\n'; },
    expected: { code: 'process.cargo-dependency', path: 'frontend/src-tauri/Cargo.toml' },
  },
  {
    name: 'rejects a unicode-escaped Cargo package alias in an inline table',
    mutate: (input) => { input.cargoToml = 'dependencies = { restart = { package = "tauri-plugin-\\u0070rocess", version = "2" } }\n\n[package]\nname = "meetily"\n'; },
    expected: { code: 'process.cargo-dependency', path: 'frontend/src-tauri/Cargo.toml' },
  },
  {
    name: 'rejects the updater Cargo lock package',
    mutate: (input) => { input.cargoLock += '\n[[package]]\nname = "tauri-plugin-updater"\nversion = "2.0.0"\n'; },
    expected: { code: 'updater.cargo-lock', path: 'Cargo.lock' },
  },
  {
    name: 'rejects the process Cargo lock package',
    mutate: (input) => { input.cargoLock += '\n[[package]]\nname = "tauri-plugin-process"\nversion = "2.0.0"\n'; },
    expected: { code: 'process.cargo-lock', path: 'Cargo.lock' },
  },
  {
    name: 'rejects Rust updater plugin registration',
    mutate: (input) => { input.sourceFiles['frontend/src-tauri/src/lib.rs'] = '.plugin(tauri_plugin_updater::Builder::new().build())'; },
    expected: { code: 'updater.rust-registration', path: 'frontend/src-tauri/src/lib.rs' },
  },
  {
    name: 'rejects Rust process plugin registration',
    mutate: (input) => { input.sourceFiles['frontend/src-tauri/src/lib.rs'] = '.plugin(tauri_plugin_process::init())'; },
    expected: { code: 'process.rust-registration', path: 'frontend/src-tauri/src/lib.rs' },
  },
  {
    name: 'rejects a production updater package import',
    mutate: (input) => { input.sourceFiles['frontend/src/runtime.ts'] = "import {\n  check,\n} from '@tauri-apps/plugin-updater';"; },
    expected: { code: 'updater.frontend-import', path: 'frontend/src/runtime.ts' },
  },
  {
    name: 'rejects a production process package import',
    mutate: (input) => { input.sourceFiles['frontend/src/runtime.ts'] = "import { relaunch } from '@tauri-apps/plugin-process';"; },
    expected: { code: 'process.frontend-import', path: 'frontend/src/runtime.ts' },
  },
  {
    name: 'rejects a no-space production updater import',
    mutate: (input) => { input.sourceFiles['frontend/src/runtime.js'] = 'import{check}from"@tauri-apps/plugin-updater"'; },
    expected: { code: 'updater.frontend-import', path: 'frontend/src/runtime.js' },
  },
  {
    name: 'rejects a dynamic production process import',
    mutate: (input) => { input.sourceFiles['frontend/src/runtime.mjs'] = 'await import("@tauri-apps/plugin-process")'; },
    expected: { code: 'process.frontend-import', path: 'frontend/src/runtime.mjs' },
  },
  {
    name: 'rejects a production import resolving to updateService',
    mutate: (input) => { input.sourceFiles['frontend/src/components/Caller.tsx'] = "import updateService from '@/services/updateService';"; },
    expected: { code: 'updater.local-import', path: 'frontend/src/components/Caller.tsx' },
  },
  {
    name: 'rejects a production import resolving to useUpdateCheck',
    mutate: (input) => { input.sourceFiles['frontend/src/components/Caller.tsx'] = "import { useUpdateCheck } from '../hooks/useUpdateCheck';"; },
    expected: { code: 'updater.local-import', path: 'frontend/src/components/Caller.tsx' },
  },
  {
    name: 'rejects a production import resolving to UpdateCheckProvider',
    mutate: (input) => { input.sourceFiles['frontend/src/app/layout.tsx'] = "import { UpdateCheckProvider } from '@/components/UpdateCheckProvider';"; },
    expected: { code: 'updater.local-import', path: 'frontend/src/app/layout.tsx' },
  },
  {
    name: 'rejects a production import resolving to UpdateDialog',
    mutate: (input) => { input.sourceFiles['frontend/src/components/About.tsx'] = "import UpdateDialog from './UpdateDialog';"; },
    expected: { code: 'updater.local-import', path: 'frontend/src/components/About.tsx' },
  },
  {
    name: 'rejects a production import resolving to UpdateNotification',
    mutate: (input) => { input.sourceFiles['frontend/src/hooks/useBanner.ts'] = "import UpdateNotification from '../components/UpdateNotification';"; },
    expected: { code: 'updater.local-import', path: 'frontend/src/hooks/useBanner.ts' },
  },
  {
    name: 'rejects a direct production updater command prefix',
    mutate: (input) => { input.sourceFiles['frontend/src/runtime.ts'] = "invoke('plugin:updater|check')"; },
    expected: { code: 'updater.direct-command', path: 'frontend/src/runtime.ts' },
  },
  {
    name: 'preserves a direct updater command after an HTTPS string literal',
    mutate: (input) => { input.sourceFiles['frontend/src/runtime.ts'] = "const endpoint = 'https://provider.invalid'; invoke('plugin:updater|check')"; },
    expected: { code: 'updater.direct-command', path: 'frontend/src/runtime.ts' },
  },
  {
    name: 'rejects a backtick updater direct production invoke',
    mutate: (input) => { input.sourceFiles['frontend/src/runtime.ts'] = 'invoke(\n  `plugin:updater|check`,\n)'; },
    expected: { code: 'updater.direct-command', path: 'frontend/src/runtime.ts' },
  },
  {
    name: 'rejects a backtick process direct production invoke',
    mutate: (input) => { input.sourceFiles['frontend/src/runtime.ts'] = 'invoke (\n  `plugin:process|restart`\n);'; },
    expected: { code: 'process.direct-command', path: 'frontend/src/runtime.ts' },
  },
  {
    name: 'rejects an updater command inside a template expression',
    mutate: (input) => { input.sourceFiles['frontend/src/runtime.ts'] = 'const result = `${invoke("plugin:updater|check")}`'; },
    expected: { code: 'updater.direct-command', path: 'frontend/src/runtime.ts' },
  },
  {
    name: 'rejects an updater command stored before an indirect invoke',
    mutate: (input) => { input.sourceFiles['frontend/src/runtime.ts'] = 'const command = "plugin:updater|check"; invoke(command)'; },
    expected: { code: 'updater.direct-command', path: 'frontend/src/runtime.ts' },
  },
  {
    name: 'rejects a unicode-escaped updater command',
    mutate: (input) => { input.sourceFiles['frontend/src/runtime.ts'] = 'invoke("plugin:\\u0075pdater|check")'; },
    expected: { code: 'updater.direct-command', path: 'frontend/src/runtime.ts' },
  },
  {
    name: 'preserves updater detection after a URL regex literal',
    mutate: (input) => { input.sourceFiles['frontend/src/runtime.ts'] = 'const url = /https?:\\/\\//; invoke("plugin:updater|check")'; },
    expected: { code: 'updater.direct-command', path: 'frontend/src/runtime.ts' },
  },
  {
    name: 'preserves updater detection after a statement-leading URL regex literal',
    mutate: (input) => { input.sourceFiles['frontend/src/runtime.ts'] = 'if (enabled) /https?:\\/\\//.test(value); invoke("plugin:updater|check")'; },
    expected: { code: 'updater.direct-command', path: 'frontend/src/runtime.ts' },
  },
  {
    name: 'rejects an embedded updater capability permission',
    mutate: (input) => { input.tauriConfig.app.security.capabilities[0].permissions.push('updater:default'); },
    expected: { code: 'updater.embedded-permission', path: 'frontend/src-tauri/tauri.conf.json' },
  },
  {
    name: 'rejects an embedded process capability permission',
    mutate: (input) => { input.tauriConfig.app.security.capabilities[0].permissions.push('process:default'); },
    expected: { code: 'process.embedded-permission', path: 'frontend/src-tauri/tauri.conf.json' },
  },
  {
    name: 'rejects an embedded updater allow-check capability permission',
    mutate: (input) => { input.tauriConfig.app.security.capabilities[0].permissions.push('updater:allow-check'); },
    expected: { code: 'updater.embedded-permission', path: 'frontend/src-tauri/tauri.conf.json' },
  },
  {
    name: 'rejects an embedded process allow-restart capability permission',
    mutate: (input) => { input.tauriConfig.app.security.capabilities[0].permissions.push('process:allow-restart'); },
    expected: { code: 'process.embedded-permission', path: 'frontend/src-tauri/tauri.conf.json' },
  },
  {
    name: 'rejects an embedded updater capability permission object',
    mutate: (input) => { input.tauriConfig.app.security.capabilities[0].permissions.push({ identifier: 'updater:allow-check' }); },
    expected: { code: 'updater.embedded-permission', path: 'frontend/src-tauri/tauri.conf.json' },
  },
  {
    name: 'rejects an embedded process capability permission object',
    mutate: (input) => { input.tauriConfig.app.security.capabilities[0].permissions.push({ identifier: 'process:allow-restart' }); },
    expected: { code: 'process.embedded-permission', path: 'frontend/src-tauri/tauri.conf.json' },
  },
  {
    name: 'rejects an updater permission in a standalone capability',
    mutate: (input) => { input.capabilityFiles['frontend/src-tauri/capabilities/main.json'] = '{"permissions":["updater:default"]}'; },
    expected: { code: 'updater.capability-permission', path: 'frontend/src-tauri/capabilities/main.json' },
  },
  {
    name: 'rejects a process permission in a standalone capability',
    mutate: (input) => { input.capabilityFiles['frontend/src-tauri/capabilities/main.json'] = '{"permissions":["process:default"]}'; },
    expected: { code: 'process.capability-permission', path: 'frontend/src-tauri/capabilities/main.json' },
  },
  {
    name: 'rejects an updater allow-check permission in a standalone capability',
    mutate: (input) => { input.capabilityFiles['frontend/src-tauri/capabilities/main.json'] = '{"permissions":["updater:allow-check"]}'; },
    expected: { code: 'updater.capability-permission', path: 'frontend/src-tauri/capabilities/main.json' },
  },
  {
    name: 'rejects a process allow-restart permission in a standalone capability',
    mutate: (input) => { input.capabilityFiles['frontend/src-tauri/capabilities/main.json'] = '{"permissions":["process:allow-restart"]}'; },
    expected: { code: 'process.capability-permission', path: 'frontend/src-tauri/capabilities/main.json' },
  },
  {
    name: 'rejects an updater permission object in a standalone capability',
    mutate: (input) => { input.capabilityFiles['frontend/src-tauri/capabilities/main.json'] = '{"permissions":[{"identifier":"updater:allow-check"}]}'; },
    expected: { code: 'updater.capability-permission', path: 'frontend/src-tauri/capabilities/main.json' },
  },
  {
    name: 'rejects a process permission object in a standalone capability',
    mutate: (input) => { input.capabilityFiles['frontend/src-tauri/capabilities/main.json'] = '{"permissions":[{"identifier":"process:allow-restart"}]}'; },
    expected: { code: 'process.capability-permission', path: 'frontend/src-tauri/capabilities/main.json' },
  },
  {
    name: 'rejects updater endpoints configuration',
    mutate: (input) => { input.tauriConfig.plugins = { updater: { endpoints: ['https://updates.invalid/latest.json'] } }; },
    expected: { code: 'updater.config-endpoints', path: 'frontend/src-tauri/tauri.conf.json' },
  },
  {
    name: 'rejects updater public-key configuration',
    mutate: (input) => { input.tauriConfig.plugins = { updater: { pubkey: 'PUBLIC_UPDATE_KEY' } }; },
    expected: { code: 'updater.config-pubkey', path: 'frontend/src-tauri/tauri.conf.json' },
  },
  {
    name: 'rejects base updater artifact configuration even when false',
    mutate: (input) => { input.tauriConfig.bundle.createUpdaterArtifacts = false; },
    expected: { code: 'updater.config-artifacts', path: 'frontend/src-tauri/tauri.conf.json' },
  },
  {
    name: 'rejects Universal updater artifact configuration even when false',
    mutate: (input) => { input.universalConfig.bundle.createUpdaterArtifacts = false; },
    expected: { code: 'updater.config-artifacts', path: 'frontend/src-tauri/tauri.universal.conf.json' },
  },
  {
    name: 'rejects Universal updater endpoints configuration',
    mutate: (input) => { input.universalConfig.plugins = { updater: { endpoints: ['https://updates.invalid/latest.json'] } }; },
    expected: { code: 'updater.config-endpoints', path: 'frontend/src-tauri/tauri.universal.conf.json' },
  },
  {
    name: 'rejects Universal updater public-key configuration',
    mutate: (input) => { input.universalConfig.plugins = { updater: { pubkey: 'PUBLIC_UPDATE_KEY' } }; },
    expected: { code: 'updater.config-pubkey', path: 'frontend/src-tauri/tauri.universal.conf.json' },
  },
  {
    name: 'rejects Universal embedded updater permissions',
    mutate: (input) => { input.universalConfig.app = { security: { capabilities: [{ permissions: ['updater:allow-check'] }] } }; },
    expected: { code: 'updater.embedded-permission', path: 'frontend/src-tauri/tauri.universal.conf.json' },
  },
  {
    name: 'rejects Universal embedded process permission objects',
    mutate: (input) => { input.universalConfig.app = { security: { capabilities: [{ permissions: [{ identifier: 'process:allow-restart' }] }] } }; },
    expected: { code: 'process.embedded-permission', path: 'frontend/src-tauri/tauri.universal.conf.json' },
  },
  ...[
    'frontend/src/services/updateService.ts',
    'frontend/src/hooks/useUpdateCheck.ts',
    'frontend/src/components/UpdateCheckProvider.tsx',
    'frontend/src/components/UpdateDialog.tsx',
    'frontend/src/components/UpdateNotification.tsx',
  ].map((filePath) => ({
    name: `rejects tracked updater-owned file ${filePath}`,
    mutate: (input) => { input.trackedFiles.add(filePath); },
    expected: { code: 'updater.owned-file', path: filePath },
  })),
  {
    name: 'rejects the root layout updater provider mount',
    mutate: (input) => { input.sourceFiles['frontend/src/app/layout.tsx'] = '<UpdateCheckProvider>{children}</UpdateCheckProvider>'; },
    expected: { code: 'updater.layout-provider', path: 'frontend/src/app/layout.tsx' },
  },
  {
    name: 'rejects the About manual updater call',
    mutate: (input) => { input.sourceFiles['frontend/src/components/About.tsx'] = 'await updateService.checkForUpdates(true);'; },
    expected: { code: 'updater.about-call', path: 'frontend/src/components/About.tsx' },
  },
  {
    name: 'rejects the About update button text',
    mutate: (input) => { input.sourceFiles['frontend/src/components/About.tsx'] = '<Button>Check for Updates</Button>'; },
    expected: { code: 'updater.about-button', path: 'frontend/src/components/About.tsx' },
  },
  {
    name: 'rejects the About updater dialog render',
    mutate: (input) => { input.sourceFiles['frontend/src/components/About.tsx'] = '<UpdateDialog open={open} />'; },
    expected: { code: 'updater.about-dialog', path: 'frontend/src/components/About.tsx' },
  },
  {
    name: 'rejects the tray updater match arm',
    mutate: (input) => { input.sourceFiles['frontend/src-tauri/src/tray.rs'] = '"check_updates" => check_updates_handler(app),'; },
    expected: { code: 'updater.tray-match', path: 'frontend/src-tauri/src/tray.rs' },
  },
  {
    name: 'rejects the tray updater handler',
    mutate: (input) => { input.sourceFiles['frontend/src-tauri/src/tray.rs'] = 'fn check_updates_handler<R: Runtime>(app: &AppHandle<R>) {}'; },
    expected: { code: 'updater.tray-handler', path: 'frontend/src-tauri/src/tray.rs' },
  },
  {
    name: 'rejects the tray updater event',
    mutate: (input) => { input.sourceFiles['frontend/src-tauri/src/tray.rs'] = 'app.emit("check-updates-from-tray", ())'; },
    expected: { code: 'updater.tray-event', path: 'frontend/src-tauri/src/tray.rs' },
  },
  {
    name: 'rejects the tray updater menu item',
    mutate: (input) => { input.sourceFiles['frontend/src-tauri/src/tray.rs'] = 'MenuItemBuilder::with_id("check_updates", "Check for Updates")'; },
    expected: { code: 'updater.tray-menu', path: 'frontend/src-tauri/src/tray.rs' },
  },
  {
    name: 'rejects the tracked updater manifest script',
    mutate: (input) => { input.trackedFiles.add('scripts/generate-update-manifest-github.js'); },
    expected: { code: 'updater.manifest-script', path: 'scripts/generate-update-manifest-github.js' },
  },
  {
    name: 'rejects the tracked local updater test script',
    mutate: (input) => { input.trackedFiles.add('scripts/test-update-locally.js'); },
    expected: { code: 'updater.local-test-script', path: 'scripts/test-update-locally.js' },
  },
  {
    name: 'rejects the updater private-key workflow field',
    mutate: (input) => { input.workflowFiles['.github/workflows/build.yml'] = 'TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}'; },
    expected: { code: 'updater.signing-private-key', path: '.github/workflows/build.yml' },
  },
  {
    name: 'rejects the updater private-key-password workflow field',
    mutate: (input) => { input.workflowFiles['.github/workflows/build.yml'] = 'TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}'; },
    expected: { code: 'updater.signing-password', path: '.github/workflows/build.yml' },
  },
  {
    name: 'rejects an updater private key in a YAML flow map',
    mutate: (input) => { input.workflowFiles['.github/workflows/build.yml'] = 'env: { TAURI_SIGNING_PRIVATE_KEY: secret }'; },
    expected: { code: 'updater.signing-private-key', path: '.github/workflows/build.yml' },
  },
  {
    name: 'rejects a quoted updater password key in a YAML flow map',
    mutate: (input) => { input.workflowFiles['.github/workflows/build.yml'] = 'env: { "TAURI_SIGNING_PRIVATE_KEY_PASSWORD": secret }'; },
    expected: { code: 'updater.signing-password', path: '.github/workflows/build.yml' },
  },
  {
    name: 'rejects an explicit YAML updater signing key',
    mutate: (input) => { input.workflowFiles['.github/workflows/build.yml'] = 'env:\n  ? TAURI_SIGNING_PRIVATE_KEY\n  : secret'; },
    expected: { code: 'updater.signing-private-key', path: '.github/workflows/build.yml' },
  },
  {
    name: 'rejects an explicit YAML updater signing key in a flow map',
    mutate: (input) => { input.workflowFiles['.github/workflows/build.yml'] = 'env: { ? TAURI_SIGNING_PRIVATE_KEY : secret }'; },
    expected: { code: 'updater.signing-private-key', path: '.github/workflows/build.yml' },
  },
  {
    name: 'rejects a unicode-escaped YAML updater signing key',
    mutate: (input) => { input.workflowFiles['.github/workflows/build.yml'] = 'env: { "\\u0054AURI_SIGNING_PRIVATE_KEY": secret }'; },
    expected: { code: 'updater.signing-private-key', path: '.github/workflows/build.yml' },
  },
  {
    name: 'rejects latest.json manifest handling',
    mutate: (input) => { input.workflowFiles['.github/workflows/release.yml'] = 'run: gh release download --pattern latest.json'; },
    expected: { code: 'updater.latest-manifest', path: '.github/workflows/release.yml' },
  },
  {
    name: 'rejects macOS updater archive globs',
    mutate: (input) => { input.workflowFiles['.github/workflows/build-macos.yml'] = 'path: target/release/bundle/macos/*.app.tar.gz'; },
    expected: { code: 'updater.macos-archive', path: '.github/workflows/build-macos.yml' },
  },
  {
    name: 'rejects macOS updater signature globs',
    mutate: (input) => { input.workflowFiles['.github/workflows/build-macos.yml'] = 'path: target/release/bundle/macos/*.app.tar.gz.sig'; },
    expected: { code: 'updater.macos-signature', path: '.github/workflows/build-macos.yml' },
  },
  {
    name: 'rejects MSI updater signature globs',
    mutate: (input) => { input.workflowFiles['.github/workflows/build-windows.yml'] = 'path: target/release/bundle/msi/*.msi.sig'; },
    expected: { code: 'updater.msi-signature', path: '.github/workflows/build-windows.yml' },
  },
  {
    name: 'rejects executable updater signature globs',
    mutate: (input) => { input.workflowFiles['.github/workflows/build-windows.yml'] = 'path: target/release/bundle/nsis/*.exe.sig'; },
    expected: { code: 'updater.exe-signature', path: '.github/workflows/build-windows.yml' },
  },
];

for (const { name, mutate, expected } of updaterSurfaceCases) {
  test(name, () => {
    const input = cleanUpdaterPolicyFixture();
    mutate(input);
    assert.deepEqual(
      validateUpdaterDisabled(input).map(({ code, path }) => ({ code, path })),
      [expected],
    );
  });
}

const updaterPolicyFalsePositiveCases = [
  {
    name: 'resets Cargo dependency state at an array-table header',
    mutate: (input) => { input.cargoToml += '\n[[bin]]\ntauri-plugin-updater = "not-a-dependency"\n'; },
  },
  {
    name: 'allows import-shaped text inside a multiline template literal',
    mutate: (input) => { input.sourceFiles['frontend/src/runtime.ts'] = 'const example = `\nimport{check}from"@tauri-apps/plugin-updater"\n`'; },
  },
  {
    name: 'allows updater artifact text in an inline YAML comment',
    mutate: (input) => { input.workflowFiles['.github/workflows/build.yml'] = 'run: echo done # latest.json removed'; },
  },
  {
    name: 'allows updater examples inside a multiline Cargo description',
    mutate: (input) => {
      input.cargoToml = '[package]\nname = "meetily"\ndescription = """\n[dependencies]\ntauri-plugin-updater = "example only"\n"""\n\n[dependencies]\ntauri = "2"\n';
    },
  },
];

for (const { name, mutate } of updaterPolicyFalsePositiveCases) {
  test(name, () => {
    const input = cleanUpdaterPolicyFixture();
    mutate(input);
    assert.deepEqual(validateUpdaterDisabled(input), []);
  });
}

const typescriptCommentCases = [
  {
    name: 'allows a direct updater command inside a TypeScript line comment',
    source: '// invoke(`plugin:updater|check`)',
  },
  {
    name: 'allows an updater package import inside a TypeScript block comment',
    source: "/*\nimport { check } from '@tauri-apps/plugin-updater';\n*/",
  },
  {
    name: 'allows a process package import inside a TypeScript block comment',
    source: "/*\nimport { relaunch } from '@tauri-apps/plugin-process';\n*/",
  },
];

for (const { name, source } of typescriptCommentCases) {
  test(name, () => {
    const input = cleanUpdaterPolicyFixture();
    input.sourceFiles['frontend/src/runtime.ts'] = source;
    assert.deepEqual(validateUpdaterDisabled(input), []);
  });
}

test('allows unrelated provider network, platform signing, update words, tests, and mocks', () => {
  const input = cleanUpdaterPolicyFixture();
  input.sourceFiles = {
    'frontend/src/components/SummaryUpdaterButtonGroup.tsx': 'import { getVersion } from "@tauri-apps/api/app"; // updater words are allowed here',
    'frontend/src/providers.ts': [
      'const localProvider = "https://localhost:11434";',
      'const customProvider = `https://custom-openai.example/${model}`;',
      'const openAiProvider = "https://api.openai.com/v1";',
      'const anthropicProvider = "https://api.anthropic.com";',
      'const groqProvider = "https://api.groq.com/openai/v1";',
      'const openRouterProvider = "https://openrouter.ai/api/v1";',
      'const posthogProvider = "https://us.i.posthog.com";',
      'const licenseProvider = "https://project.supabase.co";',
      'const modelProvider = "https://huggingface.co/models";',
    ].join('\n'),
    'frontend/src/testing/tauri-browser-mocks.ts': "invoke('plugin:updater|check')",
    'frontend/tests/updater.spec.ts': "invoke('plugin:updater|check')",
    'frontend/src-tauri/src/lib.rs': 'let endpoint = "https://provider.invalid"; // .plugin(tauri_plugin_updater::Builder::new().build())',
    'frontend/src-tauri/src/tray.rs': '// "check_updates" => check_updates_handler(app)',
  };
  input.tauriConfig.license = { pubkey: 'PUBLIC_LICENSE_KEY' };
  input.tauriConfig.app.security.capabilities[0].permissions.push({ identifier: 'fs:scope', allow: ['$HOME/**'] });
  input.capabilityFiles['frontend/src-tauri/capabilities/filesystem.json'] = '{"permissions":[{"identifier":"fs:scope","allow":["$HOME/**"]}]}';
  input.workflowFiles['.github/workflows/build.yml'] = [
    '# TAURI_SIGNING_PRIVATE_KEY: updater comment only',
    '# latest.json and *.app.tar.gz are updater comments only',
    'APPLE_SIGNING_IDENTITY: Developer ID Application',
    'DIGICERT_CERTIFICATE: ${{ secrets.DIGICERT_CERTIFICATE }}',
    'path: artifacts/platform-signature.sig',
  ].join('\n');
  input.cargoToml += '\n[dev-dependencies]\ntauri-plugin-updater = "2"\n\n[target.\'cfg(test)\'.dev-dependencies]\ntauri-plugin-process = "2"\n';
  input.cargoLock += '\n# [[package]]\n# name = "tauri-plugin-updater"\n';

  assert.deepEqual(validateUpdaterDisabled(input), []);
});

function validFixture() {
  return {
    existingPaths: new Set([
      'src-tauri/icons/icon.png',
      'src-tauri/icons/app_icon.icns',
      'src-tauri/icons/app_icon.ico',
      'src-tauri/templates/default.json',
      'src-tauri/resources/diarization/licenses/Apache-2.0.txt',
    ]),
    nextConfig: { output: 'export' },
    packageJson: {
      scripts: { build: 'next build' },
      version: '0.4.1',
    },
    tauriConfig: {
      productName: 'meetily',
      version: '0.4.1',
      identifier: 'com.meetily.ai',
      build: {
        beforeBuildCommand: 'pnpm build',
        devUrl: 'http://localhost:3118',
        frontendDist: '../out',
      },
      bundle: {
        active: true,
        externalBin: ['binaries/llama-helper', 'binaries/ffmpeg'],
        icon: ['icons/icon.png', 'icons/app_icon.icns', 'icons/app_icon.ico'],
        macOS: {
          hardenedRuntime: true,
          minimumSystemVersion: '14.2',
          signingIdentity: '-',
        },
        resources: {
          'templates/': 'templates/',
          'resources/diarization/': 'diarization/',
        },
      },
    },
    universalConfig: {
      bundle: {
        resources: {
          'templates/': 'templates/',
          'resources/diarization/': 'diarization/',
        },
      },
    },
  };
}

test('accepts the production static-export and Tauri bundle contract', () => {
  assert.deepEqual(validateProductionConfig(validFixture()), []);
});

test('reports every unsafe or incomplete production setting', () => {
  const fixture = validFixture();
  fixture.nextConfig.output = undefined;
  fixture.packageJson.version = '0.4.2';
  fixture.tauriConfig.build.frontendDist = 'http://localhost:3118';
  fixture.tauriConfig.build.beforeBuildCommand = 'pnpm dev';
  fixture.tauriConfig.bundle.externalBin = ['binaries/ffmpeg'];
  fixture.tauriConfig.bundle.macOS.hardenedRuntime = false;
  fixture.tauriConfig.bundle.macOS.minimumSystemVersion = '10.13';
  fixture.tauriConfig.bundle.macOS.signingIdentity = '/Users/developer/key.p12';
  fixture.existingPaths.delete('src-tauri/icons/app_icon.icns');
  fixture.existingPaths.delete('src-tauri/templates/default.json');

  const codes = validateProductionConfig(fixture).map(({ code }) => code);

  assert.deepEqual(codes.sort(), [
    'bundle.hardened-runtime',
    'bundle.missing-external-bin',
    'bundle.missing-icon',
    'bundle.missing-resource',
    'bundle.minimum-system-version',
    'bundle.signing-identity',
    'config.version-mismatch',
    'next.static-export',
    'tauri.before-build',
    'tauri.frontend-dist',
  ].sort());
});

test('rejects a conflicting Universal macOS deployment target', () => {
  const fixture = validFixture();
  fixture.universalConfig.bundle.macOS = { minimumSystemVersion: '10.13' };

  assert.deepEqual(
    validateProductionConfig(fixture).map(({ code, path }) => ({ code, path })),
    [{ code: 'universal.minimum-system-version', path: 'src-tauri/tauri.universal.conf.json' }],
  );
});

test('requires diarization resources in both production and Universal configs', () => {
  const fixture = validFixture();
  fixture.tauriConfig.bundle.resources = { 'templates/': 'templates/' };
  fixture.universalConfig.bundle.resources = { 'templates/': 'templates/' };

  const codes = validateProductionConfig(fixture).map(({ code }) => code);
  assert.deepEqual(codes.sort(), [
    'bundle.missing-diarization-resources',
    'universal.missing-diarization-resources',
  ]);
});

test('rejects private keys, credential literals, developer paths, and source dev URLs', () => {
  const privateKey = `-----BEGIN ${'PRIVATE KEY'}-----`;
  const unixPath = `/${'Users'}/release-engineer/Meetily/model.gguf`;
  const windowsPath = `C:${'\\\\'}Users${'\\\\'}release-engineer${'\\\\'}Meetily`;
  const credential = `api_key = "${'sk-' + 'liveCredentialValue1234567890'}"`;

  assert.equal(scanTextFile('config/private.pem', privateKey)[0].code, 'secret.private-key');
  assert.equal(scanTextFile('src/config.ts', credential)[0].code, 'secret.credential-literal');
  assert.equal(scanTextFile('src/model.ts', unixPath)[0].code, 'path.developer-home');
  assert.equal(scanTextFile('src/model.ts', windowsPath)[0].code, 'path.developer-home');
  assert.equal(
    scanTextFile('src/runtime.ts', "fetch('http://localhost:3118/api')")[0].code,
    'production.dev-server',
  );
});

test('allows environment variable names, public keys, documented providers, and Tauri devUrl', () => {
  const samples = [
    ['src/config.ts', 'const key = process.env.OPENAI_API_KEY'],
    ['src/config.rs', 'let key = std::env::var("OPENAI_API_KEY")?;'],
    ['src-tauri/tauri.conf.json', '"pubkey": "PUBLIC_UPDATE_KEY"'],
    ['src/providers.ts', "const endpoint = 'https://api.openai.com/v1'"],
    ['src-tauri/tauri.conf.json', '"devUrl": "http://localhost:3118"'],
    ['src/analytics.rs', 'api_key: "phc_PublicProjectToken123456789012345"'],
  ];

  for (const [path, content] of samples) {
    assert.deepEqual(scanTextFile(path, content), [], path);
  }
});

test('allows only the reviewed third-party build prefixes in the candidate verifier', () => {
  const verifierPath = 'frontend/scripts/verify-release-candidate.mjs';
  const reviewedPrefixes = [
    '/Users/runner/work/ort-artifacts/ort-artifacts/',
    '/Users/runner/work/onnxruntime-build/onnxruntime-build/',
    '/Users/runner/work/sherpa-onnx/sherpa-onnx/',
    '/Users/Shared/ffmpeg/',
  ];

  assert.deepEqual(scanTextFile(verifierPath, reviewedPrefixes.join('\n')), []);
  assert.equal(
    scanTextFile(verifierPath, '/Users/unreviewed/private-build/')[0].code,
    'path.developer-home',
  );
  assert.equal(
    scanTextFile('frontend/src/runtime.ts', reviewedPrefixes[0])[0].code,
    'path.developer-home',
  );
});

test('rejects a PostHog personal API key even though public project tokens are allowed', () => {
  const personalKey = `api_key: "${'phx_PrivatePersonalToken123456789'}"`;
  assert.equal(
    scanTextFile('src/analytics.rs', personalKey)[0].code,
    'secret.credential-literal',
  );
});

test('findings include a path and remediation', () => {
  const credential = `const ${'password'} = '${'this-is-an-accidental-password'}';`;
  const finding = scanTextFile(
    'src/runtime.ts',
    credential,
  )[0];

  assert.equal(finding.path, 'src/runtime.ts');
  assert.ok(finding.message.length > 20);
  assert.ok(finding.remediation.length > 20);
});
