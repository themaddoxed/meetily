> Public privacy copy: local user paths and contact/provenance fields were anonymized. Historical test results and original hashes do not verify this edited copy. See [publication notes](/docs/development/public-source-snapshot.md).

# Theme shared contrast helper: independent SPEC amendment review

2026-10-03. **SPEC PASS, Critical 0 / Important 0 / Minor 0.** No actionable finding in the bounded path amendment. This accepts the test-module boundary, not its unwritten implementation or any measured contrast result.

## Scope and snapshot

Read `docs/superpowers/plans/2026-10-02-semantic-theme.md`, “Shared rendered-contrast helper amendment”, lines 85–100, exact allowlist and Task 3 acceptance. SHA-256 before/after inspection: `e993b579f5b5adce9c418a76d30c479d21db893fc4a9c77e2c6c640bf925b005`.

Task root `/Users/developer/Desktop/meeting-minutes/.worktrees/semantic-theme-2026-10-03`; base `53f4415058fb01e38b8892a47ac036bfffa394ba`. Read-only filesystem check shows `frontend/tests/helpers/rendered-contrast.ts` is not created; `git cat-file -e <base>:<path>` confirms absence at base (expected exit 128). Scoped Git status still lists only the previously reviewed layout, ThemeScript and foundation test changes. No helper implementation exists to review.

## Contract fit

Exactly one new path is named and added as entry 19: `frontend/tests/helpers/rendered-contrast.ts`. The amendment explicitly makes it test-only, without registered tests or production callers, shared by dark-surface and foundation keyboard-focus suites. This avoids importing a test-registration module or putting test geometry into runtime ownership. It changes code placement, not thresholds or coverage.

Task 3 still requires full-precision ancestor/background/opacity compositing, opaque resolved canvas, explicit rejection of unsupported effects/geometry, real named Home/Settings/Meeting Details elements, and fail-on-missing locators. Text remains at least 4.5:1; meaningful indicators/focus remain at least 3:1 against both actual adjacent surfaces. Opaque/multilayer fixtures and rejection cases must test the same engine used on those real surfaces. Synthetic/token-only/helper-only GREEN cannot satisfy acceptance. Keyboard focus must be reached with real Tab navigation.

The explicit amendment governs the older Task 3 wording that locates the resolver in the dark-surface suite: its implementation is now shared in the named helper. No new production resolver, UI primitive, palette scope, dependency, snapshot or other file is authorized. The original 18 entries remain existing-file provenance; the nineteenth must be recorded as newly created, with its actual resulting hash, rather than claimed to match the historical census.

## Next action and boundaries

After matching independent QUALITY amendment acceptance, create the helper and targeted fixtures within existing test files, then verify both consumers use that same engine. Freeze new hashes and obtain implementation review of compositing/group opacity, fail-closed geometry and actual surface/focus measurements. No production token/focus correction before focused RED, and no snapshot update under this amendment.

Current full-suite activity and prior adapter PASS are separate evidence; neither establishes contrast acceptance. Reviewer ran no browser, contrast, native or release checks in this amendment review. `changed_paths`: only `docs/fork/theme-contrast-helper-amendment-spec-review-2026-10-03.md`; no product/plan/status/Git changes or Graphify. No SPEC blocker remains for the bounded helper creation after QUALITY; complete theme acceptance remains open.
