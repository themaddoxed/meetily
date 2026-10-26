> Public privacy copy: local user paths and contact/provenance fields were anonymized. Historical test results and original hashes do not verify this edited copy. See [publication notes](/docs/development/public-source-snapshot.md).

# FORK-123 recovery transport — independent bounded SPEC review

Status: CHANGES_REQUIRED, C0 / I1 / M0. Implementation remains NOT_ADMITTED. This is a locked-source design review, not proof of native isolation or a complete executable recovery plan.

## Important finding

**I1 — Freeze pre-processing versus response-byte verification before permitting nonce-bearing HTML.** `docs/superpowers/plans/2026-10-04-webview-recovery-transport.md:38–51` requires a build-produced graph with exact byte hashes and requires requested response bytes/hash to match it. Lines 73–78 nevertheless permit inline assets with hashes/nonces retained from Tauri's processed response. These are incompatible for the nonce-bearing branch as currently specified.

Pinned `tauri-2.11.1/src/manager/mod.rs:437–454` transforms HTML with `set_csp` before returning the asset. `replace_csp_nonce` at lines 126–150 replaces each nonce token with a fresh random u64 string and adds matching CSP sources. `src/protocol/tauri.rs:224–240` constructs the processed body/header before calling the resource hook. Thus the legitimate HTML observed by the hook can differ on each request/reload from the build bytes; a fixed graph digest cannot validate it. Conversely, ignoring mismatches would violate the promised exact-byte/fallback protection. The future graph is correctly left unclaimed, but the specified comparison contract still needs a choice before implementation.

Resolve narrowly: either require a nonce-free recovery asset pipeline (prove the admitted HTML contains no runtime nonce substitution tokens; allow exact static script hashes and external local assets), or define distinct authenticated embedded-template hashes and exact processed-response verification, permitting only validated nonce substitutions with one-to-one CSP binding and no other byte changes. Do not normalize arbitrary HTML differences. Require reload coverage plus rejection of altered executable bytes/fallback HTML in the chosen path. Classification: contradictory/missing verification contract, not a current product vulnerability or a demand for existing implementation.

## Other conclusions

The resource hook's external-devURL exclusion, codegen's empty default assets under devUrl and asset index fallback are accurately stated. A bundled checked recovery graph and fail-closed developer-server mode are consistent with those sources; forcing a custom URL alone would not suffice.

Pinned IPC templates confirm custom-protocol fetch then native postMessage fallback, including the persistent fallback flag. CSP alone therefore cannot authorize commands. The internal channel-fetch ACL exception and shared numeric-ID cache are real; the proposed fresh recovery-only process, single WebView, sender/state-gated admitted producers and restart-only transition address that threat as a conditional lifecycle invariant. The document correctly requires native proof instead of claiming empty capabilities disable internal channel transport. No additional actionable finding was found in this bounded policy.

## Snapshot and checks

| Input | SHA-256 |
| --- | --- |
| docs/superpowers/plans/2026-10-04-webview-recovery-transport.md | 929bdc73d043b78a8971a9a2f5dd33a00f7f1571e6b65cc56fe24e1154937715 |
| docs/fork/evidence/webview-recovery-transport-source-2026-10-04.json | b2c37912d54d4ad264b69b621d798cdfdc8763cfe05bb69fffe644653fc70d2a |
| Cargo.lock | b209fc096d95592ac0ffa296ce7664e0eda882f24d3434370ccddb37d5b14032 |
| Cached tauri 2.11.1 crate | b93bd86d231f0a8138f11a02a584769fe4b703dc36ae133d783228dbc4801405 |
| Cached tauri-codegen 2.6.1 crate | 6bd11644962add2549a60b7e7c6800f17d7020156e02f516021d8103e80cc528 |

Reviewer independently checked both cache archives against Cargo.lock and all ten extracted-source SHA records against the evidence. Relevant exact sources include:

- `/Users/developer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tauri-2.11.1/src/webview/mod.rs`: `954cf212cb79d6b566c10b76d2ff2da05bc637cfc9f666c2f03d4e3dddf35f1c`.
- `/Users/developer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tauri-2.11.1/src/manager/webview.rs`: `5b2610ddf033a646ab3e62ca6403d548ab111d9d1df967b81132f8bf17a87ff0`.
- `/Users/developer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tauri-2.11.1/src/manager/mod.rs`: `3b35d04bded3a068d3d9e1737d52a66a69ccfbc4b2300ec00ed6a5deaa85c79a`.
- `/Users/developer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tauri-2.11.1/src/protocol/tauri.rs`: `393cf458c003d72d1505b7438fa4d36ee3bf3dc8a3f1480705dc3c81685297b1`.
- `/Users/developer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tauri-2.11.1/src/ipc/protocol.rs`: `000d8a26eb5aa181e32c8b7631410c184941624fa34585d68cbc932abb8d45d6`.
- `/Users/developer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tauri-2.11.1/src/ipc/channel.rs`: `8533436b36afb959401052ab0bd7990c63817e9d984fe0daa49a80bc6ed35e5f`.
- `/Users/developer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tauri-2.11.1/src/app.rs`: `d2bbeff336c5699ca74b1455ed7575b48ad3315e2266b9871e8355cfab6e61c1`.
- `/Users/developer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tauri-2.11.1/scripts/ipc-protocol.js`: `68ae690606006f733bde75dd8bb6748464954a22c7e83914a93b1abc221d7072`.
- `/Users/developer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tauri-2.11.1/scripts/process-ipc-message-fn.js`: `d51917f89e4d3d318d63f1f4f5a7ac7ea1399a5b85c81a840c4d606597f29132`.
- `/Users/developer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tauri-codegen-2.6.1/src/context.rs`: `ea13b70394f0166813d3aa1938d423ddd3ed809f81ecf541586c2f32f15255bd`.

The Node VM probe script and result were read, and their hashes matched evidence: script `5b5e8b2bf454564eab314d491d6c5cafd7805f77ae45cb9f4abb2d45d8387900`, result `30b3c283522959468beb91f4233c7a1b4bc73ba5bdde41eac87945f4af74240e`. Its two scenarios assert mock fetch success and mock rejection followed by two native-bridge messages; result reports exit 0. It exercises actual retained JS templates with mocked transport, not real CSP evaluation, sender validation, native IPC, response filtering or egress isolation. Reviewer did not rerun it.

Reviewer actions: `shasum`, targeted `sed`/`rg`/`nl` source reads and Python SHA-256/TOML comparison on local macOS. Initial lock-path lookup was corrected to root Cargo.lock. Plan/evidence hashes were unchanged at final check. changed_paths: only this report; product/plans/Git/user profiles untouched.

Next action: resolve I1, then retain the document's open asset-build, effective-CSP, actual WKWebView origin/store continuity, both IPC transport guard, producer/cache exclusivity, navigation/egress and restart/interruption tests. Clean implementation source, actual identity pair/producer and schema/FS/SQLite/IDB gates remain open. No FORK-123, combined-protocol, installed migration or release-ready PASS is given.
