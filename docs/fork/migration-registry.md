# macOS fork migration registry

This is the publish-safe frozen routing record for `FORK-030`. Raw local paths,
session identifiers, parent graphs, event times, claim locators, worktree status
names, GitHub responses and the HMAC key remain only in the protected local
evidence bundle. The tracked files contain project-scoped source rows, stable
labels, keyed tokens and aggregate counts. Normal validation never opens the
protected bundle or any live session.

<!-- FORK030 baseline_commit=1e742fa22030fe5720db6a791407c64eea03ede4 baseline_tree=1fbcc32d422e5d86c5266f4dfb8b7ce863c86845 pre_task_sha=0613f989133112f13d6bb0a2856b349a334ca680 patch_source_parent=6dc65701a05dec8379bdf4a5f78ab26675ae73d2 patch_source_commit=5424bc7648dd7fcd44098269a27bb7b3cdc17927 patch_destination_parent=6fe2a10628fa651fc73fe6a50f72bd4241f4f7da patch_destination_commit=c0201eb064a17229b137ba878ca5eab55db59417 patch_id=b360c26eba64fcb44dcfccde8bf484df297bd2f7 -->

## Frozen source summary

| Source | Publish-safe accounting |
|---|---|
| PRs | 8 project PR rows; repository/author/head provenance is keyed and exact readback commands remain protected |
| Branches | 125 project ref rows: 43 local plus an 82-name remote union; raw ref names remain protected |
| Worktrees | 18 stable labels and 319 keyed status tuples; raw locations and status names remain protected |
| Session discovery | 1860 cutoff-bounded paths proven by aggregate HMAC; only 539 project candidates have pseudonymous tracked rows |
| Session accounting | 256 included claims and 283 structured exclusions; exact metadata, prefix and claim locators remain protected |

`scripts/check-fork-migration-registry.sh validate` is clone-safe: it checks the
sealed publish-safe records, assertion formats and references to tracked paths,
without requiring historical objects outside the current branch. The explicit
`--evidence-check BUNDLE` mode enforces the private bundle boundary, derives the
complete candidate projection from independently derived authoritative roots,
exact project-location patterns and the integer cutoff, while excluding valid
later rollouts from the frozen universe. It also recomputes the protected Git,
session and worktree evidence. `--drift-check BUNDLE` additionally classifies
live state.

## Canonical WORK routing matrix

| id | title | status | source_ids | destination_or_decision | destination_sha | verifier_ids | evidence_ids | dependencies | owner_tasks | gap_id | target_id | successor_work_id | notes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| WORK-001 | Clean lockfile baseline | partial | BR-0003 | Current source is inventoried; provisional-line destination and frozen-install evidence remain open. | - | V-LOCKFILE | - | - | FORK-031 | GAP-001 | - | - | Historical verified state is not current fork evidence. |
| WORK-002 | Semantic theme and custom summary prompt | partial | BR-0030,BR-0032,PR-001,PR-002,PR-008 | Preserve semantic behavior rather than final visual design. | - | V-THEME | - | WORK-001 | FORK-039 | GAP-002 | - | - | PR state is source state only. |
| WORK-003 | Neutral palette and native macOS theme sync | partial | BR-0013,BR-0020,BR-0030,PR-002 | Route semantic native sync and visual replacement separately. | - | V-NATIVE-THEME | - | WORK-002 | FORK-039,FORK-046 | GAP-003 | - | - | Latest-wins lifecycle requires current verification. |
| WORK-004 | Universal Tauri target and sidecar packaging | partial | BR-0016,BR-0020,BR-0032 | Produce the macOS-only package path and one exact Alpha artifact. | - | V-RELEASE-PROVENANCE,V-UNIVERSAL-PACKAGING | - | WORK-001 | FORK-032,FORK-033,FORK-035,FORK-053 | GAP-004 | - | - | Historical unsigned DMGs remain diagnostic only. |
| WORK-005 | Interface polish typography transitions and motion | partial | BR-0011,BR-0020,BR-0032 | Retain accessibility and motion evidence as input to the new UI. | - | V-UI-BEHAVIOR | - | WORK-002,WORK-003 | FORK-046,FORK-048 | GAP-005 | - | - | Product appearance is owned by the new design. |
| WORK-006 | Gemma 4 built-in summary models | partial | BR-0009,BR-0036,PR-001,PR-003,PR-008 | Verify the model flow on the provisional line. | - | V-GEMMA-FLOW | - | WORK-001,WORK-002 | FORK-040 | GAP-006 | - | - | Historical test counts are not current evidence. |
| WORK-007 | Gemma 4 design and implementation plans | partial | BR-0001,BR-0010,BR-0023 | Preserve historical docs and establish a current canonical destination. | - | V-DOC-PROVENANCE | - | WORK-006 | FORK-040 | GAP-007 | - | - | Legacy aliases remain provenance only. |
| WORK-008 | Gemma 4 macOS recommendation | partial | BR-0008,BR-0020,PR-003 | Reverify the recommendation and route benchmark decisions. | - | V-GEMMA-RECOMMENDATION | - | WORK-006 | FORK-040,FORK-049,FORK-050 | GAP-008 | - | - | RAM boundaries need current evidence. |
| WORK-009 | Interrupted-meeting recovery on first relaunch | partial | BR-0012,BR-0020,BR-0032 | Route first-relaunch offline and upgrade recovery behavior. | - | V-RECOVERY | - | WORK-001 | FORK-052 | GAP-009 | - | - | Repository recovery claims are routed to WORK-028. |
| WORK-010 | Browser user-agent analytics privacy fix | partial | BR-0005,BR-0020,BR-0032 | Retain privacy behavior after macOS cleanup. | - | V-ANALYTICS-PRIVACY | - | WORK-001 | FORK-031,FORK-038 | GAP-010 | - | - | Raw user-agent exclusion must be reverified. |
| WORK-011 | Audio verification stabilization canonical copy | partial | BR-0022,BR-0032 | Keep exactly one patch-equivalent implementation. | - | V-AUDIO-GATE | - | WORK-013 | FORK-034,FORK-051 | GAP-011 | - | - | Exact stable patch inputs and patch ID are machine-checked. |
| WORK-012 | Speaker diarization design and migration plans | partial | BR-0007,BR-0033,PR-004 | Preserve historical docs and establish a current canonical destination. | - | V-DOC-PROVENANCE | - | WORK-001 | FORK-041,FORK-042 | GAP-012 | - | - | Planning evidence is not runtime parity. |
| WORK-013 | Speaker diarization and speaker rename chain | partial | BR-0006,BR-0022,BR-0033,PR-004 | Keep runtime parity and visible settings as separate gates. | - | V-DIARIZATION-RUNTIME,V-DIARIZATION-UI | - | WORK-011,WORK-012 | FORK-041,FORK-042 | GAP-013 | - | - | No current UI or device claim is inferred. |
| WORK-014 | Native Whisper streaming foundation | missing | BR-0038,BR-0080,PR-005 | Selectively review and implement later. | - | V-WHISPER-STREAMING | - | WORK-011 | FORK-045 | - | FORK-045 | - | Draft PR is not implementation evidence. |
| WORK-015 | Native Whisper streaming review follow-up | missing | BR-0038,BR-0080,PR-005 | Apply only after WORK-014 and review the broad follow-up diff. | - | V-WHISPER-STREAMING | - | WORK-014 | FORK-045 | - | FORK-045 | - | Deleted historical docs require explicit review. |
| WORK-016 | Recording save-folder selection | partial | BR-0002,BR-0014,BR-0031,BR-0079,PR-006 | Repeat save import restart concurrency and recovery behavior. | - | V-RECORDING-FOLDER | - | WORK-001 | FORK-043 | GAP-016 | - | - | PR state is not destination proof. |
| WORK-017 | Design-system extraction and Pencil assets | partial | BR-0040 | Preserve artifact provenance while the new UI owns product behavior. | - | V-DESIGN-ASSET | - | WORK-002,WORK-013 | FORK-046,FORK-048 | GAP-017 | - | - | Design assets are not product migration evidence. |
| WORK-018 | LFM2.5-Audio experiment | missing | BR-0067 | Make an architecture decision only after the model roadmap. | - | V-MODEL-EXPERIMENT | - | WORK-006 | FORK-050 | - | FORK-050 | - | Never merge the experiment wholesale. |
| WORK-019 | Earlier Whisper streaming improvements | superseded | BR-0077 | Preserve as a comparison oracle only. | - | V-PATCH-COMPARISON | - | WORK-014 | FORK-045 | - | - | WORK-014 | Superseded never authorizes branch deletion. |
| WORK-020 | Disable live transcription while recording | missing | BR-0075 | Record an explicit transcription-policy decision. | - | V-TRANSCRIPTION-POLICY | - | WORK-023 | FORK-045,FORK-051 | - | FORK-045 | - | Policy must precede queue work. |
| WORK-021 | Background transcription queue and progress | missing | BR-0075 | Selectively implement queue behavior after policy. | - | V-TRANSCRIPTION-QUEUE | - | WORK-020 | FORK-045,FORK-052 | - | FORK-045 | - | Queue and recording regressions remain open. |
| WORK-022 | Consolidated toast placement and suppression | partial | BR-0075 | Select toast behavior under the new design. | - | V-TOAST-BEHAVIOR | - | WORK-021 | FORK-048 | GAP-022 | - | - | One accessible region and deduplication need proof. |
| WORK-023 | Audio import and retranscription | partial | BR-0032,BR-0065 | Retain baseline provenance and reverify on the provisional line. | - | V-AUDIO-IMPORT | - | WORK-001 | FORK-031,FORK-052 | GAP-023 | - | - | Historical merged state is not current fork evidence. |
| WORK-024 | Toolchain pin alignment | missing | BR-0024 | Make an isolated macOS CI and toolchain alignment change. | - | V-TOOLCHAIN | - | WORK-001 | FORK-035 | - | FORK-035 | - | No incidental lockfile regeneration is allowed. |
| WORK-025 | Immutable macOS-fork baseline | present | BR-0032 | baseline-document | 952e3614fe271118d0783dbc0e84ef10d7d61650 | V-BASELINE | E-BASELINE | - | FORK-026,FORK-031 | - | - | - | Exact baseline object and blob passed. |
| WORK-026 | Universal processors and macOS 14.2 policy | present | BR-0043 | platform-policy | 0613f989133112f13d6bb0a2856b349a334ca680 | V-PLATFORM-POLICY | E-PLATFORM-POLICY | WORK-025 | FORK-027,FORK-035,FORK-053 | - | - | - | Policy consistency passed; physical runtime is outside this claim. |
| WORK-027 | Short Whisper utterance fix | missing | BR-0041,BR-0125,PR-007 | Transfer the focused fix from PR 676. | - | V-SHORT-WHISPER | - | WORK-014 | FORK-044 | - | FORK-044 | - | Draft PR remains a source artifact only. |
| WORK-028 | Provisional Git and final handoff policy | partial | BR-0043,BR-0107,WT-011 | Preserve current provisional policy; final switch and archive decision remain open. | - | V-GIT-POLICY,V-HANDOFF | - | WORK-025,WORK-026 | FORK-029,FORK-031,FORK-054 | GAP-028 | - | - | Dirty worktrees remain unresolved FORK-054 handoff gates. |
| WORK-029 | Product name bundle ID and repository identity | missing | BR-0043 | Require an explicit owner-approved identity decision. | - | V-IDENTITY-DECISION | - | WORK-028 | FORK-028,FORK-036,FORK-037,FORK-046,FORK-054 | - | FORK-028 | - | Legacy labels are source labels only. |

## Verifier catalog

| id | required gate |
|---|---|
| V-BASELINE | Exact commit, tree, object and baseline document check |
| V-DOC-PROVENANCE | Source object reachability plus destination digest |
| V-LOCKFILE | Frozen install plus unchanged lockfile digest |
| V-THEME | System light/dark semantic contrast and summary prompt behavior |
| V-NATIVE-THEME | Native appearance lifecycle and latest-wins behavior |
| V-UNIVERSAL-PACKAGING | App and sidecar slices plus macOS-only package composition |
| V-RELEASE-PROVENANCE | One source commit to one app and DMG with signing/notarization report |
| V-UI-BEHAVIOR | Reduced motion, keyboard, accessibility and geometry acceptance |
| V-GEMMA-FLOW | Download, hash, selection, prompt, RAM rule and local generation |
| V-GEMMA-RECOMMENDATION | Boundary cases plus benchmark handoff |
| V-RECOVERY | First relaunch, offline and upgrade persistence |
| V-ANALYTICS-PRIVACY | Payload excludes raw user agent while retaining safe categories |
| V-AUDIO-GATE | Patch-id uniqueness plus focused audio verifier |
| V-DIARIZATION-RUNTIME | Engine, alignment, persistence, reopen and rename runtime |
| V-DIARIZATION-UI | Visible enable/disable and post-call settings behavior |
| V-WHISPER-STREAMING | Latency, bounded memory, cancellation, Intel fallback and lifecycle |
| V-RECORDING-FOLDER | New recording, restart, import, concurrent save and recovery |
| V-DESIGN-ASSET | Exact artifact label/digest inventory only |
| V-MODEL-EXPERIMENT | Architecture, privacy, packaging, resource and quality decision |
| V-PATCH-COMPARISON | Stable patch-id and diff oracle comparison |
| V-TRANSCRIPTION-POLICY | Disabled-live recording and post-call route decision |
| V-TRANSCRIPTION-QUEUE | Ordering, cancellation, progress and recording regression |
| V-TOAST-BEHAVIOR | One accessible region and deduplication behavior |
| V-AUDIO-IMPORT | Import, retranscribe, persist and reopen |
| V-TOOLCHAIN | Local/CI agreement, frozen install, build and clean lockfile |
| V-PLATFORM-POLICY | Policy consistency only; not physical runtime |
| V-SHORT-WHISPER | Non-empty short text without text-length confidence gate |
| V-GIT-POLICY | Policy and ref-topology consistency without mutation |
| V-HANDOFF | Exact final refs, remotes, protection, rollback and owner approval |
| V-IDENTITY-DECISION | Owner-approved product/repository/fork-network identity |

## Evidence boundary

In protected evidence mode, the two `present` rows resolve to exact immutable
Git objects. Normal clone-safe validation checks only their sealed assertions
and tracked path references. All other rows remain non-present because
historical PR state, session claims, automated tests or diagnostic artifacts do
not prove current behavior, physical-device runtime, Developer ID signing,
Gatekeeper acceptance, notarization or release readiness.

`superseded` is routing only. It never permits branch/worktree deletion, PR
closure or data cleanup. Protected dirty/untracked worktrees remain FORK-054
handoff gates until separately inventoried and approved.

## FORK-031 provisional integration line

- Ref: `fork/macos-integration`.
- Worktree: `.worktrees/macos-fork-integration`.
- Immutable commit: `1e742fa22030fe5720db6a791407c64eea03ede4`.
- Immutable tree: `1fbcc32d422e5d86c5266f4dfb8b7ce863c86845`.
- `main`, `dev`, remotes, local remote-tracking refs, tags and all pre-existing
  worktrees remained unchanged under the private canonical topology comparison.
- No push or publication occurred. Rollback is quarantine only; it does not
  authorize deleting, detaching, resetting, renaming or overwriting the line.
- The protected exception is stored only as `fork-031-exception.tsv` in the
  ignored evidence bundle; raw location and HMAC values are not published.

`WORK-001`, `WORK-010`, `WORK-023` and the final `FORK-054` portion of
`WORK-028` remain `partial`. Creating this branch is routing infrastructure,
not behavioral migration evidence.

## FORK-032 macOS packaging cleanup

- `fork/macos-integration` and `fork/fork-032-remove-non-macos-artifacts` both
  point to the clean commit `45bebcbf184da4d2b16c48a0415050019becff98`,
  which is a fast-forward descendant of the immutable baseline.
- The exact product diff contains 35 paths: 34 deleted Windows scripts and
  binary resources plus the one Tauri configuration edit. No tracked
  DEB/RPM/AppImage output existed at the baseline.
- Tauri packaging now declares only `app` and `dmg`, retains only PNG/ICNS
  bundle icons, and omits the Windows bundle configuration.
- The six retained broad-inventory paths are routed without claiming their
  implementation: Linux and Windows audio source to `FORK-034`, the two
  Windows/Linux workflows to `FORK-035`, and the Linux build document plus the
  permanent web-favicon exception to `FORK-038`.
- The scoped release-policy suite passed 8/8. `tauri info` is
  `NOT_COMPLETED` after hanging beyond its environment output, and full Xcode
  is not installed on this host. No build, Universal slice, physical runtime,
  signing, notarization or release-readiness claim follows from this cleanup.
- No push or publication occurred.

No canonical WORK status is promoted by `FORK-032`. `WORK-004` remains
`partial` until its Rust, CI, build and exact-artifact gates are satisfied;
`WORK-011` and `WORK-024` likewise remain unchanged because this task did not
implement audio or toolchain/CI work.
