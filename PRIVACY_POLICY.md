# Meetily data and network behavior

Updated 2026-10-27. This document describes the source snapshot and configurable
behavior; it does not claim legal certification or completed native offline
acceptance for a released artifact.

## Local data

The Rust/Tauri application records audio, transcribes with local Whisper/Parakeet
models, and stores meeting data in local files and SQLite. Export destinations,
retention and installed permissions are controlled through the application/OS.
The source snapshot includes no user database, real meeting recording or private
credentials. Synthetic fixtures and public license attribution remain.

## Selected providers

Local summary and meeting-Q&A providers need an available model and runtime.
Selecting a remote provider sends the relevant prompt/meeting text to that
provider's configured endpoint. Meeting-Q&A requires explicit network consent
for a network transport. Credentials configured for a provider belong in runtime
settings or the supported secret mechanism, not in repository files.

Model downloads, explicitly configured endpoints and platform services may use
the network. Request routing and selected providers must be considered when
choosing a local-only configuration; the repository does not promise that every
configuration is offline.

## Analytics and evidence

The current product source contains no PostHog analytics runtime. Older documents
about an optional analytics collector describe removed upstream code, not a
currently implemented settings feature. Development logs, user-selected remote
providers and OS/platform behavior are separate from an analytics collector.

The browser `desktop-local-safety` contract observes WebView requests and mocked
provider/updater failures. It does not observe all native traffic. A claim of
packaged offline operation requires the exact installed artifact and native
network smoke in [docs/testing.md](docs/testing.md).

## Public source hygiene

The initial public commit uses neutral author/committer metadata. Personal home
paths/contact fields were anonymized; screenshots with meeting/local context and
unreviewed animations were omitted. Edited historical logs are labeled as privacy
copies and their original hashes are not evidence for the edited bytes.
See [publication audit](docs/reviews/cross-cutting/publication-privacy-audit-2026-10-27.md).

Automated secret scanning and OCR reduce accidental exposure; they cannot prove
absence of every kind of sensitive content or erase earlier public copies.
