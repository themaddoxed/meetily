# Meetily Testing System and Release Gate Implementation Plan

> **For Codex:** REQUIRED SUB-SKILL: Use `executing-plans` to implement this plan task by task. Apply `test-driven-development` for production code and `verification-before-completion` before every success claim.

**Goal:** Add a practical, evidence-producing testing system and mandatory release gate that protects Meetily recordings and transcripts and can be operated by a person without QA experience.

**Architecture:** Build three layers around the existing Tauri 2 application: deterministic repository checks, a verified macOS candidate builder, and a parsed manual smoke attestation. Add real file-backed SQLite tests at repository boundaries, browser IndexedDB recovery tests at the frontend boundary, and pure Node modules for policy/report validation. Keep real inference and macOS hardware in manual smoke; do not mock persistence or claim missing UI features.

**Tech Stack:** POSIX shell, Node.js built-in test runner, Vitest for existing TypeScript unit tests, ESLint 8/Next.js 14, Playwright, Rust/cargo/sqlx/SQLite/tempfile, Tauri 2, GitHub Actions.

## Baseline Recorded Before Implementation

- Branch: `feature/testing-release-gate` from clean `dev` commit `71285d2`.
- `pnpm exec tsc --noEmit`: passes.
- Existing Node tests: 12 pass.
- `cargo test --workspace --no-run`: passes after selecting full Xcode and provisioning FFmpeg/helper sidecars; existing warnings remain.
- `pnpm lint`: fails by opening interactive Next.js ESLint setup even though an unused flat config exists.
- `cargo fmt --all -- --check`: fails with a large pre-existing repository-wide diff.
- No user-facing speaker rename action or export UI was found. The Rust `save_transcript` file command exists but is not wired to the meeting UI.

The implementation must not hide the last four facts. They are release blockers until fixed or completed in product scope.

### Task 1: Make frontend checks deterministic

**Files:**

- Modify: `frontend/package.json`
- Modify: `frontend/pnpm-lock.yaml`
- Modify: `frontend/eslint.config.mjs`
- Create: `frontend/vitest.config.ts`
- Modify: `frontend/tests/lib/blocknote-markdown.test.ts`
- Modify: `frontend/tests/lib/summary-language-preferences.test.js`

**Step 1: Demonstrate the current lint/test runner failures**

Run:

```bash
cd frontend
pnpm lint
pnpm vitest run
```

Expected: lint requests interactive configuration; Vitest command is missing.

**Step 2: Add exact compatible development dependencies**

Run:

```bash
cd frontend
pnpm add --save-dev eslint@8.57.1 eslint-config-next@14.2.35 vitest@4.1.6
```

Use the lockfile-resolved Next.js 14 patch version. Do not upgrade Next.js or React.

**Step 3: Configure non-interactive lint and focused Vitest**

Update `eslint.config.mjs` with global ignores for `.next`, `out`, `playwright-report`, `test-results`, generated Tauri output, and fixture snapshots. Keep Next core-web-vitals and TypeScript rules.

Create `vitest.config.ts`:

```ts
import { defineConfig } from 'vitest/config';

export default defineConfig({
  resolve: { alias: { '@': new URL('./src', import.meta.url).pathname } },
  test: {
    environment: 'node',
    include: ['tests/lib/**/*.test.{ts,js}'],
    clearMocks: true,
    restoreMocks: true,
    watch: false,
  },
});
```

Change Bun imports/mocks to `vitest` equivalents. Keep Node built-in `.test.mjs` files outside Vitest.

Add package scripts:

```json
"lint": "eslint . --max-warnings=0",
"typecheck": "tsc --noEmit",
"test:node": "node --test tests/e2e-build-boundary.test.mjs tests/lib/*.test.mjs",
"test:unit": "vitest run",
"test:frontend": "pnpm test:node && pnpm test:unit"
```

**Step 4: Run the focused checks**

Run:

```bash
cd frontend
pnpm typecheck
pnpm test:frontend
pnpm lint
```

Expected: typecheck and both test runners execute. Lint may reveal real baseline violations; record them rather than weakening rules globally.

**Step 5: Commit**

```bash
git add frontend/package.json frontend/pnpm-lock.yaml frontend/eslint.config.mjs frontend/vitest.config.ts frontend/tests/lib
git commit -m "test(frontend): make checks deterministic"
```

### Task 2: Add a tested production-policy validator

**Files:**

- Create: `frontend/scripts/release-policy.mjs`
- Create: `frontend/tests/release-policy.test.mjs`
- Modify: `frontend/package.json`

**Step 1: Write failing policy tests**

Cover pure exported functions with temporary fixtures:

- accepts `output: 'export'`, `frontendDist: '../out'`, declared resources/sidecars, identifier/version/icons, and hardened runtime;
- rejects missing `frontendDist`, production E2E enablement, missing resources/sidecars, unsafe signing fields, and inconsistent package/Tauri versions;
- rejects tracked PEM private keys, high-confidence credential literals, `/Users/<name>/...`, and `C:\\Users\\<name>\\...`;
- does not flag secret environment variable names or documented external-provider endpoints merely for existing;
- rejects a production source reference to `localhost:3118` outside the Tauri `devUrl` field;
- reports every finding with a path and remediation.

Run:

```bash
cd frontend
node --test tests/release-policy.test.mjs
```

Expected: FAIL because the module does not exist.

**Step 2: Implement the minimum validator**

Implement pure functions and a CLI that:

- reads `package.json`, `next.config.js`, `src-tauri/tauri.conf.json`, and `src-tauri/tauri.universal.conf.json`;
- asks `git ls-files -z` for tracked files and scans only bounded text source/config files;
- checks required declared files on disk;
- uses narrow high-confidence secret patterns and an explicit in-code allowlist with reasons;
- prints a deterministic PASS/FAIL summary and exits nonzero on findings.

The scanner must say it is an accidental-secret/configuration check, not a complete security audit.

**Step 3: Run RED/GREEN verification**

Run the test, temporarily introduce one fixture violation inside the test, confirm failure, restore, then run:

```bash
cd frontend
node --test tests/release-policy.test.mjs
node scripts/release-policy.mjs
```

**Step 4: Add package script and commit**

Add `"check:policy": "node scripts/release-policy.mjs"`, then commit:

```bash
git add frontend/package.json frontend/scripts/release-policy.mjs frontend/tests/release-policy.test.mjs
git commit -m "test(release): validate production policy"
```

### Task 3: Add real file-backed SQLite data-integrity tests

**Files:**

- Create: `frontend/src-tauri/tests/data_integrity.rs`
- Reuse: `frontend/src-tauri/migrations/*.sql`
- Reuse: `frontend/src-tauri/src/database/manager.rs`
- Reuse: `frontend/src-tauri/src/database/repositories/{meeting,transcript,transcript_chunk,summary}.rs`

**Step 1: Add a file-backed test harness**

Use `tempfile::TempDir`, a `sqlite://...` URL, `SqlitePoolOptions::max_connections(1)`, and the production `sqlx::migrate!` migrator. Provide helpers to close and reopen a pool against the same file.

**Step 2: Add characterization tests for existing required behavior**

Add tests for:

1. a new file migrates to every current migration exactly once;
2. running the production migrator again is idempotent and keeps data;
3. `DatabaseManager::new` copies a representative previous `.db`, upgrades it, and preserves meeting/transcript data;
4. saving a meeting with timed transcript segments survives pool close/reopen;
5. saving an empty transcript creates a valid reopenable meeting without deleting other meetings;
6. transcript-chunk intermediate save/upsert survives restart;
7. a forced transcript insert failure rolls back the new meeting and leaves an existing meeting/transcript unchanged;
8. summary regeneration failure restores the previous summary and leaves transcript bytes unchanged;
9. summary failure without a previous summary still leaves the meeting/transcript intact.

These tests may pass immediately because they characterize existing code. If a test fails, stop and apply `systematic-debugging`; do not weaken the assertion.

**Step 3: Run focused tests**

```bash
DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
  cargo test -p meetily --test data_integrity -- --nocapture
```

Expected: all invariants pass or expose a production defect that is fixed in a separate RED/GREEN step.

**Step 4: Run migration and diarization suites together**

```bash
DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
  cargo test -p meetily --test data_integrity --test diarization_migration --test diarization_apply
```

**Step 5: Commit**

```bash
git add frontend/src-tauri/tests/data_integrity.rs
git commit -m "test(storage): verify data integrity and migrations"
```

### Task 4: Make the existing file export boundary atomic and test it

**Files:**

- Create: `frontend/src-tauri/src/export.rs`
- Modify: `frontend/src-tauri/src/lib.rs`
- Create: `frontend/src-tauri/tests/export_integrity.rs`

**Step 1: Write failing export tests**

Test a public `write_export_atomically(path, content)` helper against a real temporary directory:

- writes exact UTF-8 transcript/summary content;
- replaces an existing export without partial content;
- creates a missing parent directory;
- removes temporary files after success;
- returns an error for an invalid destination while preserving an existing sibling meeting file.

Run:

```bash
DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
  cargo test -p meetily --test export_integrity
```

Expected: FAIL because `app_lib::export` does not exist.

**Step 2: Implement the atomic helper**

Write to a uniquely named same-directory temporary file, `sync_all`, and rename over the destination. Clean only the helper's own temporary file on error. Do not change any meeting/database format.

Keep the existing Tauri command name `save_transcript`; delegate it to the helper so no frontend API changes.

**Step 3: Verify RED/GREEN**

Run focused tests. Temporarily revert delegation to direct write and confirm the replacement/cleanup regression test fails, then restore and rerun.

**Step 4: Commit**

```bash
git add frontend/src-tauri/src/export.rs frontend/src-tauri/src/lib.rs frontend/src-tauri/tests/export_integrity.rs
git commit -m "fix(export): write files atomically"
```

This proves the real backend export boundary. It does not claim a user-facing export button exists.

### Task 5: Extend interrupted-recording recovery evidence

**Files:**

- Modify: `frontend/tests/interrupted-recovery.spec.ts`
- Reuse: `frontend/src/services/indexedDBService.ts`
- Reuse: `frontend/src/testing/tauri-browser-mocks.ts`

**Step 1: Write the failing/characterization browser tests**

Using real browser IndexedDB and only mocking the heavy Tauri/model event boundary:

- seed a meeting with two transcript segments;
- emit `transcription-error` and `transcription-warning`;
- reload as a new session;
- verify the meeting remains recoverable and both transcript rows remain in IndexedDB;
- seed one malformed recovery item plus one healthy item and prove the healthy meeting still renders;
- verify an audio-checkpoint inspection failure leaves transcript recovery available.

Run:

```bash
cd frontend
pnpm exec playwright test tests/interrupted-recovery.spec.ts
```

If malformed-item isolation fails, add the smallest validation/filter in `indexedDBService.ts` with a failing unit/browser test first.

**Step 2: Verify and commit**

```bash
git add frontend/tests/interrupted-recovery.spec.ts frontend/src/services/indexedDBService.ts
git commit -m "test(recovery): preserve transcripts across failures"
```

### Task 6: Implement the aggregate pre-build command

**Files:**

- Create: `scripts/check.sh`
- Modify: `frontend/package.json`
- Create: `frontend/tests/check-command.test.mjs`

**Step 1: Write command-contract tests**

Test observable shell behavior:

- `--help` lists quick/full modes and every stage;
- an unknown flag exits nonzero;
- a failed child stage exits nonzero and names the failed standalone command;
- quick mode omits only Next production build and packaged candidate build;
- full mode includes production static export;
- no platform-unavailable stage prints PASS.

Use a temporary stub `PATH` to make child commands deterministic; do not assert only source text.

Run:

```bash
cd frontend
node --test tests/check-command.test.mjs
```

Expected: FAIL because `scripts/check.sh` does not exist.

**Step 2: Implement the shell orchestrator**

Use `set -eu`, repository-root discovery, named stages, elapsed time, and explicit failure output. Set full Xcode automatically when present. Run:

1. `git diff --check`;
2. frontend policy;
3. frontend lint;
4. TypeScript;
5. Node and Vitest unit tests;
6. Playwright critical recovery/contract subset;
7. Next production build in full mode;
8. `cargo fmt --all -- --check`;
9. `cargo clippy --workspace --all-targets -- -D warnings`;
10. `cargo test --workspace` including migration/data-integrity/export integration tests.

The script must not use `|| true`, baseline allowlists, or success text after a skipped check.

Add frontend aliases `check:quick` and `check` that call the root script, while keeping the root command canonical.

**Step 3: Run command tests and the real quick command**

```bash
cd frontend
node --test tests/check-command.test.mjs
cd ..
./scripts/check.sh --quick
```

Expected real baseline: deterministic failure at the first unresolved lint/format/clippy debt, with earlier stage results visible.

**Step 4: Commit**

```bash
git add scripts/check.sh frontend/package.json frontend/tests/check-command.test.mjs
git commit -m "build(test): add aggregate pre-build check"
```

### Task 7: Add candidate verification and clean-build command

**Files:**

- Create: `frontend/scripts/verify-release-candidate.mjs`
- Create: `frontend/tests/release-candidate.test.mjs`
- Create: `scripts/release-candidate.sh`
- Modify: `.gitignore`
- Modify: `frontend/package.json`

**Step 1: Write failing verifier tests**

Test pure metadata and scan functions with temporary fake bundle trees and injected command results:

- require product name, version, `com.meetily.ai`, executable, icons, templates, helper and FFmpeg;
- require both `arm64` and `x86_64` for main executable and sidecars in Universal mode;
- reject developer-home/repository paths, E2E bootstrap markers, obvious secret literals, and missing resources;
- distinguish ad-hoc signature from Developer ID signature;
- report notarization as `NOT_RUN`, never `PASS`, when Gatekeeper evidence is unavailable;
- emit a JSON report bound to commit SHA and artifact SHA-256.

Run:

```bash
cd frontend
node --test tests/release-candidate.test.mjs
```

**Step 2: Implement verifier and builder**

The shell command:

- refuses tracked changes for normal mode;
- records `git rev-parse HEAD`;
- runs `./scripts/check.sh`;
- seeds the documented local Universal artifact store;
- runs `pnpm --dir frontend tauri:build:universal`;
- invokes the verifier on the `.app` and DMG;
- writes ignored evidence under `artifacts/release-gate/<sha>/`;
- supports an explicit `--diagnostic-build` mode that may build after a failed pre-check but always exits nonzero and labels the artifact `UNGATED`.

Normal mode never builds or reports success after a failed pre-check.

**Step 3: Verify tests and help/preflight behavior**

```bash
cd frontend
node --test tests/release-candidate.test.mjs
cd ..
./scripts/release-candidate.sh --help
./scripts/release-candidate.sh
```

The last command is expected to remain blocked while mandatory baseline checks fail.

**Step 4: Commit**

```bash
git add .gitignore scripts/release-candidate.sh frontend/package.json frontend/scripts/verify-release-candidate.mjs frontend/tests/release-candidate.test.mjs
git commit -m "build(release): verify macOS candidates"
```

### Task 8: Add a machine-checked manual smoke report and release gate

**Files:**

- Create: `docs/release/smoke-report-template.md`
- Create: `frontend/scripts/verify-smoke-report.mjs`
- Create: `frontend/tests/smoke-report.test.mjs`
- Create: `scripts/release-gate.sh`

**Step 1: Write failing report parser tests**

Test that the parser rejects:

- placeholder operator/date/commit/checksum/hardware fields;
- missing required IDs;
- `FAIL`, `NOT_RUN`, blank evidence, or duplicate IDs;
- a report for another commit/artifact checksum;
- claims of signing/notarization without matching candidate report evidence.

Test a complete fixture passes parsing.

**Step 2: Implement template and parser**

Required smoke IDs cover launch, clean-install permissions, audio sources, mic/system capture, live transcript, normal finish, forced quit/recovery, permanent save, speaker operations, diarization success/failure, local summary success/failure, export UI/artifact, offline operation, restart/reopen, previous-version upgrade, old healthy meetings, logs, and checksum.

Use explicit `PASS | FAIL | NOT_RUN`; blank or unavailable is not PASS.

**Step 3: Implement release gate**

`release-gate.sh` requires an existing successful automatic candidate JSON, recomputes SHA-256, invokes the smoke parser, and prints one final status. Any mismatch or non-PASS required item exits nonzero.

**Step 4: Verify and commit**

```bash
cd frontend
node --test tests/smoke-report.test.mjs
cd ..
./scripts/release-gate.sh --help
git add docs/release scripts/release-gate.sh frontend/scripts/verify-smoke-report.mjs frontend/tests/smoke-report.test.mjs
git commit -m "build(release): require manual smoke evidence"
```

### Task 9: Add mandatory CI and reorder release creation

**Files:**

- Create: `.github/workflows/quality-gate.yml`
- Create: `.github/workflows/ci.yml`
- Modify: `.github/workflows/release.yml`
- Create: `frontend/tests/workflow-gate.test.mjs`

**Step 1: Write failing workflow contract tests**

Parse workflow YAML as text with focused structural assertions:

- CI triggers on pull requests and pushes to `dev` and `main`;
- reusable quality gate has frontend, macOS Rust/SQLite, macOS production-build, and final fan-in jobs;
- production build needs test jobs;
- permissions default to read-only;
- release `create-release` needs both quality gate and manual attestation;
- no release-creation action occurs in a job without those `needs` dependencies;
- no workflow publishes a release automatically.

**Step 2: Implement workflows**

`quality-gate.yml` supports `workflow_call`, installs exact pnpm/Node and Rust targets, runs frozen dependency install, provisions/builds required native helper sidecars, and uploads a native macOS production `.app` as inspection evidence. The Universal signed candidate remains the local/manual release step because CI has no committed Universal FFmpeg artifacts or signing secrets in the check job.

`ci.yml` calls the reusable gate for PR/push events.

`release.yml` adds required `manual-smoke-passed`, `smoke-report-reference`, and exact commit inputs, calls the reusable quality gate first, validates attestation, and only then creates a draft. Publication remains manual.

**Step 3: Verify and commit**

```bash
cd frontend
node --test tests/workflow-gate.test.mjs
cd ..
git diff --check
git add .github/workflows frontend/tests/workflow-gate.test.mjs
git commit -m "ci: enforce quality gate before release draft"
```

### Task 10: Add plain-language operator documentation and agent policy

**Files:**

- Create: `docs/testing.md`
- Create: `AGENTS.md`
- Modify: `README.md` only to link `docs/testing.md` if a development/testing section exists

**Step 1: Write documentation from actual commands**

In Russian, explain:

- quick, full, release-candidate, and release-gate commands;
- expected duration and what each stage proves;
- how to install dependencies and select Xcode;
- how to rerun one frontend, Playwright, Rust, migration, or policy test;
- clean install, previous-version upgrade, forced-quit recovery, offline, permissions, audio-device, model, diarization, summary, export, and restart smoke steps;
- log locations and the existing whole-diagnostics-folder/zip handoff;
- bug evidence fields;
- automatic versus manual checks;
- P0 release blockers and the current known missing speaker-rename/export UI;
- why an `UNGATED` diagnostic build is not a release candidate.

**Step 2: Add mandatory agent release policy**

`AGENTS.md` links to `docs/testing.md` and requires exact command/result reporting. It prohibits claiming permissions, hardware audio, offline model execution, signing, notarization, clean install, update, or release readiness without direct evidence.

**Step 3: Check documentation consistency**

```bash
rg -n 'scripts/(check|release-candidate|release-gate)\.sh' docs/testing.md AGENTS.md README.md
git diff --check
```

**Step 4: Commit**

```bash
git add AGENTS.md README.md docs/testing.md
git commit -m "docs(testing): document release gate and diagnostics"
```

### Task 11: Execute the full evidence run

**Files:**

- Create ignored output only: `artifacts/release-gate/<sha>/*`
- Do not modify source merely to make the report look green.

**Step 1: Verify repository status and focused tests**

```bash
git status --short
cd frontend
pnpm typecheck
pnpm test:frontend
node --test tests/release-policy.test.mjs tests/check-command.test.mjs tests/release-candidate.test.mjs tests/smoke-report.test.mjs tests/workflow-gate.test.mjs
cd ..
DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
  cargo test -p meetily --test data_integrity --test export_integrity --test diarization_migration --test diarization_apply
```

**Step 2: Run the automatic gate**

```bash
./scripts/check.sh
```

Record every stage as PASS/FAIL/NOT RUN. A baseline format/lint/clippy failure keeps the release blocked.

**Step 3: Build and verify a production artifact**

If normal candidate mode is blocked by a known automatic failure, run:

```bash
./scripts/release-candidate.sh --diagnostic-build
```

This provides factual production-build evidence while preserving a nonzero `UNGATED` status. If all automatic checks pass, run normal mode from a clean commit instead.

**Step 4: Run release gate with an unfilled template**

```bash
./scripts/release-gate.sh --smoke-report docs/release/smoke-report-template.md
```

Expected: FAIL because real Mac/hardware steps were not attested. This proves the gate cannot turn missing manual evidence into PASS.

**Step 5: Audit every user requirement**

Build a requirement/evidence table from the original goal and inspect current files, command output, candidate metadata, and manual status. Classify each as PASS, FAIL, NOT RUN, or MANUAL REQUIRED.

**Step 6: Final branch verification and handoff**

Read and apply `finishing-a-development-branch`. Do not merge or publish without the user's choice. The final report in Russian must include architecture, changed files, implemented tests, daily/release commands, factual results, manual hardware checks, risks, and P0/P1/P2 remaining work. It must explicitly say the release is not ready unless the complete release gate passed.
