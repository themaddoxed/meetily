> Public privacy copy: local user paths and contact/provenance fields were anonymized. Historical test results and original hashes do not verify this edited copy. See [publication notes](/docs/development/public-source-snapshot.md).

# Theme policy fixtures: independent bounded SPEC review

2026-10-03. **SPEC PASS — Critical 0 / Important 0 / Minor 0.** No actionable finding. Scope is only the new Node imports and final policy-fixture test in theme-primitives; no full theme or browser acceptance follows.

## Exact snapshot and checks

Task root `/Users/developer/Desktop/meeting-minutes/.worktrees/semantic-theme-2026-10-03`, base `53f4415058fb01e38b8892a47ac036bfffa394ba`.

- `frontend/tests/theme-primitives.spec.ts`: `e3b78982d27a263783a98c468ec1c60024bd71f93caaa79cebae172c94eec648`.
- Authoritative `docs/superpowers/plans/2026-10-02-semantic-theme.md`: `e993b579f5b5adce9c418a76d30c479d21db893fc4a9c77e2c6c640bf925b005`.
- Unchanged production `frontend/scripts/check-theme-tokens.mjs`: `bb9cc97b853213c17be319b68c8a9c116f3e8ef5d74cfc35f58d553c3d40a82b`, independently compared with exact base.
- Read `/private/tmp/meetily-theme-policy.log`: one test passed in 1.6 seconds; SHA-256 `57b6a85e6d68b05f65f0d293ae03ebf9a4f425f98520f9683a4086b2251003b7`.

Reviewer inspected the complete scoped diff, actual production regexes and the test's CLI invocation/cleanup. The process result is independently read from the log, not a reviewer-run test or captured shell exit. Although Playwright's reporter labels the project chromium, this case requests no page/browser fixture and performs Node subprocess/file operations. The supplied temporary configuration disables webServer; no browser gate is claimed.

## Contract and isolation

Plan Task 3 line 298 requires representative forbidden forms and semantic-clean input, exact detection, unchanged CLI semantics and no audit weakening. The eleven fixtures cover all seven current regex families: white/black surfaces, bg-opacity, white text, fixed neutral surface/text/border including directional border, malformed doubled alpha, and hard-coded palette hex including uppercase. Coverage is representative rather than exhaustive over every numeric/color/extension alternative, which matches the plan's wording.

Each forbidden input is in a separate nested .tsx file. Exact sorted stderr equality checks every expected path/value and rejects missing or extra findings; status must be 1, without spawn error/signal or stdout. After removing those fixtures, semantic tokens must produce status 0 with empty streams. This verifies the unchanged production CLI's negative and positive behavior; it does not reimplement its regex logic inside the test.

The absolute audit path is resolved from the documented frontend cwd before spawning. A fresh mkdtemp directory owns the child cwd and src tree. All fixture writes/removals remain inside that directory, and finally removes only the generated directory. The application src and other worktrees are not scanned or modified. No new allowlist entry, CLI option, dependency or product change is introduced.

Next action: retain this scoped policy evidence alongside remaining theme checks. Existing contrast/focus/dark-surface/snapshot failures and native/release gates remain open; this passing Node fixture cannot close them. `changed_paths`: only `docs/fork/theme-policy-fixtures-spec-review-2026-10-03.md`. No product/test/status/Git edits or Graphify.
