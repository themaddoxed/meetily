# Current CI and packaging workflows

`ci.yml` automatically runs on pushes and pull requests targeting `main` or
`dev`. It calls `quality-gate.yml` with read-only repository permissions.

The reusable gate checks:

- committed/source whitespace (including the complete tree for an initial push),
- frontend policy, lint, types, unit tests, browser contracts and static export,
- native Rust/SQLite/data-integrity tests on a macOS runner,
- an ad-hoc macOS app for inspection after frontend and Rust jobs pass,
- an always-running fan-in that requires every prerequisite job to succeed.

The source/frontend/fan-in jobs use Ubuntu for host tooling. This runner choice
is not a claim of a desktop product for that operating system. The product
platform policy remains [macOS](../../docs/fork/platform-support.md).

`release.yml` is manually dispatched, calls the same quality gate and requires
exact manual attestation. It creates only a draft; the complete artifact and
trust requirements are in [testing.md](../../docs/testing.md).

Other `build-*.yml` and legacy validation workflows are retained manual upstream
utilities. Their presence does not qualify the supported product, an installer,
a foreign-platform target, signing, notarization or runtime acceptance. Use the
current CI/candidate path above for product evidence.
