# Meetily development context

Start with [INDEX.md](INDEX.md) and [AGENTS.md](AGENTS.md). For a build or native
check, use [docs/BUILDING.md](docs/BUILDING.md) and [docs/testing.md](docs/testing.md).
For branch work, use [Git workflow](docs/development/git-workflow.md). The current
public repository has no legacy commit graph; old source/patch-provenance scripts
and worktree records require the private archive.

## Runtime boundaries

The product target is macOS 14.2+ on Intel and Apple Silicon. Runtime/Universal
support is an acceptance requirement, not a consequence of compiling slices.
The app is Tauri 2 with a Rust core, static Next.js/React UI, local SQLite and a
separately packaged model helper. There is no Python/FastAPI server or Electron
process. Browser fixture IPC is for tests and is absent from the production UI.

Audio capture, transcription and persistence are local. Summary and meeting-Q&A
can send meeting text to the selected remote provider; network Q&A requires
consent. Refer to [data handling](PRIVACY_POLICY.md) before making network/offline
claims. Model downloads and provider requests are distinct from an analytics
collector; the current product has no PostHog runtime.

## Source map

- `frontend/src-tauri/src/lib.rs`: commands, app setup and shared state.
- `audio/capture/backend.rs`: capture-owner controls, bounded channels and source
  sinks. `audio/capture/macos.rs` uses the CoreAudio capture implementation;
  system capture is not documented as requiring a virtual audio driver.
- `audio/recording_session.rs` and `audio/recording_session_state.rs`: session
  ownership, timeline and transitions. Preserve raw audio and bounded cleanup.
- `database/` and `migrations/`: SQLite transactions, schema and reopen behavior.
  Applied SQLx migrations are byte-immutable, including whitespace. Add a new
  migration for a schema change and preserve upgrade/reopen data.
- `whisper_engine/acceleration.rs`: Intel chooses CPU; arm64 chooses Metal.
  Compiled features and a hardware probe do not establish measured performance.
- `parakeet_engine/`, `model_download.rs`, `diarization/`: local model paths.
- `summary/summary_engine/`: helper lifecycle, generation and model definitions.
- `meeting_questions.rs`: scoped meeting context, consent and citation contract.
- `frontend/src/services/`, `hooks/`, `components/`: WebView services and UI.

The paths above are relative to the native `src/` except those prefixed with
`frontend/`. Inspect the implementation before relying on an archived code sample
or performance claim. Use Tauri path APIs rather than machine-specific paths.

## Work and verification

CI pins Node.js 22.17.0 and pnpm 10.17.1. Install the frozen lockfile; dependency
changes need an explicit reviewed diff. The build guide prepares the host helper
and SDK before native checks. Use `scripts/check.sh` for the applicable checks
and the exact candidate/smoke/release procedure before release claims.

Task status comes from `docs/prd/`; plans and old reports are evidence for their
own date and artifact, not a second backlog or proof for the current commit.
Manual/native acceptance is the final stage. Keep new commit identities neutral,
run the public privacy guard, and redact logs before publication.
