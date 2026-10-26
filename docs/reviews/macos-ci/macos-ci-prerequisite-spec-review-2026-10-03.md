# FORK-145 prerequisite amendment: independent SPEC review

2026-10-03. **PASS, Critical 0 / Important 0 / Minor 0.** No actionable specification defect found in the bounded amendment. This accepts planning observations only; Task 0 implementation admission remains incomplete.

Scope: exact plan diff against the preserved prior plan, its three prerequisite evidence files, applicable Task 0/PRD contract and relevant public runner documentation. Observed root HEAD is `53f4415058fb01e38b8892a47ac036bfffa394ba`; pre-existing dirty overlays and pending route remain outside approval. The diff adds the dated external observation and corrects Intel runner entitlement wording; no version, allowlist, workflow, or gate change is introduced.

## Checks and evidence

Reviewer used `git diff --no-index` to read the complete two-hunk change, scoped `sed`/`rg` for Task 0 and FORK-145, `git rev-parse HEAD`, and `shasum -a 256`/Python hashing. Plan and three evidence hashes match the supplied snapshot and remained unchanged on final recheck:

- `docs/superpowers/plans/2026-10-02-macos-ci-release.md`: `d3066f3b42e854e6db3ee571159b7203486a4d025fc23a32f0a1db8cb4dc21c5`
- `docs/fork/evidence/macos-ci-action-pins-2026-10-03.json`: `dc7928868455a532ec6bad7b8b4b927ee2ddf8e23a8af624ef669bd11145a000`
- `docs/fork/evidence/macos-ci-actionlint-2026-10-03.json`: `74774debe84dcf915095bfec58f6013cbe7a15eb154773a7bfe0b8be21634707`
- `docs/fork/evidence/macos-ci-prerequisites-2026-10-03.json`: `82573d474ce88cf6bab02fe624ee73a1abfe03014a4318bba4a9e43b001ed359`
- `.worktrees/integration-backups-2026-10-02/macos-ci-plan-before-external-admission-2026-10-03.md`: `4f40b439579dc453743e3fd15c8d12ee74cce53ae5481364ece5b5b2255ce373`

Independently read the current [GitHub standard runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners): its public and private tables list `macos-15-intel` as standard Intel and `macos-15` as arm64. Removing a larger-runner entitlement prerequisite is supported; actual repository scheduling/capacity remains unverified. The current [Intel inventory](https://raw.githubusercontent.com/actions/runner-images/main/images/macos/macos-15-Readme.md) and [arm64 inventory](https://raw.githubusercontent.com/actions/runner-images/main/images/macos/macos-15-arm64-Readme.md) both list Xcode 16.4 build 16F6 at `/Applications/Xcode_16.4.app`, and confirm the reported differing Rust versions. These mutable public inventories must be rechecked at execution.

Reviewer rehashed the two existing archives under `/private/tmp/meetily-actionlint-audit-2026-10-03`: arm64 `aba9ced2dee8d27fecca3dc7feb1a7f9a52caefa1eb46f3271ea66b6e0e6953f`; amd64 `5b44c3bc2255115c9b69e30efc0fecdf498fdb63c5d58e17084fd5f16324c644`. Both match plan/evidence. Reviewer did not independently witness original download/empty-store creation, extract archives or execute binaries. Existing archive byte verification is not workflow lint evidence.

The eight action entries explicitly say candidate/not adopted and reviewed_for_adoption=false. Reviewer inspected this classification and recorded manifest hashes, but did not repeat all remote ref resolutions or review executable action source. No action adoption or supply-chain PASS follows. The local artifacts operator guide describes initialization from already-existing files and lists the four hashes; it supplies no immutable producer route. The amendment correctly retains fresh acquisition/provenance as a blocker instead of treating local shared-copy validation as acquisition proof. This bounded review does not re-audit the entire unchanged source census/manifests.

## Contract fit and remaining gates

Plan lines 47–75 preserve the observation/acceptance distinction; line 86 retains architecture, Xcode, SDK and Rust-target assertions and requires actual dual-host scheduling evidence. Task 0 lines 214–234 still requires fresh authenticated native/model inputs, URL or reproducible same-run producer, hashes, size, license/authentication scope, missing/corrupt/replayed fail-before-build evidence, exact toolchain compatibility, full action SHA review and independent amended-plan acceptance. FORK-145 implementation and FORK-146 Universal verification are not marked complete by this amendment.

Blockers remain: agreed clean dev/feature route and exact base admission; immutable approved FFmpeg/ONNX producer/provenance; fresh native/model acquisition and rejection checks; reviewed action implementations; exact installed toolchains and real Intel/arm64 capacity. Hosted execution, native build, signing, notarization, draft creation and release readiness are NOT VERIFIED. Mandatory `docs/testing.md` release gates remain unchanged.

Next action: preserve these blockers, obtain route decision, then complete Task 0 prerequisite admission before product implementation. `changed_paths`: only `docs/fork/macos-ci-prerequisite-spec-review-2026-10-03.md`; no source, status, refs, workflow dispatch or Graphify changes.
