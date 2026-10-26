# Meetily packaged test-plan smoke report

Copy this file for one exact candidate. Replace every placeholder, run every row
on a real Mac, and keep concrete evidence such as a screenshot name, exported
file path, log timestamp, or observed text. `NOT_RUN`, `FAIL`, blank evidence,
and generic statements such as “works” block this test plan.

This template excludes Developer ID signing, Gatekeeper, and notarization. Use
`smoke-report-template.md` and `release-gate.sh` when validating public-release
readiness.

Do not delete or overwrite real user data to perform these checks. Use a test
macOS account or a backed-up test data directory for clean-install and upgrade
scenarios.

Operator: <full name>
Date: <YYYY-MM-DD>
Commit SHA: <full commit SHA>
Candidate report: <path to candidate-report.json>
Artifact SHA-256: <DMG SHA-256 from candidate-report.json>
Mac model: <model and chip>
macOS version: <exact version>
Audio devices: <microphone and system-audio device>

Allowed status values: `PASS`, `FAIL`, `NOT_RUN`. This test plan requires
`PASS` in every row.

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
