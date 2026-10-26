# Meetily release smoke report

Copy this file for one exact candidate. Replace every placeholder, run every row
on a real Mac, and keep concrete evidence such as a screenshot name, exported
file path, log timestamp, or observed text. `NOT_RUN`, `FAIL`, blank evidence,
and generic statements such as “works” block release.

Do not delete or overwrite real user data to perform these checks. Use a test
macOS account or a backed-up test data directory for clean-install and upgrade
scenarios.

Optional runtime evidence may include output from
[`scripts/smoke-local-models.sh`](../../scripts/smoke-local-models.sh) and
[`scripts/smoke-macos-audio.sh`](../../scripts/smoke-macos-audio.sh). These
opt-in checks do not download models and can support diagnosis, but they do not
launch the exact installed UI/package. Their output alone never changes a row
to `PASS` and does not replace visible UI, permission, microphone, packaged
offline, restart, export, or upgrade evidence. In particular, the macOS audio
script returns `OVERALL FAIL`/exit 1 if either system-device/CoreAudio subcheck
fails, while microphone remains `NOT_RUN`. If both subchecks pass, it exits 2
and keeps the overall result `NOT_RUN` until microphone and UI checks are
performed. Pass the exact built app through `MEETILY_PACKAGED_APP`; the script
uses an isolated temporary copy as the CoreAudio test host and does not modify
the original bundle.

Operator: <full name>
Date: <YYYY-MM-DD>
Commit SHA: <full commit SHA>
Candidate report: <path to candidate-report.json>
Artifact SHA-256: <DMG SHA-256 from candidate-report.json>
Mac model: <model and chip>
macOS version: <exact version>
Audio devices: <microphone and system-audio device>

Allowed status values: `PASS`, `FAIL`, `NOT_RUN`. A release requires `PASS` in
every row.

| ID | Status | Evidence |
| --- | --- | --- |
| SMOKE-LAUNCH | NOT_RUN | <Launch the DMG-installed app; record startup result and screenshot/log time> |
| SMOKE-CLEAN-PERMISSIONS | NOT_RUN | <In a clean test account, record microphone and screen/system-audio permission prompts and choices> |
| SMOKE-AUDIO-SOURCES | NOT_RUN | <Record the microphone and system-audio sources selected in the UI> |
| SMOKE-MIC-CAPTURE | NOT_RUN | <Speak a unique phrase; record visible level/transcript and saved playback evidence> |
| SMOKE-SYSTEM-CAPTURE | NOT_RUN | <Play a unique phrase through system audio; record visible level/transcript and saved playback evidence> |
| SMOKE-LIVE-TRANSCRIPT | NOT_RUN | <Record two exact live transcript phrases and timestamps> |
| SMOKE-NORMAL-FINISH | NOT_RUN | <Stop normally; record that processing finishes without losing prior transcript> |
| SMOKE-FORCED-QUIT-RECOVERY | NOT_RUN | <Force quit during recording, relaunch, recover, and record exact recovered phrases/audio status> |
| SMOKE-PERMANENT-SAVE | NOT_RUN | <Open the saved meeting and record title, transcript count, and audio location> |
| SMOKE-SPEAKER-RENAME | NOT_RUN | <Rename a speaker through the visible UI, reopen, and record the persisted name> |
| SMOKE-DIARIZATION-SUCCESS | NOT_RUN | <Run diarization; record speaker assignments and preserved transcript text> |
| SMOKE-DIARIZATION-FAILURE | NOT_RUN | <Trigger or observe a safe failure; record that the original transcript remains byte-for-byte readable> |
| SMOKE-SUMMARY-SUCCESS | NOT_RUN | <Generate with a local model; record model, summary heading, and saved result> |
| SMOKE-SUMMARY-FAILURE | NOT_RUN | <Test missing/broken local model; record understandable error and unchanged saved transcript> |
| SMOKE-EXPORT-UI | NOT_RUN | <Export through the visible app UI; record control used and destination> |
| SMOKE-EXPORT-ARTIFACT | NOT_RUN | <Open exported file; record exact transcript phrase, summary heading, and file checksum> |
| SMOKE-OFFLINE | NOT_RUN | <Disable network before launch/record/summary/reopen; record which local operations succeed and any network attempts> |
| SMOKE-RESTART-REOPEN | NOT_RUN | <Quit normally, relaunch, reopen meeting; record transcript, speakers, summary, and audio status> |
| SMOKE-UPGRADE-PREVIOUS | NOT_RUN | <Back up and open a previous-version test database; record migration and preserved known rows> |
| SMOKE-OLD-DATA | NOT_RUN | <Open at least one older healthy meeting after upgrade; record known title and transcript phrase> |
| SMOKE-LOGS | NOT_RUN | <Record log paths and timestamps; confirm errors contain diagnosis but no transcript/audio content or secrets> |
| SMOKE-CHECKSUM | NOT_RUN | <Run shasum -a 256 on the tested DMG and record exact match to Artifact SHA-256> |
| SMOKE-SIGNATURE | NOT_RUN | <Record codesign verification and exact Developer ID authority> |
| SMOKE-GATEKEEPER | NOT_RUN | <Record spctl assessment output for the installed app> |
| SMOKE-NOTARIZATION | NOT_RUN | <Record stapler validation output for this exact app> |
