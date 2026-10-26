# Meetily 0.4.1 Alpha READY Qualification Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans, then superpowers:finishing-a-development-branch and superpowers:verification-before-completion.

**Goal:** Build one clean, traceable 0.4.1 Alpha candidate and report READY only when every automatic, product/hardware, architecture, signing, notarization, and Gatekeeper requirement passes for that exact artifact.

**Architecture:** Reuse the repository release gate. Evidence is keyed by exact commit and candidate SHA-256; no result from the older diagnostic DMG is carried forward as proof.

---

## Task 1: Feature-line aggregate verification

- [ ] Confirm `feature/release-ready-ui` contains only the recovery fix, approved specs/plans, and the three implementation slices.
- [ ] Run focused Export, speaker rename/presentation, and diarization runtime tests.
- [ ] Run `./scripts/check.sh --quick`, then `./scripts/check.sh --full`.
- [ ] Run `git diff --check`, `git status --short --branch`, and record exact `git rev-parse HEAD`.
- [ ] Verify the existing `docs: plan release-ready meeting actions` commit is an ancestor; make the source worktree clean before candidate generation.

## Task 2: Universal release candidate

- [ ] Verify the local Universal artifact store contains pinned FFmpeg, ONNX Runtime, helper, diarization native runtime, models, checksums, and licenses.
- [ ] Run `./scripts/release-candidate.sh` in normal mode from the clean exact commit.
- [ ] Verify the generated candidate report is `PASS`, source fingerprint matches, and every executable/native library has required slices.
- [ ] Record DMG path, SHA-256, size, embedded bundle fingerprint, signing identity, and exact commit.

## Task 3: Packaged-app product and hardware smoke

- [ ] Create a new smoke report from `docs/release/smoke-report-template.md` for the exact candidate checksum.
- [ ] On Intel macOS, perform clean launch/permissions, real mic and system audio capture, live transcript, forced-quit recovery, normal finish, reopen, local summary, real diarization, speaker rename/restart, Export/inspect, offline repeat, and old-meeting regression.
- [ ] Exercise missing-model and provider failure paths and confirm persisted transcript/speaker/summary preservation.
- [ ] Repeat the production diarization/runtime architecture smoke on Apple Silicon for the same source and equivalent candidate resources.
- [ ] Attach logs and record every required row as PASS/FAIL; no NOT_RUN row is compatible with READY.

## Task 4: Signing, notarization, and Gatekeeper

- [ ] Resolve an available `Developer ID Application` identity with `security find-identity -v -p codesigning`; do not create or export credentials.
- [ ] Build/sign the exact clean source candidate with hardened runtime and required entitlements.
- [ ] Submit to Apple's notary service using already configured release credentials, wait for Accepted, staple the ticket, and rebuild the final DMG only through the approved release workflow.
- [ ] Verify `codesign --verify --deep --strict --verbose=2`, `spctl --assess --type execute`, `xcrun stapler validate`, and `spctl --assess --type open --context context:primary-signature` on the final app/DMG.
- [ ] Recompute the final artifact SHA-256 and ensure smoke evidence points to that exact final artifact. If signing identity or notary credentials are unavailable, status remains BLOCKED, not READY.

## Task 5: Final gate and branch handoff

- [ ] Set `COMMIT_SHA=$(git rev-parse HEAD)` and run `./scripts/release-gate.sh --smoke-report "artifacts/release-gate/manual/$COMMIT_SHA/smoke-report.md"` against the exact final candidate evidence.
- [ ] Require all automatic and manual rows PASS and `releaseEligible=true`.
- [ ] Use `finishing-a-development-branch` to present local merge/PR choices; do not push, merge, create a release, or modify GitHub without explicit user authorization.
- [ ] Final report names the branch, commit, candidate path, SHA-256, automatic count, manual count, architecture evidence, signature/notary/Gatekeeper state, and any blocker.
