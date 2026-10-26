> Public privacy copy: local user paths and contact/provenance fields were anonymized. Historical test results and original hashes do not verify this edited copy. See [publication notes](/docs/development/public-source-snapshot.md).

# macOS CI prerequisite amendment — independent QUALITY review

Source-authored by `/root/plans_quality`, 2026-10-03.

**QUALITY PASS: Critical 0 / Important 0 / Minor 0** for this planning amendment only. No actionable finding remains in the amended claims or their stated boundaries. Implementation admission, action adoption, hosted execution and release readiness are not accepted by this review.

## Observed snapshot

Root `/Users/developer/Desktop/meeting-minutes`; HEAD/main `53f4415058fb01e38b8892a47ac036bfffa394ba`; dev `6f2c4488edc6b9fffd45f8b2dd31bcd6079fe37e`. Existing dirty overlays preserved.

| Reviewed artifact | SHA-256 |
| --- | --- |
| `docs/superpowers/plans/2026-10-02-macos-ci-release.md` | `d3066f3b42e854e6db3ee571159b7203486a4d025fc23a32f0a1db8cb4dc21c5` |
| `.worktrees/integration-backups-2026-10-02/macos-ci-plan-before-external-admission-2026-10-03.md` | `4f40b439579dc453743e3fd15c8d12ee74cce53ae5481364ece5b5b2255ce373` |
| `docs/fork/evidence/macos-ci-action-pins-2026-10-03.json` | `dc7928868455a532ec6bad7b8b4b927ee2ddf8e23a8af624ef669bd11145a000` |
| `docs/fork/evidence/macos-ci-actionlint-2026-10-03.json` | `74774debe84dcf915095bfec58f6013cbe7a15eb154773a7bfe0b8be21634707` |
| `docs/fork/evidence/macos-ci-prerequisites-2026-10-03.json` | `82573d474ce88cf6bab02fe624ee73a1abfe03014a4318bba4a9e43b001ed359` |

All pins checked before/after. `diff -u` against the preserved plan shows only the bounded external-observation section and replacement of the larger-runner entitlement assumption. Product/tool versions, execution gates and workflow authority are unchanged by the amendment. All three added relative evidence links resolve to the intended local files.

## Claims and supply-chain boundaries

Plan lines 48-56 and 86 correctly separate documented labels from observed host execution. Independently opened [GitHub's standard-runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners): public and private tables both list macos-15-intel as Intel and macos-15 as arm64. The [Intel inventory](https://raw.githubusercontent.com/actions/runner-images/main/images/macos/macos-15-Readme.md) and [arm64 inventory](https://raw.githubusercontent.com/actions/runner-images/main/images/macos/macos-15-arm64-Readme.md) list Xcode 16.4 build 16F6 at `/Applications/Xcode_16.4.app`. They also corroborate the reported current Node/Rust differences. These moving documentation pages are dated observations, not proof of repository capacity or a future image. Actual dual-host runtime assertions remain required.

Plan lines 59-61 label the eight action resolutions as candidates only. Read `/private/tmp/meetily-ci-public-pins.py`: it queries public commit refs, then reads action.yml at the returned commit and hashes those bytes; it neither reviews action implementation nor installs/adopts pins. All eight rows are resolved with adoption=false. The report's manifest runtime lines include different Node runtimes; they do not establish compatibility with the proposed execution environment. No reviewer re-resolution of all eight refs or full source audit occurred. Full action review, transitive execution behavior and runner/toolchain compatibility remain explicit admission gates.

Read `/private/tmp/meetily-actionlint-artifact-audit.py`: fresh-directory creation fails if the directory exists, archives are downloaded and hashed, and no extraction/execution is performed. Independently hashed both retained files in `/private/tmp/meetily-actionlint-audit-2026-10-03`: arm64 is 2164202 bytes, SHA-256 `aba9ced2dee8d27fecca3dc7feb1a7f9a52caefa1eb46f3271ea66b6e0e6953f`; amd64 is 2355828 bytes, SHA-256 `5b44c3bc2255115c9b69e30efc0fecdf498fdb63c5d58e17084fd5f16324c644`. Both equal the plan, saved report and recorded API digest. This independently verifies retained bytes, not the original download process or API response provenance. No repeated download or binary execution was needed. There is no workflow lint PASS claim.

`frontend/scripts/verify-universal-ffmpeg.js:9-20` and `frontend/scripts/universal-onnx-runtime.js:16-27` contain the four recorded names/targets/hashes without producer URL, size or license metadata. The provisioning path copies from a local source/store; `docs/testing.md:229-235` documents pinned local-store provisioning rather than admitted reproducible remote acquisition. Therefore plan lines 67-73 correctly retain missing provenance/fresh-acquisition blockers. A known local checksum does not establish an immutable authenticated producer, and copying the shared seed cannot satisfy fresh-run evidence. Other declared model URLs/licenses do not imply admission or a successful fresh acquisition either.

## Checks and remaining gates

Reviewer checks: local Darwin read-only diff, SHA/size comparison, source/operator-guide and collection-script inspection, relative-link resolution, current primary-document reads and ref readback. No execution of either collector, actionlint, product build or workflow dispatch; no source/Git/status edits or Graphify use.

`changed_paths`: `docs/fork/macos-ci-prerequisite-quality-review-2026-10-03.md` only. `blockers`: clean dev/feature route; exact toolchain compatibility; independent full action review/adoption; immutable native/model producers and fresh acquisition with missing/corrupt/replayed fail-before-build evidence; actual repository scheduling and execution on both architectures. Signing, notarization, artifact/release gates remain separate and NOT VERIFIED.

`next_action`: retain this narrow planning acceptance and resolve the open admission prerequisites before product/workflow implementation. Do not close the task, adopt candidate pins, substitute native bytes, or infer dispatch/publication authorization from this report.
