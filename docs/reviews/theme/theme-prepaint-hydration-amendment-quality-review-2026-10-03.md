# Theme hydration adapter amendment — independent QUALITY review

Source-authored by `/root/plans_quality`, 2026-10-03.

**QUALITY PASS for the bounded experiment plan: Critical 0 / Important 0 / Minor 0.** The amendment provides a concrete next hypothesis without waiving the failed hydration requirement. The existing production implementation remains rejected; this is not code or runtime acceptance.

## Exact snapshot

Plan `docs/superpowers/plans/2026-10-02-semantic-theme.md`, SHA-256 `cb02d9b92fb27b7e0ed6a066ba8fa7d966644b4f5208e8afb7dc2203f413bd5f`, pinned before/after. Reviewed the hydration-compatible follow-up at lines 59-83 with the preceding amendment. Task root `.worktrees/semantic-theme-2026-10-03`, base `53f4415058fb01e38b8892a47ac036bfffa394ba`.

The failed production bytes remain unchanged: layout SHA-256 `dac7ec01049f481d79ae32127c80578f4438defc3f18ecf7ad5b83001bcc7723`, ThemeScript SHA-256 `6cce905b099bfb26e40ce65564b79b042454ee464b6197dbce83f1611aa10601`. Previous failure evidence/report is not overwritten.

## Assessment

The earlier experiment demonstrated correct early ordering but mismatched full head __html. The proposed adapter restricts raw head markup to server rendering and supplies a normal script child during client hydration, with the same source-owned text and stable ID. Only one mutually exclusive branch renders per environment; two exports need not create two resolvers. No stored or user-controlled data may enter markup, and both exports must derive from the same constant IIFE to prevent drift.

Read installed Next 14.2.35's bundled React development renderer. Its `resolveSingletonInstance` reuses document.head for the head host singleton; `canHydrateInstance` has root/singleton matching for script nodes and distinguishes resource scripts. The server headChunks path inspected in the prior review still supports the early serialization. These mechanisms make the hypothesis technically concrete, but the inline-script matching heuristic does not itself guarantee correct identity or absence of hydration fallback. The browser requirements remain essential; the presence of a typeof-window branch alone is neither proof of success nor a reason to declare this bounded experiment impossible.

The amended criteria explicitly reject duplicate script emission, reexecution on hydration, head/resource replacement and mismatches. No new suppression or diagnostic filtering is authorized. Existing twelve transitions, held-CSS root state, first-frame checkpoint and later-state assertions remain mandatory. Source checks must verify identical component/markup ID and text, one source IIFE, one component occurrence and mutually exclusive branches.

Implementation verification must make the added failure criteria observable. Count ID-bearing scripts in the live DOM as well as actual response HTML; retain the original node through hydration and record execution separately from ConfigContext's legitimate runtime toggle. A no-op repeated resolver can leave theme state correct, so final state alone cannot establish no reexecution. Observe required metadata/stylesheet/resource preservation across hydration rather than assuming empty console diagnostics proves it. These are concrete ways to exercise the amendment's explicit conditions, not a broader task scope.

The existing product-analytics startup mock supports the ThemeScript component name; its artificial environment may take the client branch. Run the actual unchanged startup regression alongside the real browser gates and do not modify its out-of-allowlist mock silently. Retaining the component export now serves the real client adapter rather than speculative compatibility.

`checks`: local Darwin plan/hash inspection and installed renderer source reading; no production edit, browser experiment or reviewer test execution. `changed_paths`: this report only. `blockers: []` for bounded QUALITY plan acceptance. `next_action`: implement the same-two-file adapter after both plan review gates, update the scoped tests to exercise the explicit identity/execution/resource conditions, and independently review exact code/results. Any failing observation rejects the hypothesis without suppression; any extra path/config/dependency change requires another amendment. Full theme/native/release readiness remains unverified.
