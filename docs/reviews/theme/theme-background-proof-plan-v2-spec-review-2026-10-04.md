# Background absence plan v2 — independent SPEC review

**PASS for this bounded plan amendment. C0 / I0 / M0.** No actionable findings remain in the reviewed proposal. This does not approve an implementation or complete the theme task.

## Snapshot

Reviewed 2026-10-04. Plan `docs/superpowers/plans/2026-10-04-theme-background-proof-v2.md` SHA-256 before/after: `1a8cae8e3f81786a5100ec87ca809e7c25036e4d691223c722db89fdba9e2476`.

Accepted semantic plan remains `83e82c5891c7d699485ab0910936a67bf4a8e6690e42eb2b6ec43df07767de41`. Observed feature baseline: helper `009af7c626199e6e94d56b2565d721f6c4823e12adb36581ab2f2e0ed2652b79`, dark spec `84fa37ba59d71c8a483295266e46547444fe294dc072c24ca90880b4fdd468be`. Product/test edits remain confined to those two admitted test paths; this review changed only this report.

## Findings and acceptance reasoning

Previous I1 is closed: line 19 explicitly inventories every DOM source, including zero-area and off-main owners, and examines shadow/pseudo paint before principal-fill area/intersection filtering. It preserves visible descendants of visibility-hidden ancestors. Line 37 requires zero-size spread-shadow, positioned-pseudo, off-main shadow and visibility-override regressions. Thus the previously unsafe filtering interpretation is no longer allowed.

Line 31 now supplies the relevant mathematical argument for shared group opacity. Inactive-layer events under independent primitive alpha/opacity gates are decreasing and positively associated; their intersection probability is at least the product of marginal inactivity probabilities. Replacing shared gates with independent effective-alpha gates therefore overestimates each threshold's probability of an active color, hence its maximum-color expectation. Ascending source-color composition calculates that independent maximum expectation above the opaque canvas. Actual normal source-over color is bounded by the maximum active source. This supports nested groups and external siblings under the specified model. Implementation review must still verify faithful mapping, upward numerical margin, caps and exact-compositing fixtures; the proof does not license unsupported paint effects.

Line 23's all-zero ordinary outer-shadow case is a narrowly defined non-painting case, not another color exception. An outer shadow uses the border-box shape and is clipped inside that same border box; with zero offset/spread/blur there is no external shadow area. This follows [CSS Backgrounds and Borders Level 3, §6.1.1](https://www.w3.org/TR/css-backgrounds-3/#shadow-shape). The plan correctly requires a transparent-background regression and forbids extending this rule to inset or nonzero geometry.

The complete-main upper bound and conservative failure rule preserve the no-near-white-background objective. Checked-switch accounting stays explicit, with white shadows still checked. Exact text/focus measurement, overflow checks, three actual routes and all separate acceptance gates remain required.

## Checks, limits and next action

Reviewer checks: complete v2 read, comparison to the previous I1 and current source/canonical-plan contracts, local SHA-256 recheck, and read-only primary CSS specification lookup. No browser, implementation, native or release execution was performed for this plan review. Source baseline hashes remain unchanged. Existing unresolved background results are not converted to PASS by this report.

Blockers for bounded implementation: none. Next action: implement only the two admitted test paths, establish all required positive/negative fixtures and actual-route ledgers, then request exact-source independent SPEC/QUALITY review. Full-theme, snapshots, native/macOS and release readiness remain unverified/open.
