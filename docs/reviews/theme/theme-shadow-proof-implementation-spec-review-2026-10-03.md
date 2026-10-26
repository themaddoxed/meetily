> Public privacy copy: local user paths and contact/provenance fields were anonymized. Historical test results and original hashes do not verify this edited copy. See [publication notes](/docs/development/public-source-snapshot.md).

# Text shadow-proof implementation: independent bounded SPEC review

2026-10-03. **FAIL — Critical 0 / Important 1 / Minor 0.** Joint intervention and restoration are implemented, but the disabled phase does not enforce the accepted freshness contract.

## Important: unverified non-shadow state during disabled captures

`frontend/tests/helpers/rendered-contrast.ts:344–350` checks only each candidate's computed box-shadow:none around the two disabled captures. Full candidate/sample/ancestor/region freshness is checked before disabling (lines 334–339) and after restoration (lines 362–366), not while the intervention is active. A style observer or transition can change another candidate/ancestor/sample property while none is set, then restore it when box-shadow is restored. Both outer fingerprints pass; the disabled images were taken under more than the intended single intervention. The current stale fixture leaves its color mutation permanent, so it does not cover a transient change that returns on restoration.

Requirement: accepted plan rejects “any ... stale candidate/style/ancestor/region fingerprint” and permits overriding only candidate box-shadow. Classification: partial verification / causal-proof false PASS risk. Add expected-disabled fingerprint checks before and after both disabled captures, normalizing only the authorized shadow changes for the full joint candidate set; require every other computed style, identity, ancestor, sample and region field unchanged. Add a fixture that changes another property on disable and restores it on re-enable; it must reject despite matching final original state. Do not treat PNG equality as a substitute for the independent freshness invariant.

## What is supported by the code

The driver is text-only and uses the unchanged numerical acceptance engine after proof. Candidate discovery requires disjoint actual border boxes and preserves rejection of non-shadow effects/overlap. The set grows to at most 16 candidates; every expansion disables the entire set jointly, preventing mutually masking individual certificates. It checks actual computed none with a five-second bound, compares five exact whole-region PNG buffers, restores original inline values/priorities through allSettled finally, waits for original computed shadows and checks original fingerprints again. Focus/background callers continue without the driver. The proof ledger stores joint indices, fingerprint digests and a baseline PNG/hash. These strengths do not close the disabled-phase gap.

Read installed Playwright 1.61.0 screenshot implementation: without fullPage, clip is viewport-relative. The current getBoundingClientRect/outward-rounding/in-viewport clip matches that API; no scroll-coordinate finding is raised.

## Snapshot and evidence

Task root `/Users/developer/Desktop/meeting-minutes/.worktrees/semantic-theme-2026-10-03`, base `53f4415058fb01e38b8892a47ac036bfffa394ba`, accepted plan `83e82c5891c7d699485ab0910936a67bf4a8e6690e42eb2b6ec43df07767de41`.

- Helper exact SHA-256 `c140f4945042f80ad4245159cce7e8587c5f5771185fba5de25ce4d0d2b2be77`.
- Dark-surface tests `9a068bb682f350a469d1acfb9c52c9d5dc99423807b309bc2b68ea77f853aebb`.

Read complete browser protocol/driver, fixtures and caller changes. Reviewer did not execute tests. `/private/tmp/meetily-theme-shadow-proof-fixtures.log` records five passes, 10.9 seconds, preceding the reported evidence-only addition. `/private/tmp/meetily-theme-real-text-shadow.log` is terminal JSON: **0 expected / 2 unexpected / 0 skipped**, duration 57.764 seconds. Both named real-route tests fail on mixed timestamp descendant text color; neither completed real-route contrast acceptance. This is not evidence of bad contrast, and the passing synthetic fixtures cannot replace it.

Next action: close the disabled-state invariant with its regression, then review exact corrected bytes. Select the actual timestamp text leaf or otherwise preserve the existing mixed-text rejection when repairing the real locator; do not weaken the oracle to green those tests. Full theme, native and release gates remain open.

`changed_paths`: only `docs/fork/theme-shadow-proof-implementation-spec-review-2026-10-03.md`; no helper/test/source/status/Git edits or Graphify.
