# Background absence proof plan v4 — independent QUALITY review

Source-authored by `/root/plans_quality`, 2026-10-04.

**Bounded QUALITY PASS: Critical 0 / Important 0 / Minor 0.** The admitted body-canvas branch follows the stated CSS rule under conservative restrictions. The amendment also explicitly closes the three prior implementation obligations; their code fixes still need review.

## Exact snapshot

Reviewed `docs/superpowers/plans/2026-10-04-theme-background-proof-v4.md`, SHA-256 `c61c921567f0d6e12f7760c6e0585bd7c529f51cb312585a7d00f08bf512a997`, and its delta against preserved v3. Scope remains the existing two test files, with no product palette/style, dependency, configuration or screenshot-baseline change.

## Canvas contract

The primary CSS specification states that, for an HTML root with no background image and transparent background color, the first HTML body child's background properties propagate to the canvas. The body then does not independently paint that propagated background; containment disables this special handling. The canvas itself extends beyond the root box. These rules support the proposed branch. [CSS Backgrounds 3, sections 2.11.1–2.11.2](https://www.w3.org/TR/css-backgrounds-3/#body-background).

Plan line 19 deliberately narrows acceptance to an opaque solid provider, root/body opacity 1, ordinary visible block display, no containment, no images and no unsupported effects/transformations. Therefore it does not assume a browser's default canvas or equate any visible body rectangle with whole-canvas coverage. Rejecting partial-alpha, hidden/transformed, contained or image-backed cases remains conservative. The implementation must identify the actual first HTML body child, enforce the complete predicate, and record the selected provider.

Do not count propagated paint as a second independent opacity-bearing source. Under the admitted opacity-1 solid branch, repeating the identical RGB as a possible fill cannot increase the bound beyond that initial provider, but the ledger should still identify its propagation role accurately. Canvas-positive and rejection fixtures are needed before real-route acceptance. Effective containment/effects must be rejected rather than relying on an ordinary display value alone.

## Other clarified obligations

Line 9 restricts the checked-switch exception to matches on the control itself and keeps descendants/overflowing descendants in inventory. Line 25 explicitly rejects non-normal background-blend-mode as well as mix-blend-mode. Line 23 rejects unsupported first-line/first-letter background paint without using generated-content absence as a shortcut, and rejects active native modal/popover top-layer scenes. That also avoids incorrectly applying ordinary ancestor-opacity assumptions to top-layer paint. Before/after pseudo support remains conservative and unchanged.

The component-wise bound, complete-main coverage, positive-association opacity argument, upward numeric error budget, caps and unchanged exact text/focus contracts remain in force. No acceptance threshold is relaxed to accommodate the canvas observation.

## Evidence and limits

Reviewer actions: read-only plan/diff inspection and SHA-256 pinning; opened the cited W3C primary specification and checked its relevant normative text; inspected retained RED log. No browser or paint experiment was run by this reviewer.

`/private/tmp/meetily-bg-diag/review-red.log`, SHA-256 `36908df5e858b760b0b9af551ea14ab799ecd39e59f9dfa68900c391a745e996`, records six failures for background blending, two switch-descendant cases, first-line, first-letter and active backdrop. This preserves meaningful RED evidence for the prior source, not a PASS for an implementation of v4. The actual root/body inventory was reported by the owner; this review establishes the conditional rule rather than independently re-observing that runtime scene.

`changed_paths`: this report only. `findings: []`. `blockers: []` for this bounded planning gate. `next_action`: implement the guarded canvas branch and defect corrections, run the required fixtures, then review exact implementation hashes and completed three-route evidence. All wider theme/native/release gates remain open; earlier FAIL reports stay preserved.
