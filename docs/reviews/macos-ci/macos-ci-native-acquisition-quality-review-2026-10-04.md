# Native acquisition evidence — independent QUALITY review

Date: 2026-10-04. Status: **PASS for bounded acquisition-evidence fidelity; C0 / I0 / M0.** No actionable findings remain in this scope. CI prerequisite admission remains incomplete.

## Scope and observed snapshot

Reviewed the two root census documents, their retained temporary acquisition observations and downloaded packages, locked ONNX distribution binding, FFmpeg source URL binding, and the project architecture/hash verifiers. Base recorded by this task is `53f4415058fb01e38b8892a47ac036bfffa394ba`; this is a review of exact local evidence bytes, not acceptance of the dirty root or a release revision. Product, Git refs, plans, packages, and installed state were not changed by this review. `changed_paths` consists only of this report.

| Evidence | SHA-256 |
| --- | --- |
| `docs/fork/evidence/macos-ci-onnx-source-census-2026-10-04.json` | `8ec371d733fd5fcd71c7d1d76080b91437f3b35de3c39c705ad1f30f958439fb` |
| `docs/fork/evidence/macos-ci-ffmpeg-source-census-2026-10-04.json` | `999f4ac2038cf2423e1284e79f21838ee187b492ba988ed21a1d3562269cd248` |
| ONNX temporary `fresh-acquisition-results.json` | `beabaf453e6c2be1ab3782bf4de95fccadefe267ef1f770d229da5877303628c` |
| FFmpeg temporary `fresh-acquisition-results.json` | `02ab312092eea47c2d78f9dc2f6943dc75fda9ead977a3eb9f9e0e290e7b70fe` |
| FFmpeg temporary `release-observation.json` | `88380dc0f15272b1553c4cbf184888dbae9b6f0096e7857d05442347c8694d97` |
| FFmpeg temporary `producer-repository-observation.json` | `493a24737997f294cc00b17387e5b5ada558a051dc463384d62a2b44ae7a53cd` |

Temporary evidence directories are `/private/tmp/meetily-ci-onnx-source-2026-10-04` and `/private/tmp/meetily-ci-ffmpeg-source-2026-10-04`. Both census hashes were checked again after inspection and remained unchanged.

## Independent checks

Local Python `hashlib`, `tarfile`, and `zipfile` reads confirmed all four package sizes and SHA-256 values, and streamed archive member hashes matched the separately retained raw files. These reads did not extract or execute downloaded code.

| Artifact | Package bytes / SHA-256 | Raw member bytes / SHA-256 |
| --- | --- | --- |
| ONNX aarch64 | 19,828,171 / `00fbfd6f08bac2a4e28c66723af900d58d1b4b1c73efba6290637cd3019883d5` | 71,060,144 / `e5c83560aa9e88afa39d9dca9fb5f5a767e28adb5458d1c36fe0357131b6af8b` |
| ONNX x86_64 | 20,512,981 / `e0538783248bbb77d2e97556134b8cc47cfc35ac3d762067d51eaf8355c0f48e` | 75,893,736 / `f41370c27dd380419c6cbdfe4f78238613a49506c8f491c0cb07084edac9d561` |
| FFmpeg aarch64 | 22,400,365 / `0d4efcaf6a098430a708e0af694a84792938921fa126162787ae98c6151d7a95` | 51,551,944 / `77d2c853f431318d55ec02676d9b2f185ebfdddb9f7677a251fbe453affe025a` |
| FFmpeg x86_64 | 25,809,598 / `470e482f6e290eac92984ac12b2d67bad425b1e5269fd75fb6a3536c16e824e4` | 80,083,328 / `430d60fbf419dab28daee9b679e7929a31ee9bae53f6e42e8ae26b725584290f` |

Each ONNX archive contains only `onnxruntime/lib/libonnxruntime.a`. Both FFmpeg archives contain `ffmpeg`; the ARM zip also contains the small `__MACOSX/._ffmpeg` metadata member. No archive includes a LICENSE, NOTICE, or COPYING member.

The cached `ort-sys-2.0.0-rc.10.crate` SHA-256 is `e2aba9f5c7c479925205799216e7e5d07cc1d4fa76ea8058c60a9a30f6a4e890`, matching Cargo.lock. Its archived `dist.txt` equals the observed distribution file (SHA-256 `fb86c7652b69659ad3ec56acc39a59be3ed395bd4a89a3e4430e219333127219`), whose two `none` platform entries bind the observed package hashes and URLs. Root Cargo.lock SHA-256 `b209fc096d95592ac0ffa296ce7664e0eda882f24d3434370ccddb37d5b14032` also matched the supplied base bytes. This establishes the locked distribution selection, not complete producer or license provenance.

The two FFmpeg package sizes/digests match the retained GitHub release observation. Its release and producer objects equal the census objects. `frontend/src-tauri/build/ffmpeg.rs` SHA-256 `f10836c35278abda4834b893d72d7d47fdbc8e7a9e1f40e1e57944871c5869fe` binds the two selected download URLs. The retained README bytes match Git blob `80acb1693912f167de1a0d087760e465bc071e13`. The observation supports only the recorded repository root listing, not a claim that no build sources exist anywhere in its history.

Own read-only verifier execution on local macOS completed with exit 0:

- `node frontend/scripts/verify-universal-ffmpeg.js /private/tmp/meetily-ci-ffmpeg-source-2026-10-04`: both thin binaries passed expected SHA, executable-file, and Mach-O CPU-header checks.
- A Node loop calling `verifyThinOnnxArchive` for both entries of `ONNX_RUNTIME_MANIFEST` against the temporary directory: both passed expected SHA and `xcrun lipo -archs` checks for exactly one expected architecture.

Verifier source hashes inspected: `verify-universal-ffmpeg.js` = `7b678cd5ee6fce17c242422e0ba594dac9b0877a90e5c14ce12cabba700adf7a`; `macos-sidecar-tools.js` = `eab59aa0c6f1a548ca44704832847bb26cbee8067481e7857c35321a9e773970`; `universal-onnx-runtime.js` = `9e66f4e858c4a2e1ae08bb6bacdacfadea5a6516c18e8866badabbf57232ec64`. Only the read-only validation paths were invoked. Successful architecture parsing is not runtime validation of every code path or a build provenance attestation.

## Findings, blockers, and next action

No confirmed correctness, maintainability, security-relevant, or evidence-overclaim defect was found in these census documents. They explicitly retain producer/license admission as false or incomplete and hosted execution as not run. The separately observed upstream ONNX MIT root license does not establish the complete licensing or notices required for these particular distributed archives.

**NOT VERIFIED:** independent fresh network acquisition or current remote state; authenticated/reproducible producer build lineage; complete source and third-party license/notice provenance; full universal assembly; runtime functionality; actual Intel/ARM hosted jobs and capacity; signing, installation, native smoke, or release readiness. Owner-recorded HTTP/API observations were checked against retained local evidence, not repeated as reviewer network requests.

Next action: resolve and review build/source/license admission and the remaining CI execution prerequisites before admitting these inputs to the CI implementation. This report closes only independent review of the recorded acquisition bindings; it does not close FORK-145 or any release gate.
