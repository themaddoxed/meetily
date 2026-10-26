# FORK-035 macOS-only CI and release plan — revision 2

> Execute through canonical FORK-143 → FORK-144 → FORK-145–147. This revision is a planning deliverable awaiting independent FORK-144 review.

**Goal:** replace the mixed-platform CI/release surface with a macOS-only pipeline that verifies both Universal slices and the macOS 14.2 deployment floor, produces only `.app` and `.dmg`, preserves Intel CPU and arm64 Metal behavior, and keeps the updater absent.

**Architecture:** CI and distribution remain adapters around the existing Universal build and candidate-verification modules; they do not become product runtime or a second release authority. One deep bundle-contract module verifies every embedded Mach-O, while one artifact-transport module binds each cross-job handoff to an exact artifact ID, archive digest, closed member manifest, source/input fingerprints, and filesystem metadata. Manual distribution is a dependency chain of secretless build, protected signing, protected notarization, secretless verification, and protected draft jobs; no job receives credentials for another phase.

**Tech stack:** GitHub Actions YAML, pinned GitHub Actions, `actionlint` 1.7.12, GitHub-hosted macOS 15 arm64 and Intel runners, pinned Xcode 16.4, Tauri 2 Universal `universal-apple-darwin`, Rust 1.95.0/Cargo with `--locked`, Node 22.17.0, pnpm 10.17.1, `js-yaml`, BSD pax tar, Mach-O `lipo`/`vtool`, Apple `codesign`/`notarytool`/`stapler`, existing release candidate and smoke-report contracts. Task 0 must re-admit these exact tool versions against the future base rather than silently rolling them forward.

## Current planning authority — revision 2, 2026-10-02

This FORK-143 amendment is bound to local main
`53f4415058fb01e38b8892a47ac036bfffa394ba`, tree
`bb8b8bdda6b53deaa926362f59b661410479ef78`, parents
`952e3614fe271118d0783dbc0e84ef10d7d61650` and
`61e890f444f81718d8136beae6afc6f61a5f0e44`.
The historical reviewed plan is preserved unchanged in the foundation checkout,
SHA-256 `a1d57ae7e1e1c418e4c90d9495af402c1f078a44606eb0e8b44c8e6cb6db2637`. Its design and supply-chain boundaries
are retained below. Its old execution base and STOP on FORK-034/FORK-057 are
superseded: both tasks are canonically closed and their product work is in main.

FORK-143 updates the plan only. FORK-144 must independently review this exact
revision before FORK-061 closes or FORK-145 edits workflows. The current owner
approval covered the preceding integration commit, not new product commits,
GitHub dispatch, signing, notarization, secrets or draft creation. No such action
is performed or authorized by this document.

Future implementation starts in a separately selected clean product checkout
at the admitted current product commit. Do not return to historical
`fork/macos-integration` or repoint dev/main automatically. The canonical PRD
is now `docs/prd` in the current product repository, not the preservation
foundation checkout. The older single-product-commit and staged-review boundary
is retained, subject to explicit commit/integration authorization.

Still required before FORK-145: fresh independent plan acceptance, authenticated
immutable native-input acquisition from an empty store, validated action SHA
pins, and runner/toolchain admission. The original Node 22.17.0, Rust 1.95.0,
Xcode 16.4, macOS-15 runner labels and actionlint 1.7.12 below are proposed
historical pins, not current hosted availability or an admitted execution
configuration. Locally observed on this audit: Node 26.10.0, pnpm 10.17.1,
Rust 1.98.1. No exact repository Node/Rust toolchain file exists; CI setup
versions disagree across existing workflows. Reconcile and review explicit
versions before implementation; do not silently upgrade or copy local defaults.

## External prerequisite observation — 3 October 2026

[The bounded prerequisite report](../../fork/evidence/macos-ci-prerequisites-2026-10-03.json)
records read-only public-source checks. GitHub documents `macos-15` as arm64 and
`macos-15-intel` as a standard x64 runner for public/private repositories; both
image inventories list Xcode 16.4 build 16F6 at the proposed explicit path.
See [runner labels](https://raw.githubusercontent.com/actions/runner-images/main/README.md),
[standard-runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners),
and the [Intel](https://github.com/actions/runner-images/blob/main/images/macos/macos-15-Readme.md)/[arm64](https://github.com/actions/runner-images/blob/main/images/macos/macos-15-arm64-Readme.md)
image inventories. Documentation does not prove that this repository scheduled
or ran either host. The bundled Rust/Node versions differ from the historical
proposal; no implicit adoption or version upgrade is made here.

Eight existing action major/ref lines have candidate immutable commit and
`action.yml` hashes in [the pin report](../../fork/evidence/macos-ci-action-pins-2026-10-03.json).
This is ref/manifest resolution, not independent source review or pin adoption.
Both actionlint 1.7.12 Darwin archives were downloaded to an empty temporary
store and matched the two proposed SHA-256 values; [the archive report](../../fork/evidence/macos-ci-actionlint-2026-10-03.json)
records asset IDs, sizes and hashes. They were not extracted or executed, so no
workflow lint PASS follows.

The four FFmpeg/ONNX entries in current manifests and operator documentation
lack admitted immutable producer/download URLs, sizes and license provenance.
The shared local seed is preserved but cannot satisfy fresh-run acquisition.
Do not substitute different bytes, fabricate provenance, publish the store or
relax Task 0. Fresh native/model acquisition and missing/corrupt/replayed
fail-before-build checks, exact toolchain compatibility, full action source
review, repository capacity, and clean dev/feature admission remain open.
This amendment changes planning evidence only and requires independent review
before implementation; it grants no workflow dispatch or publication authority.

## Verified current inventory and decisions

- The exact PRD requires Universal `x86_64` + `arm64`, minimum macOS 14.2, and only `.app`/`.dmg`. ADR 0001 and the accepted architecture contract require every mandatory Alpha function to retain Intel CPU execution while arm64 acceleration remains optional and falls back to CPU.
- FORK-033 already routes `llama-helper` x86_64 with `--no-default-features` and arm64 with `--features metal`, and Whisper has Intel CPU plus arm64 Metal routing/fallback. CI must exercise both target builds and must not add Metal/CoreML to the Intel target.
- FORK-056 removed updater dependencies, endpoints, keys, manifest/archive/signature generation, and updater workflow variables. Apple application signing is allowed; `TAURI_SIGNING_PRIVATE_KEY`, updater `.sig`, `latest.json`, `.app.tar.gz`, updater endpoints, or `createUpdaterArtifacts` remain forbidden until FORK-036.
- Ten workflow YAML files are tracked. Seven are obsolete standalone/reusable mixed-platform workflows; three retained entry points are `ci.yml`, `quality-gate.yml`, and `release.yml`. Current mixed workflows mention Windows/Linux runners, installers, Vulkan/OpenBLAS, removed Windows scripts, updater-era controls, and writable contents permissions.
- Current `quality-gate.yml` uses Ubuntu for source/frontend/fan-in, host-only helper builds, and a thin host `.app`; it neither constructs the reviewed Universal sidecars nor proves recursive Mach-O slices/minimum OS. Current `release.yml` creates a draft before builds and then builds arm64 macOS plus Windows, so it cannot qualify the required artifact.
- Both current Tauri configs already declare `minimumSystemVersion: "14.2"` and app/dmg bundles. Cargo deployment flags still live only in `frontend/src-tauri/.cargo/config.toml`. Workspace-root Cargo, helper Cargo, and Tauri children launched from higher roots do not consistently discover that member-local file; the Universal builder also sets `RUSTFLAGS` for path remaps. The implementation must move one authoritative config to repository-root `.cargo/config.toml`, delete the member-local duplicate, preserve inherited flags, append and validate the 14.2 linker floor, and preserve the existing Tauri minimum without reimplementing it.
- Current Universal structural checks cover the main executable, `llama-helper`, and FFmpeg only. The accepted contract requires a recursive inventory of all Mach-O files under the app, including frameworks, dylibs, plugins, helpers, and nested executables. Every Mach-O must have exactly `arm64` and `x86_64`; first-party app/helper slices must report `minos = 14.2`, while reviewed third-party slices may report `minos <= 14.2`; missing or unparsable build-version evidence fails closed.
- The historical planning lookup listed `macos-15` as arm64 and `macos-15-intel` as x64; macOS 15 images default to Xcode 16.4. The proposed workflows would pin those labels and explicitly select `/Applications/Xcode_16.4.app/Contents/Developer`, but every job must assert `uname -m`, `xcodebuild -version`, installed SDK, and both Rust targets because labels are configuration, not execution proof. Task 0 must verify current runner-image documentation and actual runner availability for this repository. Current GitHub documentation lists `macos-15-intel` as a standard Intel runner; a larger-runner entitlement is not a prerequisite for that label. Actual scheduling/host evidence for both architectures remains mandatory, and unavailable Intel capacity blocks the dual-host gate rather than silently dropping it.
- Tauri 2 documentation supports `tauri build --target universal-apple-darwin --bundles app` and `app,dmg`, plus `bundle.macOS.minimumSystemVersion`. Apple distribution evidence requires Developer ID signing, notarization submission success, stapling and validation; ordinary build success does not prove any of them.
- GitHub Actions documentation recommends least-privilege token permissions, full-SHA action pins, protected environments for deployment secrets, and concurrency. Repository settings, secret presence, required reviewers, branch protection, actual hosted runner architecture, signing identity, and Apple notarization cannot be proven from YAML or local tests.

## Current workflow, caller and artifact census

Local source inspection only; no hosted run was started.

| File | Trigger | Current caller/runner surface | Disposition |
| --- | --- | --- | --- |
| build-devtest.yml | workflow_dispatch | ${{ matrix.platform }} | Remove after callers migrate |
| build-linux.yml | workflow_dispatch | ${{ github.event.inputs.ubuntu-version || 'ubuntu-22.04' }} | Remove after callers migrate |
| build-macos.yml | workflow_dispatch | macos-latest | Remove after callers migrate |
| build-test.yml | workflow_dispatch | ./.github/workflows/build.yml; macos-latest, ubuntu-22.04, ubuntu-24.04, windows-latest | Remove after callers migrate |
| build-windows.yml | workflow_dispatch | windows-latest | Remove after callers migrate |
| build.yml | workflow_call | ${{ inputs.platform }} | Remove after callers migrate |
| ci.yml | pull_request, push | ./.github/workflows/quality-gate.yml | Retain and rewrite |
| pr-main-check.yml | workflow_dispatch | ubuntu-latest | Remove after callers migrate |
| quality-gate.yml | workflow_call, workflow_dispatch | ubuntu-latest; macos-14 | Retain and rewrite |
| release.yml | workflow_dispatch | ./.github/workflows/quality-gate.yml; ./.github/workflows/build.yml; ubuntu-latest; macos-latest, windows-latest | Retain and rewrite |

The exact census is [the adjacent JSON snapshot](2026-10-02-macos-ci-census.json),
with 10 workflow hashes, parsed jobs/needs/matrices/actions and declared
artifact names. ci calls quality-gate; build-test and release call build.yml.
release creates its draft before build-all-platforms and uses secrets inheritance;
manual-attestation checks text/boolean/SHA but does not bind a verified candidate
report/artifact. quality-gate still uses Ubuntu plus a host-only ad-hoc app.
Retain its search browser cases and SQLite/data-integrity suites when rewriting.

Existing adapters to reuse: scripts/check.sh, scripts/release-candidate.sh,
scripts/release-gate.sh, scripts/test-plan-gate.sh, release-policy.mjs,
verify-release-candidate.mjs and build-universal-macos.js. The Universal builder
verifies three declared executables; it is not yet a recursive all-Mach-O gate.
Its helper Cargo call lacks --locked. Its native inputs use the shared ignored
store through provision-universal-artifacts.js, universal-onnx-runtime.js,
verify-universal-ffmpeg.js and diarization-artifacts.js; hash-verified local
copying is already implemented, immutable CI acquisition remains a prerequisite.

The original 32-path product allowlist is retained below. Existing Tauri minimum
fields are verified as present; preserve them. New input acquisition or runner
configuration paths require an explicit reviewed allowlist amendment. No
lockfile/runtime/audio/model/identity expansion is implied by this plan.

## Contract boundaries

- Retained workflow set is exactly `.github/workflows/ci.yml`, `.github/workflows/quality-gate.yml`, and `.github/workflows/release.yml`. All jobs use macOS runners; no `ubuntu-*`, `windows-*`, `linux`, Windows/Linux target, installer, or acceleration branch remains in active workflow YAML.
- `ci.yml` runs on pull requests and pushes to the admitted `dev`/`main` product lanes, has `contents: read`, and calls only the reusable quality gate. It never receives secrets or writes repository state.
- `quality-gate.yml` uses a native-host matrix with exact `{runner: macos-15, arch: arm64}` and `{runner: macos-15-intel, arch: x86_64}` for format/clippy/tests and FORK-033 routing. A separate arm64 job cross-builds one Universal inspection candidate from authenticated native inputs. Every job selects Xcode 16.4 and validates the actual host. The final fan-in also runs on macOS and fails unless every required job succeeded.
- Source checkout is always the workflow's exact `github.sha`; distribution requires full 40-hex `release-commit == github.sha` and an explicit reviewed `release-tag` matching exactly `^v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$`. The tag must also equal the exact repository version mapping frozen in Task 0. No workflow computes/increments a tag, builds another ref, or accepts an existing tag/release/draft.
- Remote actions are pinned to reviewed full commit SHAs with an adjacent version comment; local reusable workflows need no remote pin. Default and ordinary-job permissions are `contents: read`; only the final draft-creation job gets `contents: write` after all candidate gates. No `secrets: inherit` crosses a reusable workflow. `actionlint` is pinned to 1.7.12: Darwin arm64 archive SHA-256 `aba9ced2dee8d27fecca3dc7feb1a7f9a52caefa1eb46f3271ea66b6e0e6953f`, Darwin amd64 archive SHA-256 `5b44c3bc2255115c9b69e30efc0fecdf498fdb63c5d58e17084fd5f16324c644`; acquisition fails closed before extraction.
- Universal build consumes only the authenticated input manifest frozen in Task 0, validates before copying/building, remains offline during native linking, and records input SHA-256 values in candidate evidence. A cache can optimize downloads only after the immutable source and hashes are known; cache miss/corruption must re-acquire and revalidate from that source.
- Repository-root `.cargo/config.toml` is the sole Cargo deployment config. It forces `MACOSX_DEPLOYMENT_TARGET=14.2` and `CMAKE_OSX_DEPLOYMENT_TARGET=14.2` and defines both Darwin target linker floors; the member-local config is deleted. `deploymentEnvironment(baseEnv)` preserves every inherited `RUSTFLAGS`, `CFLAGS`, and `CXXFLAGS` token, rejects any conflicting deployment-minimum token, appends the exact 14.2 linker/native floors plus path remaps, and passes the result to every helper and Tauri child. Both Tauri configs declare `minimumSystemVersion: "14.2"`, bundle only `app`/`dmg`, and omit updater artifacts. `vtool` output, not configuration text, is acceptance evidence.
- `rust-toolchain.toml` freezes the admitted channel (historical proposal `1.95.0`), profile `minimal`, components `rustfmt` and `clippy`, and targets `aarch64-apple-darwin` and `x86_64-apple-darwin`. Every direct Cargo command contains `--locked`. The Universal builder's direct helper call does too; its Tauri call uses a private temporary Cargo runner which receives Tauri's Cargo argv, inserts `--locked` immediately after the Cargo subcommand, and delegates without rewriting later arguments. Tests reject `stable`, missing components/targets, a direct Cargo call without `--locked`, and a Tauri call without the locked runner.
- The bundle-contract module recursively walks regular files without following symlinks outside the app, rejects symlink escape and unreadable entries, detects Mach-O/fat files structurally, and inspects every slice. Exact duplicates, missing slices, extra unsupported CPU slices, malformed FAT tables, missing `LC_BUILD_VERSION`/`minos`, first-party `minos != 14.2`, third-party `minos > 14.2`, and unexpected executables fail closed. Its sorted JSON inventory contains relative path, ownership class, architectures, per-slice minos, size, and SHA-256.
- `build-universal-macos.js` keeps x86_64 helper without Metal and arm64 helper with Metal, but exposes two narrow modes: `buildUniversalApp({ noSign, cargoRunner })` produces and inspects only the Universal `.app`; `createDmgFromApp({ appPath, expectedAppFingerprint, dmgPath })` accepts only an explicitly named already-qualified app and rejects any fingerprint drift. Ordinary inspection CI may compose app then DMG. Distribution runs Tauri app-only with `--no-sign`, scrubs every Tauri-recognized Apple signing/notary variable from the child environment, externally signs/notarizes/staples the app, then creates the only promotion-eligible DMG from that exact post-staple app. Any pre-staple DMG is ineligible by contract.
- Ordinary CI outputs an ad-hoc/inspection `.app` and `.dmg` plus structural report. This is `BUILD_EVIDENCE`, never release readiness. It does not claim Developer ID, Gatekeeper, notarization, stapling, physical Intel/Apple Silicon runtime, macOS 14.2 runtime, or offline packaged behavior.
- `release.yml` is manual only and separates phases by jobs and immutable transports: secretless app build; app signing in protected `macos-signing`; app notarization/stapling in protected `macos-notarization`; secretless DMG creation; DMG signing in `macos-signing`; DMG notarization/stapling in `macos-notarization`; a distinct secretless `verify-candidate`; final draft in protected `macos-draft`. `verify-candidate` has no environment, no secret reference/inheritance, `contents: read`, consumes only the exact final notarization artifact ID/digest, revalidates the closed transport, runs the candidate verifier, and emits the only PASS-bound artifact eligible for draft. Candidate verification inside a signing/notarization job is forbidden.
- App Store Connect material is created only immediately before each explicit `xcrun notarytool submit` and is removed in `if: always()` cleanup. Certificate, API key, zip, keychain, extracted handoff, and transient runner live only under `$RUNNER_TEMP`; cleanup verifies their absence. Commands receive secret paths through step-local environment, never secret contents in argv; shell tracing is disabled; logs/reports/artifacts cannot contain secret values. Apple ID/password aliases and updater signing variables remain forbidden.
- Every cross-job handoff uploads exactly `candidate.tar`, `candidate-manifest.json`, and `candidate-manifest.sha256`. The pax archive is created with macOS BSD tar from a clean root and extracted with permissions; its canonical manifest binds sorted member paths/types, byte SHA-256, POSIX mode, symlink target, extended-attribute name/value digests, source SHA, native-input manifest SHA, phase, previous artifact ID/digest, bundle inventory digest, and app fingerprint. A round-trip check must prove modes, symlinks and required xattrs unchanged before upload. Each producer exports the same-run `artifact-id` and `artifact-digest`; each consumer downloads by exact ID, compares the declared digest, rejects missing/extra outer and inner members, rehashes the tar/manifest and extracted tree, and emits a new bound manifest/report. Run name, artifact name, cache key, or mutable lookup is never authority.
- Only after secretless `verify-candidate` reports `PASS` may `macos-draft` consume that job's exact artifact ID/digest and verification report. Before any mutation it queries refs/tags and releases and rejects an existing tag, release, or draft. It creates exactly one unpublished draft with explicit `tag_name = release-tag` and `target_commitish = release-commit = github.sha`; before upload it asserts the API response has the requested tag, exact target commit, `draft: true`, a nonempty release ID, and zero assets. It then uploads only the recomputed DMG, re-queries by returned release ID, and requires the exact one-asset set `{DMG filename, size, SHA-256}` with no extra asset. Any mismatch stops without publish/latest/update behavior. Draft creation remains `NOT_RUN` in FORK-035 implementation.
- Workflow policy parses every YAML semantically and fails closed on missing/unparseable files, unknown workflow files, mutable actions, non-macOS runners, unpinned toolchains, forbidden targets/bundles/updater surface, excessive permissions, cross-phase secret references, incomplete artifact binding, draft-before-verification, or publish commands. Exact `actionlint` then validates GitHub Actions schema, expressions, `needs`, contexts, permissions, and action inputs; YAML parsing is not treated as that proof.

## Success criteria

- The active workflow graph is three YAML files and contains only macOS jobs; its tested topology separates read-only CI, authenticated Universal build evidence, secret-bearing distribution, verification, and draft creation.
- Native CI runs on both asserted arm64 and x86_64 hosts, selects Xcode 16.4, compiles/tests the current code, and preserves exact FORK-033 helper/Whisper CPU-versus-Metal behavior.
- The inspection candidate and distribution candidate each contain only `.app` and `.dmg`; every embedded Mach-O has exactly both architectures and acceptable per-slice `minos`, with first-party app/helper exactly 14.2.
- The signed path cannot reach draft creation unless every phase consumes the exact prior artifact ID/digest, revalidates the closed pax transport, and exact source/input provenance, Developer ID signature, hardened runtime, entitlement checks, app+DMG notarization/stapling, Gatekeeper/candidate verification, and checksum-bound reports all pass.
- Updater absence remains green; no updater key, endpoint, manifest, archive, signature, plugin, permission, or build flag returns.
- Local syntax/policy/tests and buildable local gates pass. Unavailable trusted artifact input, Intel runner, repository environment protection, Apple secrets, hosted workflow execution, physical machines, signing/notarization, GitHub draft/publication, or network evidence is recorded `BLOCKED`/`NOT_RUN`, never inferred.
- Product diff is exactly within the allowlist below, one local non-merge product commit is fast-forwarded only after staged SPEC+QUALITY PASS, and PRD closeout remains a separate foundation commit.

## Exact provisional product allowlist: 32 unique paths

Create only:

1. `.cargo/config.toml`
2. `rust-toolchain.toml`
3. `frontend/scripts/macos-bundle-contract.mjs`
4. `frontend/tests/macos-bundle-contract.test.mjs`
5. `frontend/scripts/macos-artifact-transport.mjs`
6. `frontend/tests/macos-artifact-transport.test.mjs`

Modify only:

7. `.github/workflows/ci.yml`
8. `.github/workflows/quality-gate.yml`
9. `.github/workflows/release.yml`
10. `frontend/scripts/release-policy.mjs`
11. `frontend/tests/release-policy.test.mjs`
12. `frontend/tests/workflow-gate.test.mjs`
13. `frontend/scripts/build-universal-macos.js`
14. `frontend/tests/universal-build-config.spec.ts`
15. `frontend/scripts/verify-release-candidate.mjs`
16. `frontend/tests/release-candidate.test.mjs`
17. `scripts/release-candidate.sh`
18. `frontend/src-tauri/tauri.conf.json`
19. `frontend/src-tauri/tauri.universal.conf.json`
20. `docs/testing.md`
21. `docs/development/universal-macos-local-artifacts.md`

Delete only:

22. `frontend/src-tauri/.cargo/config.toml`
23. `.github/workflows/build-devtest.yml`
24. `.github/workflows/build-linux.yml`
25. `.github/workflows/build-macos.yml`
26. `.github/workflows/build-test.yml`
27. `.github/workflows/build-windows.yml`
28. `.github/workflows/build.yml`
29. `.github/workflows/pr-main-check.yml`
30. `.github/workflows/ACCELERATION_GUIDE.md`
31. `.github/workflows/README_DEVTEST.md`
32. `.github/workflows/WORKFLOWS_OVERVIEW.md`

This allowlist is provisional until Task 0 rewrites the exact post-FORK-057 base. No dependency/lockfile, Rust runtime, audio, updater, identity, icon, entitlement, model, or broad documentation path may be added implicitly.

## Canonical execution roots and admission

Planning source and canonical PRD: current repository at main `53f4415`.
The historical foundation plan is a preservation source only. Product checkout,
feature branch and integration destination must be selected explicitly before
FORK-145; their current commit/tree must match the reviewed admission snapshot.
No new worktree or branch is created by FORK-143.

Do not install dependencies during tests/builds. Serialize commands sharing
node_modules, target, out, binaries or resources.

## Task 0: admit the reviewed plan and current external prerequisites

Before product changes, confirm FORK-143/144/061 completion and the canonically
closed foundation dependencies; record exact base/tree/parents and compare the
workflow/config/script census below. The old FORK-034/FORK-057 STOP does not
apply. Any newly changed caller or input requires plan amendment and review.

Acquire each pinned FFmpeg, ONNX Runtime, sherpa-onnx and model input from an
approved immutable authenticated source into a fresh empty store. Record URL or
reproducible same-run source, SHA-256, size, license/provenance and authentication
scope. Prove missing/corrupt/replayed inputs fail before build. The current local
shared store and verified-copy helper are not fresh-run acquisition evidence.

Verify proposed runner labels/Xcode, actual Intel capacity, both target routes,
exact admitted Rust/Node/pnpm/actionlint versions and full remote-action SHA
pins. Reconcile version/config flags before choosing the final allowlist.
The historical actionlint archive hashes below are provenance to revalidate,
not evidence that an archive was downloaded in this amendment.

Freeze the amended plan hash and obtain independent SPEC/QUALITY acceptance.
Local workflow syntax and policy tests are not hosted execution or protected
repository environment proof. Signing/notarization/draft remain NOT_RUN.

## Task 1: write executable workflow and configuration contracts (RED)

**Files:** create the two implementation modules with compile-safe sentinel exports, then modify workflow/policy/config/transport tests before production wiring.

- [ ] First add loadable, unused seams: `inspectMacosBundle()` returns one typed `bundle.not-implemented` finding; `createArtifactTransport()` and `verifyArtifactTransport()` return one typed `transport.not-implemented` finding; `validateMacosWorkflowTopology()` returns one typed `workflow.not-implemented` finding; builder exports `deploymentEnvironment`, `lockedCargoRunnerArgs`, and `createDmgFromApp` with sentinel failures. Import tests must pass before behavioral RED. These seams are not called from production until their GREEN tasks.

- [ ] Add semantic tests named `macOS workflow policy accepts only the reviewed three-file graph` and `macOS workflow policy rejects every forbidden topology mutation`. Fixtures cover non-macOS runner/target/bundle, mutable action ref, write permission outside final draft, secret inheritance, missing architecture/Xcode assertion, draft-before-verification, publish command, updater key/artifact, unparseable YAML, missing file, and extra workflow.
- [ ] Extend `workflow-gate.test.mjs` with exact jobs/dependencies, runner matrix, source SHA equality, authenticated-input gate, recursive report, sign/notary/staple order, and a secretless `verify-candidate` after the immutable notarization handoff. Mutations reject verification in protected jobs, any environment/secret in verify, release input without exact tag/version validation, existing-tag/release/draft acceptance, computed/incremented tags, omitted/different `target_commitish`, upload before API-response binding, non-draft/wrong-tag/wrong-target/empty-ID response, and missing/extra post-upload assets.
- [ ] Add `macos-bundle-contract.test.mjs` fixtures for recursive main/helper/dylib/framework/plugin discovery, symlink escape, unreadable/malformed files, missing/duplicate/extra slices, per-slice `minos`, first-party exact 14.2, third-party <=14.2, >14.2 rejection, stable sorted inventory, and injected command failure.
- [ ] Extend Universal/config/candidate tests for root-only Cargo config, exact toolchain/targets, preserved inherited flags, conflicting/missing/non-14.2 mutation rejection, direct and Tauri-child lock enforcement, app-only then exact-app DMG, recursive reuse, and checksum-bound inventory. Add transport fixtures that mutate bytes, members, mode, symlink, xattr, prior artifact ID/digest, source/input hash, app fingerprint, report hash, and outer extras.
- [ ] Run exact RED commands sequentially from the future product root: `node --test --test-name-pattern='macOS workflow policy' frontend/tests/release-policy.test.mjs`; `node --test frontend/tests/workflow-gate.test.mjs`; `node --test frontend/tests/macos-bundle-contract.test.mjs`; `node --test frontend/tests/macos-artifact-transport.test.mjs`; `pnpm --dir frontend exec playwright test tests/universal-build-config.spec.ts`; `node --test --test-name-pattern='recursive Mach-O|deployment floor|artifact transport' frontend/tests/release-candidate.test.mjs`. Imports pass; assertions fail specifically on the sentinel code or missing target behavior.

## Task 2: implement the deep recursive macOS bundle contract

**Files:** create `macos-bundle-contract.mjs`; modify Universal builder, candidate verifier, and their tests.

- [ ] Implement one small interface: `inspectMacosBundle({ appPath, productExecutable, firstPartyExecutables, run }) -> inventory`. Keep walking/parsing/hashing/ownership and `lipo`/`vtool` adapters private. Tests inject `run`; production uses argument arrays without shell interpolation.
- [ ] Enumerate every in-bundle regular Mach-O without following escaping symlinks. Parse architecture and per-architecture `LC_BUILD_VERSION`/`minos`, require exact slice and deployment rules, hash bytes, and return stable relative-path inventory.
- [ ] Call the same interface from Universal build and release candidate verification. Persist inventory and its SHA-256 in the candidate report; do not add a second verifier or weaker CI-only parser.
- [ ] Preserve helper target features. Move the member Cargo config unchanged in meaning to repository root, delete the duplicate, create exact `rust-toolchain.toml`, implement append-and-validate deployment flags, add `--locked` to helper Cargo, and route Tauri through the tested temporary locked runner. Set both Tauri minimums to 14.2.
- [ ] Implement `buildUniversalApp` and `createDmgFromApp` in the Universal builder here: app-only compilation completes first; DMG creation consumes that exact inspected app and cannot rebuild it. These builder adapters and their Universal/config fixtures belong to Task 2. Task 4 wires their transport/signing orchestration without deferring these adapter implementations. No actual signing or notarization is performed by this adapter task.
- [ ] Run only Task 2-owned GREEN checks: `node --test frontend/tests/macos-bundle-contract.test.mjs`; `pnpm --dir frontend exec playwright test tests/universal-build-config.spec.ts`; `node --test --test-name-pattern='recursive Mach-O|deployment floor' frontend/tests/release-candidate.test.mjs`. Each reports zero failures and nonzero matching tests. Workflow topology and artifact transport retain their Task 1 sentinel/RED results until Tasks 3 and 4; do not count them as Task 2 GREEN.

## Task 3: replace the workflow graph and enforce it semantically

**Files:** retain/modify three YAML files and policy/tests; delete the ten obsolete workflow files/docs.

- [ ] Implement `validateMacosWorkflowTopology` inside existing release policy using parsed YAML facts and an exact tracked-workflow set. Require the secretless build/sign/notary/DMG/sign/notary/verify/draft dependency chain, environment-secret separation, exact artifact-ID/digest handoffs, action/toolchain pins, and lock enforcement. Missing/parser errors are findings.
- [ ] Rewrite quality gate with macOS-only source/frontend/native host matrix/Universal inspection/fan-in. Pin runner labels, Xcode path, Node/pnpm/Rust setup, full action SHAs, timeouts, concurrency and read permissions. Assert actual host/toolchain before build.
- [ ] Rewrite release with the exact phase jobs above. `verify-candidate` alone turns final-notary evidence into a PASS-bound artifact and exposes only report/digests/ID to `macos-draft`. Draft preflights tag/release absence, creates with explicit reviewed tag and exact SHA target, validates response before upload, and validates exact assets afterward. Only `macos-draft` receives contents write; never execute it during FORK-035.
- [ ] Delete the obsolete mixed workflows and their three inaccurate guides. Update the two retained docs to describe the three-entry workflow graph, trusted native-input prerequisite, local commands, and evidence limits.
- [ ] Run Task 3-owned GREEN from canonical product cwd: `node --test --test-name-pattern='macOS workflow policy' frontend/tests/release-policy.test.mjs`; `node --test frontend/tests/workflow-gate.test.mjs` (YAML topology/policy fixtures, not transport execution); `pnpm --dir frontend check:policy`; then run this same local actionlint gate in Task 3, Task 5, after the product commit, and after integration: create `ACTIONLINT_TMP=$(mktemp -d "${TMPDIR:-/tmp}/fork035-actionlint.XXXXXX")`, install `trap 'rm -rf -- "$ACTIONLINT_TMP"' EXIT HUP INT TERM`, require `uname -s = Darwin`, map only `uname -m` `arm64 -> actionlint_1.7.12_darwin_arm64.tar.gz / aba9ced2dee8d27fecca3dc7feb1a7f9a52caefa1eb46f3271ea66b6e0e6953f` or `x86_64 -> actionlint_1.7.12_darwin_amd64.tar.gz / 5b44c3bc2255115c9b69e30efc0fecdf498fdb63c5d58e17084fd5f16324c644`, download that exact 1.7.12 archive into the temp dir, run `shasum -a 256` and compare the entire expected digest before extraction, extract there, resolve/test the absolute `$ACTIONLINT_TMP/actionlint`, and invoke it with exactly `.github/workflows/ci.yml .github/workflows/quality-gate.yml .github/workflows/release.yml`; finally run `sh -n scripts/release-candidate.sh`. `$RUNNER_TEMP` is not used by this local gate.

## Task 4: implement signed/notarized distribution ordering without running it

**Files:** modify `scripts/release-candidate.sh`, release workflow, candidate verifier/tests.

- [ ] Implement and test the closed pax transport. Producers round-trip and hash modes/symlinks/xattrs; consumers require exact outer/inner members and bind previous artifact ID/digest, source/input hashes, bundle inventory, app fingerprint, DMG and reports. Rehash at every job and in every phase report.
- [ ] Distribution app build is `tauri build --bundles app --target universal-apple-darwin --no-sign` with the locked runner and all Apple variables absent. Sign nested code inside-out and app last; explicit notary jobs submit exactly app zip then DMG, require `Accepted`, staple/validate; DMG build accepts only the exact stapled-app fingerprint. Final notarization emits immutable evidence only; the subsequent secretless `verify-candidate` job performs all candidate verification.
- [ ] Do not echo credentials or put secret contents in argv/log/report. Materialize only phase credentials under `$RUNNER_TEMP`, disable tracing, clean in `if: always()`, and assert absence afterward. Shell-fake tests inject failure at every sign/submit/staple/archive/rehash boundary and prove no later artifact or draft becomes eligible.
- [ ] From canonical product cwd run `node --test frontend/tests/release-candidate.test.mjs` and `node --test frontend/tests/smoke-report.test.mjs`. No live certificate, Apple request, workflow dispatch, release, or secret check is permitted in this task.

- [ ] After Task 4 implements transport, run all six exact focused GREEN commands from Task 1. Require zero failures and nonzero matching tests for every command. This is the first combined GREEN gate for bundle, workflow and transport behavior; retain each earlier RED result as diagnostic evidence.

## Task 5: full local verification, evidence report, staged reviews, and one commit

- [ ] Run sequentially from canonical product cwd: `git diff --check`; exact 32-path allowlist/status check; focused Node/Playwright tests including transport and release-tag/API mutations; policy/full frontend/lint/typecheck/build; the exact mktemp/actionlint/trap gate from Task 3; `sh -n scripts/release-candidate.sh`; `cargo metadata --manifest-path Cargo.toml --locked --no-deps`; `cargo fmt --manifest-path Cargo.toml --all -- --check`; `cargo clippy --manifest-path Cargo.toml --workspace --all-targets --locked -- -D warnings`; `cargo test --manifest-path Cargo.toml --workspace --locked`. `cargo fmt` is the sole Cargo invocation without `--locked` because cargo-fmt rejects that flag and performs no dependency resolution; the immediately preceding locked metadata gate protects lockfile state.
- [ ] If the approved native-input source and full Xcode are locally available, seed a fresh empty store and run the Universal inspection build plus recursive report. Otherwise record exact `BLOCKED` and do not claim artifact proof.
- [ ] Record separate evidence: source-policy, YAML topology, native host compile, Universal structure/minos, unsigned inspection artifact, distribution signing, notarization, Gatekeeper, physical Intel, physical Apple Silicon, macOS 14.2 boundary, hosted Actions, protected environment, draft creation, public release. Mark every unrun layer `NOT_RUN`/`BLOCKED`.
- [ ] Stage only the reconciled allowlist, freeze staged diff SHA/count/status, obtain independent staged SPEC and QUALITY PASS, then create exactly one product commit. Verify commit tree/diff byte identity and zero merge commits.

## Task 6: authorized local integration and SDD closeout

Integrate only the independently reviewed exact product commit into the selected
clean destination after explicit authorization. Preserve old refs/worktrees and
verify tree identity. Do not choose or move main/dev/legacy refs implicitly.
Run affected post-commit checks and the required full pre-candidate gate;
repeat a successful check only when source/input drift or an unresolved risk
requires it. Record the real exit status and all skipped/manual layers.

Update the current canonical `docs/prd/macos-fork.md` in both its user-facing
feature description and the completed task. FORK-143 completion does not close
FORK-144, FORK-061 or FORK-035. Documentation and product commits require their
own applicable authorization; no push, dispatch or publication is included.

## Risks and controls

- **Unreproducible CI native inputs:** current local ignored store is not runner provenance. Hard stop until every input has an immutable authenticated source and fresh-store proof.
- **False Universal claim:** checking only three executables misses embedded dylibs/frameworks/plugins. Recursive Mach-O inventory and per-slice minos are mandatory.
- **Runner-label drift or unavailable Intel capacity:** pin macOS 15 labels/Xcode, assert runtime architecture/toolchain, and stop if the repository cannot schedule the Intel runner.
- **Supply-chain action drift:** every remote action uses a freshly verified full SHA; mutable tags are comments only and semantic policy rejects them.
- **Metal regression on Intel:** dual native host jobs plus exact helper/Whisper routing tests preserve CPU-only x86_64 and arm64 Metal/fallback.
- **Release before evidence:** build/sign/notarize/staple/verify precedes the only write-permission draft job; no automatic publish exists. Draft is not release readiness.
- **Secret or updater reintroduction:** named environment-scoped Apple application credentials only, cleanup on every path, existing updater semantic policy retained, no updater key/artifact/endpoint.
- **Evidence inflation:** YAML/local tests do not prove hosted execution, secrets, Developer ID, Apple acceptance, Gatekeeper, physical processors, macOS 14.2 runtime, draft creation, or publication.
