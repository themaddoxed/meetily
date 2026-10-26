# Hidden Diarization Backend Foundation

This record fixes the contract and migration boundary for the backend-only diarization foundation. The implementation follows `docs/superpowers/plans/2026-07-13-hidden-diarization-backend-foundation.md` from immutable clean planning base `a75082bbae99bf9220cd02e36c3108d9f8a108bb`.

## Fixed policies

| Policy | Fixed decision |
|---|---|
| Engine and visibility | This foundation adds no diarization engine or producer. The backend remains hidden, with no visible UI. |
| Settings authority | SQLite database settings are the runtime authority. |
| Apply gate | Apply requires `enabled=true` and a non-`off` persisted mode compatible with the explicit run kind. |
| Pre-mutation rejection | Reject empty requests, cross-meeting segments, malformed segments, nonexistent meetings, and requests with no alignable transcript before any mutation. |
| No implicit clear | An empty request never clears prior diarization data; there is no implicit clear command. |
| Generic overlap | Overlap is represented as `Multiple speakers` with `is_overlap=true` and no participating speaker identities. |
| Explicit cleanup | Meeting deletion and retranscription explicitly remove diarization rows and transcript diarization columns, independently of SQLite foreign-key enforcement. |
| Speaker cardinality | There is no speaker-count cap. |
| Command errors | Webview command errors are stable and sanitized; internal SQL details are logged locally and are never returned. |
| Frontend deferral | Frontend settings, transcript presentation, labels, copy, and rename are deferred to `2026-07-13-hidden-diarization-frontend-handoff.md`. |

## Selected backend source inventory

Only these bounded source slices are selected for the hidden backend migration, in this order.

| Source slice | Disposition | Backend scope |
|---|---|---|
| `aa5e006` | selected | diarization domain types |
| `d7b0c1c` code only | selected | domain constructor API |
| `49c3ab5` | selected | deterministic transcript alignment |
| `c67308e` code only | selected | alignment threshold hardening |
| `a5a699b` semantic slice | selected | persistence models, DTOs, and repository fields |
| `b971a58` test only | selected | import fixture coverage |
| `7222852` | selected | database-authoritative settings repository behavior |
| `4c02489` | selected | validated atomic apply |
| `6fe2a10` backend slice | selected | backend command and application handoff |

No whole source commit is assumed safe merely because one of its slices is selected. Code-only, semantic, test-only, backend-only, and frontend-only boundaries are part of the inventory.

## Excluded and deferred source inventory

These sources and slices are outside this backend wave. The frontend and rename slices remain assigned to `2026-07-13-hidden-diarization-frontend-handoff.md`.

| Source or slice | Disposition | Reason or later scope |
|---|---|---|
| docs-only staging commits | excluded | documentation staging is not backend implementation |
| duplicate theme/Gemma commits | excluded | already migrated through their own registry rows |
| `c0201eb` | excluded | duplicate audio-verification patch |
| `8dceceb` | excluded | merge commit |
| `dea4578` | deferred | frontend settings |
| `29b24fc` | deferred | frontend settings and presentation |
| `28e1bb0` | deferred | frontend transcript presentation |
| `d2db258` | deferred | frontend labels and copy |
| `2576ba8` | deferred | frontend presentation follow-up |
| `6fe2a10` frontend slice | deferred | frontend application handoff |
| `0160d84` | deferred | speaker rename |

## Verified backend checkpoint

The hidden backend-only checkpoint is verified at code tip `90bcbf867c299c72e5d9bca034eb4d8d9014186c`. The combined implementation registry row remains `migrating`: no diarization engine or producer exists, no visible UI is enabled, and the deferred frontend and rename work is not claimed complete.

| Selected source slice | Verified destination |
|---|---|
| `aa5e006` | domain types in `f18ff4d` |
| `d7b0c1c` code only | constructor API in `f18ff4d` |
| `49c3ab5` | deterministic alignment in `f18ff4d`, overlap-precedence correction in `7db0d53` |
| `c67308e` code only | alignment threshold hardening in `f18ff4d` |
| `a5a699b` semantic slice | persistence in `12a1bea`, immutable migration compatibility in `4481511`, sanitized persistence errors in `17f6d9a` |
| `b971a58` test only | import fixture coverage in `12a1bea` |
| `7222852` | typed database settings in `146b64f`, strict persisted-boolean validation in `d1a6043` |
| `4c02489` | validated atomic apply in `27f8c3e`, transactional settings recheck in `90bcbf8` |
| `6fe2a10` backend slice | backend handoff in `27f8c3e`, transactional settings recheck in `90bcbf8` |

Verification evidence at the checkpoint:

- Focused `rustfmt --edition 2021 --check` passed for the changed repository, command, and integration-test files; `git diff --check` passed for the complete wave, including the one-line handler registration.
- `cargo check -p meetily --tests` passed offline with the local Tauri test configuration.
- `cargo test -p meetily` passed: 241 library tests, 11 atomic-apply integration tests, 9 migration/cleanup integration tests, and 2 doctests; 3 hardware-dependent tests were ignored and no test failed.
- `scripts/check-diarization-backend-migration.sh --self-test`, the normal scope guard, and the exact visible-path audit passed.
- Repo-wide `cargo fmt --all --check` is not claimed green: it fails identically on clean `dev` at `a75082b` because of pre-existing formatting drift in more than one hundred unrelated Rust files. This backend wave does not absorb that legacy cleanup.
- No GitHub, network publication, or user-visible feature activation occurred.

Deferred slices remain exactly `dea4578`, `29b24fc`, `28e1bb0`, `d2db258`, `2576ba8`, the frontend portion of `6fe2a10`, and `0160d84`, all assigned to `2026-07-13-hidden-diarization-frontend-handoff.md`.

## Scope guard

Run `bash scripts/check-diarization-backend-migration.sh` from a normal clone, a linked worktree, or any subdirectory of either topology. The guard requires this record and its fixed inventories, keeps the implementation registry row at `migrating`, preserves the planning row at `verified`, verifies ancestry from `a75082bbae99bf9220cd02e36c3108d9f8a108bb`, and rejects committed, staged, or unstaged changes to the deferred visible frontend settings paths.

Passing the guard is evidence only for the hidden backend boundary. It is not evidence that diarization is user-visible or feature-complete.
