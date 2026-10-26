# Fresh publication authority v2 — bounded independent SPEC review

**I1 resolved at design level. C0 / I0 / M0 in the reviewed fix. Implementation remains NOT_ADMITTED.** No full execution-plan or runtime acceptance is granted.

## Exact snapshot

Reviewed 2026-10-04. Amendment v2 SHA-256 before/after: `37332f101672e60a26d443b0c39d75a1c516be8c5406cd4c85bfaf4970f9783b`. Compared the full delta against preserved original `72d4435d7ae3333667962f2767c5561c8b5ef4c8696801cd90cf8098627ee6e8` and prior SPEC finding `ca339957bbaad1b17a7d47952e0b949d083c2e6d65bddfa645823e11c2190016`. Relevant migration source remains `f249093eaa83e3116aece3f754135afd92e531e97c5a438affac7acba6d9349c`.

## Finding closure

The new section explicitly factors internal copy/hash/no-clobber/readback mechanics behind separately validated authority types. Existing legacy stage/publish/resume retain unchanged validate_manifest and its nonempty different-identity rule. OwnedFreshPublication instead binds the actual target identity, first-launch operation/generation, candidate manifest and target. Thus fresh publication no longer needs a fictional source identity or an equal-identity bypass through the legacy public API.

Native-derived candidate and staging directories are disjoint from one another and from actual target app-data, on the target filesystem. Explicit containment/symlink/alias checks and durable owner binding preserve the current source-root relationship restrictions instead of treating new identity semantics as permission to relax them. The fresh-owner lock mapping remains an explicit source-inspection gate.

Required evidence now includes successful genuine same-identity fresh publication paired with continued rejection of equal-identity legacy journals, plus overlap/alias negatives and existing legacy regressions. This directly covers the previous source-interface mismatch. No code or successful test is claimed by the amendment.

## Checks, limits and next action

Reviewer checks: read-only complete original-to-v2 diff and SHA-256 verification of amendment and unchanged migration source. No app, Git operation, profile access or runtime experiment. Review changed_paths: only this report.

No blocker remains from I1. Full canonical encoding, ownership bootstrap/persistence, fresh SQLx-prefix classification, clean source, actual identity pair and plugin/CSP gates remain open. Freeze and review those before admitting implementation; test the exact eventual shared-mechanics diff rather than inferring correctness from this design acceptance. FORK-123, native migration and release gates are not closed.
