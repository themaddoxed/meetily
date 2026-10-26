# FORK-123: recovery transport and asset admission amendment

Status: locked-source review candidate extending recovery contract v3. This
amendment resolves the transport assumptions; implementation is NOT_ADMITTED
until its source and runtime conditions below are satisfied. It does not alter
normal application CSP, native profiles, the identity pair or existing checksums.

## Observed implementation constraints

Exact Cargo.lock pins Tauri 2.11.1 and tauri-codegen 2.6.1; cached crate SHA-256
matches the lock. Source bindings and line locators are retained in
`docs/fork/evidence/webview-recovery-transport-source-2026-10-04.json`.

The resource response hook is only applied to Tauri-protocol responses, not an
external development server. `WebviewUrl::App` resolves through configured
`devUrl` during development; default generated embedded assets are empty when
devUrl is present. Forcing a custom-protocol URL alone does not restore assets.
Tauri asset resolution also falls back through `.html`, `/index.html` and the
normal `index.html`; a successful route response is not proof of the recovery
entrypoint. Recovery must never silently mount that fallback normal application.

The macOS JS transport first fetches `ipc://localhost/<command>`. On custom
protocol or CSP rejection it switches to native `window.ipc.postMessage`, with
`customProtocolIpcBlocked:true`, and subsequent calls use that bridge. Both paths
reach webview command dispatch. Therefore `connect-src 'none'` cannot enforce
command denial. Native sender/state checks are mandatory on either transport.
The deterministic retained template probe confirms only those JS branches with
mocked fetch/native bridge; it is not actual CSP/WKWebView or IPC acceptance.

## Bundled recovery entrypoint admission

Recovery uses the same persistent target WebView origin as bundled normal mode:
macOS `tauri://localhost`, no incognito or label-derived data directory. Both
origins and actual storage continuity must still be tested on the exact native
candidate. No localhost HTTP recovery server or extra application origin is
introduced.

Before window construction, native admission requires an exact embedded
standalone recovery HTML entrypoint, its byte hash, and the finite transitive
set of local script/style/image/font assets produced by that build. The entrypoint
must exclude normal providers, automatic ordinary-service initialization and
normal Next route hydration; do not generate its authority from the current
normal index fallback. An absent entrypoint/dependency or hash mismatch fails
closed before creating a WebView. Reproducibly generate/check the graph during
build; registration must consume that checked graph, never scan a live user path.
No concrete graph/hash is claimed until the candidate has these actual assets.

For recovery window resources, normalize/validate URI once using the exact
Tauri asset-key rules. Reject malformed encodings, dot/alias paths, query-bearing
aliases, unknown authorities and paths absent from the graph. Exact requested
asset bytes/content type/hash must match the graph; fallback body is rejected.
The resource response hook returns an empty non-success response for disallowed
Tauri resources and no executable content. This is asset response filtering,
not a hook for arbitrary external network requests. Native navigation and new
window handlers deny every destination except the admitted recovery entrypoint
(document reload allowed); frontend navigation cannot enter normal routes.
No downloads or external-link opening command is admitted.

Default developer-server mode cannot satisfy this boundary. Recovery startup
there must report unsupported recovery environment without starting normal
services or profile writes. For native recovery testing, use a separately built
bundled-assets candidate; remove devUrl for that candidate and verify embedded
assets/origin/hooks. Do not relax production recovery CSP to accommodate HMR,
WebSocket, remote scripts or the dev server. Such packaging is diagnostic until
all existing release gates succeed.

## Recovery CSP and IPC policy

Install a dedicated response policy before recovery content is parsed. Its
baseline is `default-src 'none'; base-uri 'none'; object-src 'none'; frame-src
'none'; form-action 'none'; connect-src ipc://localhost; img-src 'self';
font-src 'self'; media-src 'none'; worker-src 'none'; script-src 'self';
style-src 'self'`. Asset responses are separately limited to the checked graph.
Inline recovery boot/style code, if generated, must be covered by the exact
build-derived hashes/nonces retained from Tauri's processed response; no
`unsafe-eval`, unrestricted inline source, wildcard, remote asset protocol,
HTTP(S), ws/wss or blanket `ipc:` permission is proposed. Do not mechanically
copy this baseline over Tauri's script hash/nonce directive and break startup.
The final effective header plus every HTML meta policy must be checked for
intersection and compatibility. Policy generation validates actual source terms;
missing/unexpected terms fail admission before window creation.

Primary IPC is the exact `ipc://localhost` custom endpoint. The native bridge
fallback remains possible and has the same sender/state authority; it is not
blocked by CSP and must not be treated as an isolation bypass that CSP solves.
Both paths must reject forged ordinary commands, payload window labels and
unsupported recovery-state transitions. The scoped contract-v3 command allowlist
still applies. No generic frontend fs/dialog/store/notification/window/app/process
permissions are granted. Native bounded selection retains its opaque-token API.
Ordinary egress suppression requires actual WKWebView observations of fetch,
WebSocket, images/fonts/scripts, workers, navigation and asset requests; a template
probe or Chromium-only policy exercise cannot close this native gate.

## Internal channel exception and lifecycle invariant

Tauri registers `__TAURI_CHANNEL__` internally and manages its data cache before
application setup. `plugin:__TAURI_CHANNEL__|fetch` is exempted from the usual ACL
rejection; the channel fetch removes by numeric header ID from a shared managed
cache and receives no invoking-webview argument. Removing application plugin
registration or assigning empty recovery capabilities does not disable it.
Do not claim all plugin commands are denied solely by capabilities.

Recovery admission therefore requires one WebView and a fresh process before
any ordinary command/provider/service/channel-producing response can occur.
All admitted custom commands check actual sender and native state before work,
including commands that could return large/raw data. The cache may contain only
responses emitted by admitted recovery work in that process. Core ordinary
plugins remain capability-denied; no ordinary window, events, model notifications
or previous normal-process cache can coexist with recovery. Completion requires
native restart into normal mode; never convert an existing normal process/window
into recovery, or recovery into normal mode while keeping the process/cache.
Single-instance focus selects only the currently admitted window and cannot
construct a second normal one. These conditions must be enforced in startup
code and tested; UI labels or frontend promises are insufficient.

The internal channel is a retained transport exception, not an additional
application command with migration authority. Unknown/malformed cache IDs fail
without data; own recovery-response retrieval may work. Actual tests must show
that no ordinary data is seeded before recovery and that a rejected ordinary
command cannot populate the queue. If any ordinary producer can coexist, this
shared-cache invariant fails admission; then a supported sender-bound channel
strategy must be designed and verified before recovery can ship. No dependency
patch, plugin removal or channel-cache access API is authorized by this document.

## Required evidence and remaining work

Implement/verify the exact bundled asset graph and effective CSP, native request
sender/state guards across custom-protocol and bridge paths, startup exclusivity
and channel-cache provenance. Retain tests for Tauri index fallback rejection,
missing assets, denied ordinary plugin/custom commands and failed bootstrap.
Prove actual persistent origin/store continuity through restart, interruption and
reopen. Pair/producer admission, finite DTO validation, first-intent authority,
clean QA-integrated implementation source and real FS/SQLite/IDB interruption
checks remain separate open conditions. No FORK-123 or release-ready claim follows.
