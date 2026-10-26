# macOS platform support

The product build targets one Universal macOS application with native
`x86_64` and `arm64` slices and a configured minimum of macOS 14.2 Sonoma.
The public source snapshot is not a distributed or accepted application release;
runtime support must be proven by the evidence below.

Support below macOS 14.2 is not claimed. Lower versions would require a
separate system-audio fallback and a new product decision.

## Evidence required

Architecture slices are build evidence only. They show that the required code
was packaged, not that the application works on both processor families.

Runtime support requires separate physical smoke evidence on Intel and Apple
Silicon for the same exact source commit and artifact. The macOS 14.2 lower
boundary also requires its own smoke evidence; a pass on a newer macOS version
does not prove the lower boundary.

The existing diagnostic DMG is Universal, but advertises an incorrect macOS
10.13 minimum. It does not comply with this policy. Apple Silicon runtime and
runtime on macOS 14.2 are not yet proven.

## Intel retirement review

Intel support has no scheduled retirement date or automatic usage threshold.
Review it when real processor-usage data becomes available or when the Rust,
Tauri, Xcode, or required native dependencies materially change their
`x86_64-apple-darwin` support. Any retirement still requires an explicit
product decision based on both user impact and the supported toolchain.
