# Meetily for macOS

A Tauri desktop meeting assistant with a Rust core and a Next.js/React interface.
This independent repository starts from an anonymized source snapshot; it is not
a signed or accepted application release.

## Current product scope

The application targets macOS 14.2 or newer on Intel and Apple Silicon. Universal
packaging is a build target; support on both architectures and the minimum OS
requires smoke evidence for the exact artifact. See [platform policy](docs/fork/platform-support.md).

The source includes microphone/system-audio capture, local Whisper/Parakeet
transcription, SQLite meeting storage, search, summary generation and questions
about the selected meeting with source citations. Implementation and automated
checks do not replace installed-app acceptance; open work is in [docs/prd](docs/prd).

## Data and network behavior

Audio capture and local transcription run on the device. Summary and meeting-Q&A
requests use the provider selected in settings. Choosing a remote provider sends
the relevant meeting text to that endpoint; network Q&A additionally requires
consent. Local providers require their model/runtime to be available. Model
acquisition and configured provider endpoints can use the network.

This repository does not promise that every configuration is offline. The
browser egress contract covers WebView traffic in a fixture harness; native
packaged offline behavior still requires the exact-artifact smoke check.
See [data handling](PRIVACY_POLICY.md) and [release/testing guide](docs/testing.md).

## Development

Start with the [project index](INDEX.md) and [macOS build guide](docs/BUILDING.md).
CI pins Node.js 22.17.0 and pnpm 10.17.1. Install the frozen frontend lockfile:

```bash
pnpm --dir frontend install --frozen-lockfile
```

The [build guide](docs/BUILDING.md) describes the required host helper and Xcode
setup before native development. Then:

```bash
pnpm --dir frontend tauri:dev
./scripts/check.sh --quick
./scripts/check.sh --full
```

CI automatically checks pushes and pull requests for `main`/`dev`; see
[workflow guide](.github/workflows/WORKFLOWS_OVERVIEW.md). Successful CI is source
and automated build evidence, not release approval. The candidate verifier,
installed smoke report, Developer ID/Gatekeeper/notarization checks and release
gate remain required by [docs/testing.md](docs/testing.md).

## Architecture and project records

The runtime is Tauri/Rust with SQLite and a separately packaged local-model
helper; there is no Python/FastAPI server or Electron process. The UI is a static
Next.js export in the packaged app. See [architecture](docs/architecture.md),
[Git workflow](docs/development/git-workflow.md), and
[privacy snapshot notes](docs/development/public-source-snapshot.md).
Historical reports and anonymized logs retain their dates and evidence limits.
They do not prove the state of a later commit or artifact.

## Attribution and license

Based on the open-source [Meetily project](https://github.com/Zackriya-Solutions/meetily).
Upstream contributor attribution and required notices remain in the source and
[MIT license](LICENSE.md). This repository has no affiliation with upstream's
separately marketed commercial products or release assets.
