> Public privacy copy: local user paths and contact/provenance fields were anonymized. Historical test results and original hashes do not verify this edited copy. See [publication notes](/docs/development/public-source-snapshot.md).

# Bug Reproducer

## ✅ FIX_PROVEN — Bug reproduced and fix proven

> The same reproducer changed from failing to passing and broader checks passed.

**Project:** Meetily
**Bug:** Meetily 0.4.1 Alpha: summary remains PENDING after interrupted regeneration
**Environment:** Meetily dev commit 6f2c4488edc6b9fffd45f8b2dd31bcd6079fe37e; macOS 15.7.7 x86_64; rustc 1.95.0; cargo 1.95.0
**Generated:** 2026-07-26

## Discovery scope

- Summary regeneration state transitions in frontend/src-tauri/src/database/repositories/summary.rs
- Production database startup in frontend/src-tauri/src/database/manager.rs
- Summary task lifecycle in frontend/src-tauri/src/summary/commands.rs
- Real temporary SQLite preservation tests in frontend/src-tauri/tests/data_integrity.rs

## Ranked and tested candidates

| # | Candidate | Contract evidence | Trigger | Location | Confidence | Outcome |
|---:|---|---|---|---|---|---|
| 1 | Interrupted summary regeneration remains permanently PENDING after restart | A restarted process cannot resume a destroyed process-local summary task, while transcript and the previous completed summary must remain preserved. | Start regeneration for an existing summary, terminate before the background task reaches a terminal state, then reopen through DatabaseManager::new. | /Users/developer/Desktop/meeting-minutes/.worktrees/dev/frontend/src-tauri/src/database/repositories/summary.rs:116 | high | REPRODUCED |

## Original report

Audit the existing Meetily 0.4.1 Alpha build and prove a concrete correctness defect before applying an approved fix.

| Contract | Expected | Actual |
|---|---|---|
| Observed behavior | On the next production database startup, an interrupted PENDING summary becomes failed, the previous summary is restored, and transcript bytes remain unchanged. | Before the fix, the same production startup path reopened the summary with status PENDING even though its background task no longer existed; the previous summary and transcript survived. |

## Minimal reproduction

A real temporary SQLite database seeds a completed summary and transcript, calls the production reset path, closes the pool, and reopens through production DatabaseManager::new.

**Confirming signal:** The focused test exited 101 with assertion left PENDING, right PENDING.

### Reproduction files approved at Gate 1

- [data_integrity.rs](/Users/developer/Desktop/meeting-minutes/.worktrees/dev/frontend/src-tauri/tests/data_integrity.rs:450) — Gate 1 regression test using real SQLite and the production startup path.

## Red to green evidence

| Evidence | Before fix | After fix |
|---|---:|---:|
| Exit code | 101 | 0 |
| Timed out | False | False |
| Duration | 14,985.389 ms | 14,958.137 ms |
| Same command | — | True |
| Broader suite | — | passed |

### Before — failing evidence

```text
Compiling meetily v0.4.1 (/Users/developer/Desktop/meeting-minutes/.worktrees/dev/frontend/src-tauri)
warning: meetily@0.4.1: 🚀 Building Meetily for: macos
warning: meetily@0.4.1: ✅ macOS: Metal GPU acceleration ENABLED by default
warning: meetily@0.4.1: 🎬 Checking FFmpeg binary for target: x86_64-apple-darwin
warning: meetily@0.4.1: 🔍 Found cached FFmpeg binary: ffmpeg-x86_64-apple-darwin
warning: meetily@0.4.1: ✅ FFmpeg verification passed: ffmpeg version 8.0.1-tessus  https://evermeet.cx/ffmpeg/  Copyright (c) 2000-2025 the FFmpeg developers
warning: meetily@0.4.1: ✅ FFmpeg binary already cached and verified: ffmpeg-x86_64-apple-darwin
warning: meetily@0.4.1: ✅ Universal FFmpeg binary is ready
    Finished ˋtestˋ profile [unoptimized + debuginfo] target(s) in 13.76s
     Running tests/data_integrity.rs (/Users/developer/Desktop/meeting-minutes/.worktrees/dev/target/debug/deps/data_integrity-2e71db128b1760c5)

thread 'interrupted_summary_regeneration_does_not_remain_pending_after_restart' (14934918) panicked at frontend/src-tauri/tests/data_integrity.rs:493:5:
assertion ˋleft != rightˋ failed: a restarted process cannot resume the destroyed background task
  left: "PENDING"
 right: "PENDING"
note: run with ˋRUST_BACKTRACE=1ˋ environment variable to display a backtrace
error: test failed, to rerun pass ˋ-p meetily --test data_integrityˋ
```

### After — fixed evidence

```text
Compiling meetily v0.4.1 (/Users/developer/Desktop/meeting-minutes/.worktrees/dev/frontend/src-tauri)
warning: meetily@0.4.1: 🚀 Building Meetily for: macos
warning: meetily@0.4.1: ✅ macOS: Metal GPU acceleration ENABLED by default
warning: meetily@0.4.1: 🎬 Checking FFmpeg binary for target: x86_64-apple-darwin
warning: meetily@0.4.1: 🔍 Found cached FFmpeg binary: ffmpeg-x86_64-apple-darwin
warning: meetily@0.4.1: ✅ FFmpeg verification passed: ffmpeg version 8.0.1-tessus  https://evermeet.cx/ffmpeg/  Copyright (c) 2000-2025 the FFmpeg developers
warning: meetily@0.4.1: ✅ FFmpeg binary already cached and verified: ffmpeg-x86_64-apple-darwin
warning: meetily@0.4.1: ✅ Universal FFmpeg binary is ready
    Finished ˋtestˋ profile [unoptimized + debuginfo] target(s) in 13.72s
     Running tests/data_integrity.rs (target/debug/deps/data_integrity-fcfe3c454974d188)
```

## Root cause

create_or_reset_process persisted PENDING before spawning a process-local asynchronous task, while DatabaseManager::new applied migrations without reconciling PENDING rows left by a terminated process.

## Approved fix

After migrations, production startup atomically changes only PENDING summary rows to failed, restores result_backup, records the interruption error and terminal timestamps, and clears backup fields.

**Why this is causal:** The startup recovery executes after the destroyed task can no longer report a terminal state and updates the exact persisted state responsible for the permanent PENDING status.

### Production files approved at Gate 2

- [summary.rs](/Users/developer/Desktop/meeting-minutes/.worktrees/dev/frontend/src-tauri/src/database/repositories/summary.rs:10) — Gate 2 repository recovery for interrupted PENDING rows.
- [manager.rs](/Users/developer/Desktop/meeting-minutes/.worktrees/dev/frontend/src-tauri/src/database/manager.rs:84) — Runs recovery after successful migrations during production database startup.

## Verification

| Check | Status | Evidence |
|---|---|---|
| Exact regression test | ✅ passed | Same command changed from exit 101 to exit 0. |
| Data integrity integration suite | ✅ passed | 10 passed, 0 failed on real temporary SQLite storage. |
| Meetily full automatic check | ✅ passed | ./scripts/check.sh --full exited 0 in 227 seconds, including 26 Playwright tests, production frontend build, Clippy, workspace tests, and packaged FFmpeg recovery. |
| Patch integrity | ✅ passed | cargo fmt --check and git diff --check passed. |

## Reproduce

```bash
cargo test -p meetily --test data_integrity interrupted_summary_regeneration_does_not_remain_pending_after_restart -- --exact --nocapture
```
```bash
cargo test -p meetily --test data_integrity -- --nocapture
```
```bash
./scripts/check.sh --full
```

## Limitations

- The code change was not committed, packaged, signed, notarized, or exercised through a newly built installed app.
- The previously audited 0.4.1 Alpha DMG is unchanged and remains NOT READY for public release.

## Residual risks

- Recovery is intentionally performed on the next application database startup; it does not alter the behavior of an in-process running task.

## Notes

- No migration, dependency, public API, Git history, or GitHub state changed.
- Gate 1 and Gate 2 approvals were obtained separately.

---

Generated by `$bug-reproducer`. A fix is proven only by the same red-to-green reproducer plus relevant broader checks.
