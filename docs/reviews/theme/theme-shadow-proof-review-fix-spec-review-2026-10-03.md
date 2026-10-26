> Public privacy copy: local user paths and contact/provenance fields were anonymized. Historical test results and original hashes do not verify this edited copy. See [publication notes](/docs/development/public-source-snapshot.md).

# Text shadow proof freshness fix: independent bounded SPEC review

2026-10-03. **SPEC PASS — Critical 0 / Important 0 / Minor 0.** The prior disabled-phase freshness Important is closed. Named real text measurements now complete on the supplied browser snapshot. This does not close full theme acceptance.

## Exact snapshot and review

Task root `/Users/developer/Desktop/meeting-minutes/.worktrees/semantic-theme-2026-10-03`, base `53f4415058fb01e38b8892a47ac036bfffa394ba`; accepted plan `83e82c5891c7d699485ab0910936a67bf4a8e6690e42eb2b6ec43df07767de41`.

- `frontend/tests/helpers/rendered-contrast.ts`: `432f308480192f96ae4ef59e7ac88e3a5ce27fdd3b84fab15fcf09938bad3b92`.
- `frontend/tests/theme-dark-surfaces.spec.ts`: `8caf61251d0185caab80c9580b6d89543b679e20164e35cb1393d20eb642b9f1`.

Reviewer read the complete new inventory/phase validation and restoration flow, transient-mutation fixtures, caller changes and actual TranscriptView timestamp span. Hashes match the supplied current snapshot. No source edits or reviewer-run browser tests.

## Finding closure

An opaque browser JSHandle retains actual DOM node and child identities, text, attributes, canonical inline properties/priorities, all enumerated computed properties, rectangles, generated pseudo state, font-face identities/state, viewport/DPR/scroll. Disabled-phase verification now runs around both after screenshots, allowing only the joint candidates' authorized box-shadow change; computed none remains separately required. Original-state checks and restoration still bracket the restored capture. Node replacement, foreign insertion and temporary style changes that later disappear no longer satisfy the proof. Evaluation uses the captured sample handle rather than silently reacquiring another matching locator node. All five captures preserve caret state and allow animation; the driver never disables unrelated paint.

Transient-style, inserted-paint and clone/replacement regressions explicitly require rejection and verify restoration. Joint-set removal, delayed override rejection, exact whole-clip PNG equality, source/region freshness, 16-candidate cap and finally restoration remain intact. No blind shadow skip or tolerance was added. This is a bounded state-checked browser proof, not a universal guarantee about arbitrary changes between event-loop observations.

The Meeting Details selector now targets the actual nested muted timestamp span (`TranscriptView.tsx:287`) instead of its mixed-color button container. That corrects sample identity while preserving mixed-text rejection. It does not lower the text threshold or omit the required timestamp sample. Both complete route tests still measure Home, Settings and Meeting Details with the same engine and attach ledgers even on failure.

## Evidence independently read

- `/private/tmp/meetily-theme-shadow-proof-fix-fixtures.log`: **8 passed, 10.6 seconds**, SHA-256 `3a7732ee1eb79df66773edc720bfcccb0e55ddad520401cacd65ab90c9cb3c45`. This run preceded the reported type-only inventory annotation and real-caller/ledger additions; it is not misrepresented as a fresh rerun of every final byte.
- `/private/tmp/meetily-theme-real-text-shadow-fixed.log`: terminal JSON **2 expected / 0 unexpected / 0 skipped / 0 flaky**, 43.636 seconds, SHA-256 `61a18b1c5555f257835a6f2b068050944d71ae638e5a15387f3386f458c3b45d`. Decoded each ledger: nine samples across all three routes in each mode, minimum **4.542288769612194 light** / **5.999400854306462 dark**; each mode records two shadow-proof entries. This supplies completed bounded real-text acceptance for the final caller path.

These are independent source/artifact reads; reviewer did not launch the processes or capture shell exits. Lint/typecheck were pending in the capsule and are not asserted PASS here. Focus/background scans retain their strict original paths and are not accepted through the text driver.

Next action: combine with independent QUALITY, finish pending static checks and remaining exact-snapshot theme gates. Dark background scans, snapshot drift, native accessibility and release are separate; no full theme/native/release PASS follows. `changed_paths`: only `docs/fork/theme-shadow-proof-review-fix-spec-review-2026-10-03.md`; no product/test/status/Git edits or Graphify.
