> Public privacy copy: local user paths and contact/provenance fields were anonymized. Historical test results and original hashes do not verify this edited copy. See [publication notes](/docs/development/public-source-snapshot.md).

# FORK-123 recovery contract — independent QUALITY review

Status: **NOT_ADMITTED for implementation.** Evidence conclusions are substantiated, with one minor locator correction. Findings: **C0 / I0 / M1** in the reviewed candidate/evidence. The already documented design stops remain implementation blockers, not waived requirements or newly discovered implementation defects. This is not full plan, migration, native, or release acceptance.

## Finding

**Minor — incorrect plugin-dispatch source line.** `docs/fork/evidence/webview-migration-rebind-2026-10-04.json:106` records `plugin_dispatch_line: 1866`. In the exact locked `tauri-2.11.1/src/webview/mod.rs` bytes, `manager.extend_api(plugin, invoke)` is at **1863**; 1866 opens a mobile-only block. Correct the locator to 1863. The conclusion remains correct: plugin dispatch is separate from the application `manager.run_invoke_handler(invoke)` branch at 1909. Consequence is inaccurate reproducibility of the citation, not an invalid isolation conclusion.

No documented coding-standard violation or additional actionable smell was established. The design deliberately remains incomplete; the following choices must be resolved before implementation rather than represented as accepted code.

## Concrete admission decisions

1. **Bootstrap and recovery surface** — contract lines 32–51. `database/setup.rs:23` rejects unfinished file migrations; `lib.rs:369–373` propagates that failure before tray/notifications/models. Returning success to reach a recovery UI would also reach normal startup effects at `lib.rs:375–438`. Introduce a typed read-only classification and branch before those effects. Keep normal window creation and the provider tree (`src/app/layout.tsx:266` onward) withheld, not merely visually hidden. First-launch detection currently emits an event after a fixed 500 ms (`setup.rs:30–39`); use queryable bootstrap state for readiness and make events notifications only. Explicitly decide the first-launch import choice before starting normal writers.

2. **Capability and origin contract** — lines 41–51, 109–112. Current `tauri.conf.json:39` onward binds broad fs/store/notification permissions to main. A distinct recovery label avoids inheriting that label's capability but does not replace the native state/sender command gate. Freeze a minimal capability and exact command allowlist; inspect plugin initialization and the single-instance callback (`lib.rs:338–345`) before admitting builder changes. Keep recovery on the intended target WebView origin/data store: a separate label must not silently create a different IDB/localStorage namespace. Prove data written in recovery is visible after normal restart. Do not claim capabilities alone suppress ordinary WebView network access; retain an explicit recovery route/CSP and no-effect test boundary.

3. **Intent and Preparing resume** — lines 52–61. `migration_commands.rs:126–143` accepts compatible later phases but rejects Preparing. Define durable ordering of retained package, native intent, legacy journal and staged files, including crashes between them. Reconcile staged files by bound manifest/hash under the same owner, refusing mismatches and foreign entries. Preserve the existing app-data/migration lock order and prevent concurrent requests from opening two owners. Safe refusal is valid for corrupt data, but interruption of a valid operation needs the explicitly promised resume path.

4. **Preferences and legacy markers** — lines 62–82. Current adapter writes preferences before the IDB transaction (`webMigration.ts:255–265`), performs best-effort rollback, and skips all preference work on an existing marker (`webMigration.ts:413`). IDB currently commits records plus marker atomically with `add`, not overwriting destination rows (`indexedDBService.ts:711–782`). Preserve that collision behavior. Freeze a preference policy before durable intent: a conservative option is absent/equal-only admission with differing target values reported as conflicts; absent package keys remain untouched. If overwrite is desired, make the choice explicit and bind it to the intent. Define how a pending combined intent encountering an old, unversioned marker is handled; never reinterpret that marker as preference completion. Require exact record identity/content and intended preference readback, not counts or marker existence alone.

5. **Receipt authority and post-completion behavior** — lines 73–82. Browser facts remain a trusted-adapter report, not native proof of IDB. Freeze the versioned report schema, operation/hash binding, allowed sender, and owner-checked receipt transition. Native completion must require fresh file verification rather than merely returning a legacy Completed journal (`migration_commands.rs:197`). Treat a durable receipt as the normal-startup boundary, without replaying old preferences after later user edits. Specify response-loss and receipt-directory synchronization handling, and test each boundary with actual temporary FS/SQLite plus browser IDB/localStorage as the candidate already requires.

Export entry-point selection, producer/consumer identity admission, exact schemas/path allowlist and clean implementation checkout remain open. They must be reviewed before source edits. These resolutions reuse existing authority and avoid a second parallel migration framework.

## Observed snapshot and checks

The supplied base is `53f4415058fb01e38b8892a47ac036bfffa394ba`. Independently recomputed every current and base hash in the eleven-path census using Python hashlib and `git show <base>:<path>`: all matched. Only lib.rs differs within this census; its diff adds meeting-question module/command registrations and must be preserved. This is not a claim that the entire repository has only one changed file.

Read the FORK-123 PRD (`docs/prd/macos-fork.md:825–832`), current setup, migration commands/startup guard, adapter, IDB transaction, layout and inline capabilities. The existing standards require real preservation/crash recovery tests (`docs/testing.md:118`, `:529`); no mock result was substituted for those gates.

Verified the cached Tauri 2.11.1 crate SHA against Cargo.lock, and compared its archived webview source with the local extracted source. Both matched the evidence. No external API assumption was needed beyond these exact local dependency bytes.

Independently ran the retained Node probe in an outer VM using the actual transpiled adapter. A narrow fs proxy intercepted its sole output write, compared the resulting object with the retained JSON, and did not overwrite the original script/report. The check completed exit 0: `already_imported`, one marker read, zero commits, zero preference writes, target dark while package light. This demonstrates the marker's insufficiency for preferences; it does not demonstrate a real browser crash or prove that successfully migrated preferences should overwrite subsequent edits. The retained probe's empty records and fake IDB/Storage are explicit limits. The recorded earlier harness failure was not independently reconstructed.

No product, Git, native application or user profile mutation occurred. `changed_paths`: only this report. No browser/native acceptance tests or full project gate were run for this design review.

## Exact evidence hashes

- `/Users/developer/Desktop/meeting-minutes/docs/superpowers/plans/2026-10-04-webview-recovery-contract.md`: `e517235c686d6ed08c23ac0da5d377298d99910ca2bd9ccdf96749b793e968b3`
- `/Users/developer/Desktop/meeting-minutes/docs/fork/evidence/webview-migration-rebind-2026-10-04.json`: `a218576b6f034d79dadef2b603002af74547b3147ece11d42feb6008f95cdcff`
- `/private/tmp/meetily-webview-marker-probe-2026-10-04.cjs`: `5fdd3b9f755844b93a68fe2bf210c41fc93dfcb15245ac35ef07e241ab9e8f9c`
- `/private/tmp/meetily-webview-marker-probe-2026-10-04.json`: `668e3e382d81a8727b78ddf1753d386f654e9bdcc896071f0c71ba1d4194486a`

Locked crate SHA-256: `b93bd86d231f0a8138f11a02a584769fe4b703dc36ae133d783228dbc4801405`. Locked webview source SHA-256: `954cf212cb79d6b566c10b76d2ff2da05bc637cfc9f666c2f03d4e3dddf35f1c`. The exact eleven source hashes remain enumerated in the pinned census and were independently checked.

Next action: correct the minor citation, settle the enumerated admission decisions in a concrete bounded contract, then obtain fresh SPEC/QUALITY review and admit a clean implementation snapshot. No task close or real-profile migration is authorized by this review.
