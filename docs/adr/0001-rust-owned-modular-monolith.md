# ADR 0001: Rust-Owned Modular Monolith for the macOS Fork

Status: Accepted

Date: 2026-08-11

## Context

**Current.** The application splits lifecycle authority across Rust globals, browser state, global events, SQLite, and file-system behavior. That makes ownership, ordering, recovery, and architecture capability difficult to reason about as one contract. The shared language is defined in the [domain context](../../CONTEXT.md), and the normative target is detailed in the [macOS modular-monolith contract](../architecture/macos-modular-monolith-contract.md).

Plain `FORK-NNN` references in this ADR identify work in the separately maintained foundation-line backlog. They are intentionally not links because that backlog is absent from this task baseline.

## Decision

**Target.** The product is one in-process Rust modular monolith, shipped as one macOS application. Rust is the sole owner and writer of Meeting, Recording Session, and Processing Job lifecycles, authoritative persistence transitions, model installation state, and update-sequence allocation. UI state is a projection of Rust snapshots and ordered updates.

The modules communicate through seven named interfaces: `RecordingSession`, `CaptureBackend`, `TranscriptionEngine`, `DiarizationEngine`, `SummaryEngine`, `MeetingRepository`, and `ModelRegistry`. This keeps ownership behind explicit seams without adding local HTTP or a second product runtime.

## Interface and transport consequences

**Target.** Tauri command adapters expose read-only queries and state-changing actions. Per-session or per-job `Channel` streams carry ordered authoritative lifecycle and progress updates. Global Tauri events MUST NOT carry audio chunks or act as the authoritative Recording Session or Processing Job stream, although they MAY remain for non-authoritative shell notifications.

The UI calls only the Tauri adapters. Core interfaces do not expose Tauri types, SQL rows, absolute recording paths, or concrete capture and engine implementations.

## Persistence consequences

**Target.** SQLite owns relational metadata, revisions, and lifecycle state. Recording files live below a configured external Recording Root. Meeting and session rows store only an opaque Recording Root identifier and normalized Relative Recording Paths, never an absolute recording path or audio blob.

Rust persists every authoritative transition before exposing it to the UI. File publication and recovery follow the no-clobber, integrity, and durable-publication protocol defined by the architecture contract.

## Platform consequences

**Target.** The deliverable is Universal `x86_64` and `arm64`, with macOS 14.2 as the minimum operating system. Every required Alpha behavior has a CPU implementation on Intel. Apple Silicon acceleration is optional, capability-probed, and MUST fall back to CPU for mandatory Alpha behavior.

## Identity consequence

**Current.** Legacy `Meetily`, `meetily`, and `com.meetily.ai` names remain temporary compatibility labels. **Target scope.** They remain provisional until an explicit identity decision. This ADR does not approve the old updater, endpoint, signing key, bundle ID, or public name and does not make a rename or publication decision.

## Alternatives considered

- **UI-owned lifecycle:** rejected because browser state cannot be the durable single writer for capture, jobs, persistence transitions, or recovery.
- **Global Tauri events as the job stream:** rejected because owner-scoped ordering, reconnect snapshots, generation invalidation, and gap recovery need an explicit interface.
- **Python/FastAPI sidecar as product orchestration:** rejected because a second runtime creates another lifecycle authority and a local network seam inside a single-user desktop product.
- **Absolute recording paths in SQLite:** rejected because external-root relocation and identity validation cannot be separated safely from Meeting metadata.
- **Apple-Silicon-only required execution:** rejected because Universal Alpha support requires every mandatory function to execute on Intel through a CPU path.

## Consequences

The benefits are one lifecycle authority, deterministic recovery, stable wire behavior, and explicit deep module seams. The costs are migration work in `FORK-033`, `FORK-034`, `FORK-043`, `FORK-045`, `FORK-052`, and `FORK-057`; temporary compatibility adapters while callers move; and persisted protocol versioning for snapshots and ordered updates.

## Scope of this ADR

This change creates documentation only. It does not alter runtime behavior and does not implement or prove the Channel protocol, storage migration, macOS 14.2 deployment floor, Universal slices, Intel fallback, updater state, identity, signing, notarization, legacy-runtime removal, or any downstream runtime or release requirement.
