# FEAT-082 — remove product analytics

Date: 2026-09-23
Result: source-level zero-analytics contract implemented and independently reviewed; packaged-app network behavior remains `NOT VERIFIED`.

## Snapshot and scope

- Product worktree: `.worktrees/feat071-audio-controls`, base `61e890f444f81718d8136beae6afc6f61a5f0e44`.
- Full tracked working-tree diff SHA-256: `9c835e454df467b4be8b92d1c05c7706e26a6fcaf9578f5192bde5f4b4a7d3b0`.
- New regression test: `frontend/tests/lib/product-analytics-absence.test.mjs`, SHA-256 `d9439833d042d94949f29239bf20251e0a14976b4b0b53fa83e1403e442bf7f0`.
- The product worktree already contained unrelated summary, persistence, test, and documentation edits. The full diff hash includes those edits. No staged changes or commits were made. In overlapping files, exact byte-level attribution to the analytics change is not available; the independent reviewer found no unexplained unrelated hunks.

## Change

- Removed the Rust analytics module and all registered analytics commands, the `posthog-rs` dependency, and its now-unused lockfile dependencies.
- Removed frontend analytics initialization, event facade and callsites, user-ID/preference persistence, consent controls, modal, and localization strings. The old `analyticsOptedIn` preference is no longer read by application code and cannot initialize telemetry.
- Replaced analytics payload tests and mocks with a source-absence regression check. Product actions remain in place where only a tracking call was removed.
- Kept model downloads and configured provider requests on their separate request paths. Source inspection found no application updater implementation in this snapshot; updater behavior is `NOT VERIFIED`.
- Static searches found no Xray or corporate-metrics identifiers in frontend or native application source. This is a source search, not a comparison against every donor revision.

## Verification

| Check | Result |
|---|---|
| `pnpm --dir frontend test:node` | 406 passed, 0 failed, 0 skipped |
| `pnpm --dir frontend typecheck` | Passed (`tsc --noEmit`) |
| `cargo check --manifest-path frontend/src-tauri/Cargo.toml --offline` | Passed; `dev` profile finished successfully. Offline Cargo resolution removed the unused PostHog dependency tree and normalized the lockfile. |
| `git diff --check` | Passed |
| Static absence search across app source and manifests | No PostHog, analytics client, preference, event command, or `Analytics.*` call matches. The frontend lockfile has no PostHog or analytics package. |
| Xray/corporate-metrics search in app source | No matches |
| Independent source review | `PASS WITH LIMITATIONS`, 0 actionable findings, against the same tracked-diff SHA |

The first `cargo check --locked --offline` correctly refused the manually pruned lockfile because Cargo needed to normalize the remaining graph. The subsequent offline check completed and compiled the current Rust application target successfully.

## Limits

No app bundle was rebuilt or launched, and no network capture was made. Thus source code and compilation support the absence of a product analytics path, but do not prove the behavior of a packaged binary or every transitive dependency at runtime. Recording, permission, and first-launch native scenarios remain blocked by FORK-119's only available `.app` being the known crash artifact and the required local Tao patch source being absent.
