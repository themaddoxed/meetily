# FORK-119 — app artifact and native test preparation

Date: 2026-09-23. This note records the selected combined app candidate, its hashes, launch evidence, and the native test plan. The exact `.app` has been opened through Launch Services and its process is running. FORK-119 remains blocked because this run used the ordinary `themaddoxed` account, no isolated test profile was prepared, and the initial TCC state could not be read.

## Current combined candidate — 2026-09-23

| Property | Value |
| --- | --- |
| App bundle | `.worktrees/feat071-fork051-integration-local/target/release/bundle/macos/meetily.app` |
| Disk image | `.worktrees/feat071-fork051-integration-local/target/release/bundle/dmg/meetily_0.4.1_x64.dmg` |
| Bundle ID / version | `com.meetily.ai` / `0.4.1` |
| Architecture / host | `x86_64` / macOS 15.8 (24H23), Mac mini 2018 (`Macmini8,1`) |
| Source identity | Detached HEAD `61e890f444f81718d8136beae6afc6f61a5f0e44`, MERGE_HEAD `f35d864370cbdafd088447c16f78b24e3f07c06a`, full tracked dirty diff SHA-256 `0609ae4c707c6d4e6df899a1cecfcf9c1bbc0b98d5f656e22d51e474c782df07` |
| Bundle content fingerprint | `85ee255367ba0102d53de5a02fdc7c15e04bd8b5459689a4cf9ec111ce26fb0e` |
| Bundle fingerprint method | SHA-256 over sorted lines `F\t<relative path>\t<SHA-256 of file contents>` for 17 regular files, with no trailing newline; 0 symlinks |
| Executable SHA-256 | `b98032e71e38413892dfaed07175be83a7f713667bfefddfd925d0a97df27eb8` |
| Info.plist SHA-256 | `865cd2da29adb93b575b4e9e3d7eb969f53faaae2fcc0cc97f1520353fb3233f` |
| DMG SHA-256 | `b4aac87efd317290d1409d876d1cead3aab151343d1b884dcc764fa51e8006e9` |
| Signing | Ad hoc, no TeamIdentifier; `codesign --verify --deep --strict` passed; no notarization |
| DMG validation | `hdiutil verify` passed; CRC32 `C5F061A4` |
| Launch | `open -a <exact bundle>` exited `0`; PID `84731` was confirmed running from the bundle's `Contents/MacOS/meetily` executable |

The app bundle built successfully from the combined dirty candidate. The full Tauri bundling flow hung while Finder decoration was running; the generated bundler script completed with `--skip-jenkins`, so the DMG has no Finder icon layout. The disk image itself passes `hdiutil verify`.

The launch evidence was initially process-level only. A direct executable invocation from inside the sandbox separately aborted in macOS `NSApplication`/Launch Services initialization; that path bypassed normal application launching. The subsequent normal `open -a` call returned success and the exact executable was observed running. On a live recheck on 2026-09-23, `CGWindowListCopyWindowInfo(.optionOnScreenOnly, ...)` reported an on-screen window owned by PID `84731`, titled `meetily`, layer `0`, bounds `1100×700` at `(410,129)`. This confirms that macOS presented an application window; its rendered contents and controls were not inspected, and no recording was observed. A read-only AX traversal found the window and app menus but no frontend button/slider nodes; an app-window-only `screencapture` attempt could not create an image. These inspection limits do not establish that the web UI is blank.

Read-only `system_profiler SPAudioDataType` outside the sandbox found PA248 (default system output), USB Condenser Microphone (default input), and built-in Mac mini speakers. The sandboxed query returned an empty list, so the device list above is from the unsandboxed read-only recheck. A read-only query for `com.meetily.ai` in the current user's TCC database returned `authorization denied`; no TCC setting was changed. `sysadminctl -guestAccount status` reported Guest User disabled; no isolated test profile was prepared or activated, and the user's original permission baseline remains unknown.

Focused checks against the same combined source diff passed: Node audio/lifecycle `31/31`; Rust macOS capture `28/28`, backend `25/25`, and RecordingSession `50 passed, 1 ignored`; privacy startup `9/9`. The Rust tests used Xcode 26.3 through `DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer` because `xcode-select -p` still points to Command Line Tools. These tests validate controlled code paths, not live device samples or permission recovery.

This combined candidate supersedes the earlier alignment conclusion below: it brings the lifecycle, source-control, and zero-analytics source changes into one exact app for the native acceptance plan. The source and artifact are now identified, but FORK-119 remains `blocked` until an isolated profile and initial permission state are recorded. No capture, pause/resume, mute/gain, device-reconnect, or quit/relaunch recording scenario has been run.

### Current-session capability recheck — 2026-09-23

The combined source diff and the executable, Info.plist, and DMG SHA-256 values still match the identifiers above; `codesign --verify --deep --strict` passed. This managed session could not enumerate the app process (`operation not permitted`), inspect its controls through System Events/Accessibility (`-10827`), or read the narrow `com.meetily.ai` TCC database query (`unable to open database file`). Its read-only `system_profiler SPAudioDataType` query returned an empty device list, and Guest User remains disabled. A separate macOS test account was not prepared or activated. No app launch, permission change, recording, or device manipulation occurred in this recheck. The previous Launch Services/window observation remains historical evidence for the same artifact, not current-session UI proof. FORK-119 remains blocked; FORK-120–122 and FEAT-084–086 remain unverified.

### Installation and launch recheck — 2026-09-23

At the user's request, the same exact candidate was installed with `ditto` at `/Applications/Meetily.app`; there had been no Meetily app in `/Applications` or `~/Applications`. `diff -qr` found no content difference between the source and installed bundles. Installed executable SHA-256 is `b98032e71e38413892dfaed07175be83a7f713667bfefddfd925d0a97df27eb8`, installed Info.plist SHA-256 is `865cd2da29adb93b575b4e9e3d7eb969f53faaae2fcc0cc97f1520353fb3233f`, bundle ID is `com.meetily.ai`, and `codesign --verify --deep --strict` passed. `spctl --assess` returned `internal error in Code Signing subsystem`, so Gatekeeper assessment is inconclusive; this remains an ad hoc, non-notarized candidate.

Launch Services `open -a /Applications/Meetily.app` exited `0`; a read-only process query found PID `7648` running from `/Applications/Meetily.app/Contents/MacOS/meetily`, still present after 43 seconds. No new Meetily crash report appeared during that check. The available Computer Use provider still reports `apps: []` and has no native `getApp` or `listApps` API, so the window contents and controls were not inspected. The user data directories were not removed. No recording, TCC change, or device change was performed. Installation and process survival do not prove that the reported app malfunction is resolved, and FORK-119 plus the dependent native checks remain open.

## Earlier FORK-051-only candidate — superseded by the combined candidate above

| Property | Value |
| --- | --- |
| App bundle | `.worktrees/fork-051-permissions-lifecycle/target/release/bundle/macos/meetily.app` |
| Bundle ID / version | `com.meetily.ai` / `0.4.1` |
| Architecture / host | `x86_64` / macOS 15.8 (24H23), x86_64 |
| Source base / combined dirty diff | `f35d864370cbdafd088447c16f78b24e3f07c06a` / `71f55701468f2438300c13bbc7e163eb18b28e275c983e9d471d2ffdd446bde5` |
| Bundle content fingerprint | `44fe95dabe37837a36b6a60535e4e904502711944c63f2654c28cd062fec4b1d` |
| Bundle fingerprint method | SHA-256 over the sorted manifest of all 18 regular-file content hashes and symlink targets |
| Executable SHA-256 | `7089846f63ef5cffb61a6acaa0802a83e0ab4843d95395872591df863eca9276` |
| Info.plist SHA-256 | `7ae38d497e673442ef02c754acccb1c5f5201c7801fcc1aa4052db6a5c116a8a` |
| Signing | ad hoc; `codesign --verify --deep --strict` passed; no notarization |
| Runtime launch | Not run; startup/UI behavior is not verified |

Build command: `pnpm --dir frontend exec tauri build --bundles app` in the `fork-051-permissions-lifecycle` worktree. It exited `0`, completed the release build in 3m 05s, and bundled one `.app`. Cargo compiled `tao v0.35.2` from `/private/tmp/fork051-tao-patched`. The app build emitted warnings, including two unnecessary nested `unsafe` blocks in the patch; they did not prevent the build.

The separate offline metadata probe, `cargo metadata --offline --locked --format-version 1`, now resolves the restored Tao path but stops because `alsa v0.9.1` is not cached. This did not block the normal Tauri build.

The original Tao registry archive SHA-256 is `a33f7f9e486ade65fcf1e45c440f9236c904f5c1002cdc7fc6ae582777345ce4`; it matches the local Cargo registry index checksum, and all 119 extracted files match the archive. The recovered two-site `app_state.rs` patch SHA-256 is `465325c969066decaa15aa3abd8e415f66c53be91b441712dbaf9bb2a4e1b499`; patched file SHA-256 is `01cee928403caacf7c32e2701359341dfafb462ba9dabd192c66c3dda6f54b57`. The static safety review found no confirmed actionable defect in the two replacements on Tao's normal launch path. It did not verify the suspected macOS 15.8 main-thread detection failure or prove that this workaround fixes startup; runtime confirmation remains required.

The source worktree diff fingerprint was unchanged by the build. The app bundle is not an accepted runnable test artifact: although it builds, it has not been launched in an isolated test profile. The current account's TCC state was not obtained: a read-only query limited to `com.meetily.ai` returned `OperationalError: unable to open database file`. No permission was changed. An isolated test account has not been prepared or activated in this run. `system_profiler SPAudioDataType` returned an empty `Devices:` list in this environment, so no audio input/output device is visible for the requested capture or reconnect scenarios. No app launch, capture, physical-device change, TCC scenario, or audio-track scenario was performed. The controlled native scenarios still need an isolated account, an agreed permission change, and visible audio hardware.

## Earlier candidate alignment check — 2026-09-23 (superseded)

At the time of this alignment check, no current exact source/app candidate satisfied the combined requirements. The combined candidate documented at the top of this report supersedes that conclusion.

The rebuilt bundle is bound to the dirty FORK-051 worktree at HEAD `f35d864370cbdafd088447c16f78b24e3f07c06a`, diff SHA-256 `71f55701468f2438300c13bbc7e163eb18b28e275c983e9d471d2ffdd446bde5`. That source retains `posthog-rs` and the native analytics module, and it has no `SourceControl` / `set_recording_source_control` UI or command. It therefore cannot be used for the current FEAT-082 privacy requirement or FEAT-084–086 mute/gain checks.

The FEAT-071 worktree at HEAD `61e890f444f81718d8136beae6afc6f61a5f0e44`, dirty diff SHA-256 `9c835e454df467b4be8b92d1c05c7706e26a6fcaf9578f5192bde5f4b4a7d3b0`, contains the source-control surface and the FEAT-082 analytics removal. It does not contain the FORK-051 commit series from the shared base `b7126ea82be887c38e8a8e8de30a48f9bb409e51`; neither worktree is an ancestor of the other. From that common base, the FORK branch changes 26 tracked paths and the FEAT branch changes 331; 24 paths are touched by both branch histories, including capture backend/CoreAudio/macOS, permission state/UI, `RecordingSession`, `recordingService`, and command registration. Their current dirty diffs also overlap in five paths: `Cargo.lock`, `frontend/src-tauri/Cargo.toml`, `frontend/src-tauri/src/lib.rs`, `frontend/src-tauri/src/tray.rs`, and `frontend/src/hooks/useRecordingStop.ts`. The FEAT worktree also does not configure the recovered Tao patch path used by the successful FORK-051 build.

**Selection:** no current exact source/app candidate satisfies the combined lifecycle, source-control, and zero-analytics requirements. The rebuilt FORK-051 app is not selected for FEAT native checks, and no app was built from the FEAT snapshot. Do not run the requested native scenarios against either tree until an isolated combined candidate is reviewed. Preparing that candidate requires an explicitly authorized local integration because the branches diverge and the dirty source changes overlap. The earlier build success remains valid as a build result for its exact source; it is not a launch or combined-feature result.

## Previous failed diagnostic artifact — 2026-09-22

| Property | Value |
| --- | --- |
| App bundle | `.worktrees/fork-051-permissions-lifecycle/target/release/bundle/macos/meetily.app` |
| Bundle ID / version | `com.meetily.ai` / `0.4.1` |
| Architecture | `x86_64` |
| Signing | ad hoc; no development team |
| Executable SHA-256 | `cb8de0f550c7d540d21299e94267c7e053ff1cbd4604cdd670d8dd1d62e8c837` |
| Bundle fingerprint | `60e0e4404eadb801977e348f6038b65d24c9cb155c4ed7db18a12214643856f6` |
| Info.plist SHA-256 | `7ae38d497e673442ef02c754acccb1c5f5201c7801fcc1aa4052db6a5c116a8a` |
| Build file timestamp | 2026-09-22 15:11:54 +0300 |

The package is in the dirty `fork-051-permissions-lifecycle` checkout at source `f35d864370cbdafd088447c16f78b24e3f07c06a`; the combined source diff SHA-256 recorded there is `71f55701468f2438300c13bbc7e163eb18b28e275c983e9d471d2ffdd446bde5`. `frontend/src-tauri/src/main.rs` was modified at 15:16:39 +0300, after the executable timestamp, so the bundle cannot be bound to the current dirty source tree by these facts alone.

The latest existing crash report is `meetily-2026-09-22-151213.ips`, SHA-256 `afb1f3a49f1b602bc7a430841583ffa34cd55862a4679d969045432da4e110ca`. It records `EXC_CRASH` / `SIGABRT` at 15:12:13 +0300 on macOS 15.8 (24H23), with a Rust panic in `tao::platform_impl::platform::app_delegate::did_finish_launching` before the app UI appeared. Its executable UUID matches this bundle. This app is evidence of the failure, not a selected test build.

At the earlier snapshot, the initial microphone/system TCC state had not been recorded or changed, no test profile had been created, and a new build had not been attempted. The local Cargo manifest pointed at the absent `/private/tmp/fork051-tao-patched`. Full Xcode is not installed (`xcodebuild` exits with “requires Xcode”); that fact alone does not establish that a Tauri desktop build with Cargo is impossible.

## Prepared native scenarios

Use a separate macOS test account/profile for the `com.meetily.ai` bundle ID. Before launch, record the successful candidate's source commit, complete bundle and executable hashes, signing identity, macOS build, selected microphone/output device, and the visible TCC state. Do not reset TCC as setup.

1. **FEAT-084 — separate source tracks.** Capture microphone-only, system-only, and simultaneous intervals with known audible phrases/tones separated by silence. Decode the saved microphone and system tracks, confirm each signal appears in its intended track, and record signal start/end positions. An empty system fallback is a failure.
2. **FEAT-085 — pause/resume alignment.** Put distinct markers before and after a pause, then compare decoded positions and durations. Set the acceptable timing tolerance before measuring; report offsets and missing or repeated samples explicitly.
3. **FEAT-086 — mute, gain, playback, and reprocessing.** For microphone and system independently, record raw input and controlled output with mute off/on and representative UI gains 0, 0.5, 1, and 2. Compare raw tracks, downstream saved audio, playback, and transcription; repeat transcription from the unchanged raw tracks.
4. **FORK-120 — permission recovery.** On the isolated test account, record current microphone permission, deny only within an agreed controlled run, return through System Settings, and confirm recovery. Do not run until a working exact app is available and the permission change is authorized for that account.
5. **FORK-121 — device reconnection.** Start capture, disconnect the selected device, record the visible and saved outcome, reconnect it, and verify source selection and recording recovery.
6. **FORK-122 — lifecycle.** Exercise start/stop, normal quit during idle and active capture, relaunch, and a repeated recording. Confirm stop completion and readable tracks after each run.

No scenario above has been executed against the current combined candidate. Its process-level launch is not a UI, permission, or recording check. To complete FORK-119 and unblock native tasks, prepare or activate the isolated test profile, record that profile's initial permission state, and confirm that the profile and required audio devices are ready.

## Historical follow-up trace of the earlier FORK-051-only candidate (2026-09-23)

The bundle executable UUID is `3BCA36E1-C11C-380B-AC79-49A76C413DEC`, matching both the 15:12:13 crash report and the adjacent `target/release/meetily` executable. The bundle and target executable SHA-256 values differ, so this establishes a matching Mach-O build UUID, not byte-for-byte identity.

The matching Cargo target fingerprint from 15:11:51 lists only the `default` feature. Its `app_lib` dependency was also built with `default`; `tray-icon` is a separate, non-default feature, and `tray::create_tray` uses a no-op implementation when that feature is off. The tray setup is therefore not supported as the cause of this crash.

The crash frames still show `panic_cannot_unwind` inside Tao's `did_finish_launching` callback, but the report has no panic payload. A narrowly filtered Unified Log query for the 45 seconds around the crash returned no panic or abort messages. The exact panic site remains unknown. Cargo dep-info points to `/private/tmp/fork051-tao-patched`, which is no longer present, so its exact Tao source cannot be inspected.

No rebuild, relaunch, TCC change, or native scenario was run during this follow-up. Cargo and Rust are available, but the local Tao patch source required to reproduce this build is missing. `xcodebuild` is unavailable because the selected developer directory is Command Line Tools and full Xcode is not installed; this is recorded as environment context, not as proof that Cargo alone cannot build the desktop app. The artifact remains a failed diagnostic candidate; FORK-119 stays blocked.
