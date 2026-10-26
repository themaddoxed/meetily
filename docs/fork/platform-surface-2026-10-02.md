# Platform surface scanner — 2 October 2026

FORK-148 adds an opt-in source audit to `main@53f4415058fb01e38b8892a47ac036bfffa394ba`.
The changes are uncommitted. This report is source evidence, not package,
native acceptance or release readiness. [Machine-readable evidence](evidence/platform-surface-2026-10-02.json)
records the scanner/package/test hashes and findings.

## Run and interpret

Use dependencies installed from the checkout's exact `frontend/pnpm-lock.yaml`:

```sh
pnpm --dir frontend check:platform
node frontend/scripts/platform-surface.mjs --root <repository> --json
```

Exit 0 is PASS within this bounded surface; exit 1 is FAIL with findings;
exit 2 is an execution error. Missing required configs and malformed
configuration/source fail closed. Git inventory includes tracked files and
nonignored untracked files; ignored build caches, worktrees and user data are
not an input. Symlinks and unreadable inventory entries are execution errors.
Files larger than 1 MiB and gitlinks are path-only inventory; their content is
not inspected. This is deliberately not an exhaustive platform/security scan.

The source scope covers Cargo manifests and `.cargo/config`, Tauri bundle
configs, frontend package scripts, GitHub workflow runner/matrix resolution,
local reusable workflow references, product/runtime/resource paths, build
scripts, Rust cfg, direct JS/TS platform conditions, selected shell conditions,
and current README/building/testing/workflow instructions. Dynamic unresolved
runners are findings. Historical plans/evidence/reports, tests/fixtures/vendor
source and the audit tools themselves are excluded with reasons in JSON output.
Runtime Python resources are forbidden; standalone root development Python
tools are allowed. Existing release-policy semantic runtime detection is reused
without weakening it, including its strict workflow/package launch rules.
Remote provider endpoints and Ollama on port 11434 are allowed; the obsolete
product backend tree and ports 5167/8178 remain forbidden.

Condition matching is bounded: Rust cfg expressions, direct JS/TS conditions
and comparisons involving platform selectors, and line-oriented shell
conditions. Indirect aliases, arbitrary generated commands, externally fetched
workflow code and binary contents need separate review. Audit PASS never proves
that a packaged application is macOS-only or that it runs on both architectures.

## Actual results

- Focused scanner tests: **30 passed, exit 0**. Negative fixtures cover runners,
  matrices, targets, paths, bundles, conditions, docs and runtime Python;
  positive fixtures separate dev tools, provider URLs, comments and history.
- Real temporary Git repositories exercise CLI PASS/FAIL/ERROR and both tracked
  and untracked injected installer paths. The initial stub failed these tests.
- Complete existing Node suite plus scanner: **454 passed, exit 0**.
- Configured frontend lint and typecheck: **exit 0**.
- Actual current root source audit: **48 findings, exit 1**. The root dependency
  tree differs from its lockfile, so execution used the isolated checkout's exact
  dependencies; the root source was selected explicitly with `--root`.

The remaining findings cover foreign runners/conditions/targets, old workflow
instructions, README/Linux build documentation, and these nine Rust files:

- `frontend/src-tauri/src/audio/decoder.rs`
- `frontend/src-tauri/src/audio/device_detection.rs`
- `frontend/src-tauri/src/audio/devices/discovery.rs`
- `frontend/src-tauri/src/audio/devices/speakers.rs`
- `frontend/src-tauri/src/audio/diagnostics.rs`
- `frontend/src-tauri/src/audio/encode.rs`
- `frontend/src-tauri/src/audio/incremental_saver.rs`
- `frontend/src-tauri/src/audio/playback_monitor.rs`
- `frontend/src-tauri/src/audio/recording_preferences.rs`

The Rust findings were confirmed against actual source cfg attributes. They
are residual source cleanup, not missing native evidence and not a reason to
rewrite previous task evidence. Preserve the old snapshots; remove only the
remaining foreign branches after their macOS behavior is reviewed and checked.
FORK-149 must verify the resulting source and package after FORK-035; it remains
open. The scanner is not wired into `scripts/check.sh` while these known failures
remain, and no existing gate was bypassed or described as passing.

No app installation, TCC change, device capture, hosted CI dispatch,
Developer ID signing, notarization, commit, push or release was performed for
this scanner change. An independent review was not performed; the diff was
reviewed in the implementation thread.
