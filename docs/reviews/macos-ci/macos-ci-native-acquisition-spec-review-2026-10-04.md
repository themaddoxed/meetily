# macOS CI native acquisition observations — independent SPEC review

**PASS for bounded byte/source-binding and architecture claims. C0 / I0 / M0.** No actionable misstatement found in the scoped evidence. `producer_admitted=false` remains required. Task 0 and CI implementation admission are not complete.

## Exact inputs

Reviewed 2026-10-04 in the root workspace, against supplied base `53f4415058fb01e38b8892a47ac036bfffa394ba`. Before/after census SHA-256:

- `docs/fork/evidence/macos-ci-onnx-source-census-2026-10-04.json`: `8ec371d733fd5fcd71c7d1d76080b91437f3b35de3c39c705ad1f30f958439fb`.
- `docs/fork/evidence/macos-ci-ffmpeg-source-census-2026-10-04.json`: `999f4ac2038cf2423e1284e79f21838ee187b492ba988ed21a1d3562269cd248`.
- CI plan: `d3066f3b42e854e6db3ee571159b7203486a4d025fc23a32f0a1db8cb4dc21c5`.

Review changed_paths: only this report. No product, plans, refs or census flags were changed.

## Independently established checks

Local Python SHA-256/size and archive-member checks passed for all four downloaded packages and corresponding raw files under `/private/tmp/meetily-ci-{onnx,ffmpeg}-source-2026-10-04`. Extracted bytes match the relevant archive member and project manifest; FFmpeg member mode is 0755. Archive member listings contain no LICENSE/NOTICE/COPYING filenames. This establishes actual retained bytes, not merely expected hashes recorded in JSON.

ONNX package hashes match dist.txt. Independently verified cached `ort-sys-2.0.0-rc.10.crate` checksum `e2aba9f5c7c479925205799216e7e5d07cc1d4fa76ea8058c60a9a30f6a4e890` against Cargo.lock; archive dist.txt bytes equal the observed local file (`fb86c7652b69659ad3ec56acc39a59be3ed395bd4a89a3e4430e219333127219`) and contain both URL/digest pairs. Cargo.lock equals base bytes and hash `b209fc096d95592ac0ffa296ce7664e0eda882f24d3434370ccddb37d5b14032`.

FFmpeg build URLs in `frontend/src-tauri/build/ffmpeg.rs:197,200` match the observed release asset URLs; that source equals base bytes (`f10836c35278abda4834b893d72d7d47fdbc8e7a9e1f40e1e57944871c5869fe`). Saved release JSON equals the census release object, including asset IDs, package sizes and digests. Release-observation SHA-256: `88380dc0f15272b1553c4cbf184888dbae9b6f0096e7857d05442347c8694d97`.

Reviewer directly called the existing read-only project verifiers using Node: both `verifyThinOnnxArchive` calls and `verifyUniversalFfmpegSidecars` completed **exit 0**. ONNX inspection uses local `xcrun lipo -archs`; FFmpeg checks Mach-O CPU, executable permission and SHA-256. No downloaded FFmpeg binary was executed. Verifier source hashes match the census: ONNX `9e66f4e858c4a2e1ae08bb6bacdacfadea5a6516c18e8866badabbf57232ec64`; FFmpeg `7b678cd5ee6fce17c242422e0ba594dac9b0877a90e5c14ce12cabba700adf7a`.

Saved acquisition-result JSON hashes also match census references: ONNX `beabaf453e6c2be1ab3782bf4de95fccadefe267ef1f770d229da5877303628c`, FFmpeg `02ab312092eea47c2d78f9dc2f6943dc75fda9ead977a3eb9f9e0e290e7b70fe`.

## Limits and remaining Task 0 requirements

These checks substantiate a temporary acquisition observation and byte/architecture binding. Reviewer did not repeat HTTP requests, independently witness the original empty-directory/download sequence, revalidate current remote release metadata, or admit URL immutability, producer identity or build reproducibility. Recorded public HTTPS locations and package digests do not alone establish producer build/source/license provenance. The upstream ONNX root LICENSE observation does not establish the complete obligations of these distributed archives. No legal-compliance conclusion is made.

Task 0 lines 221–231 still requires approved authenticated immutable acquisition for **all** FFmpeg, ONNX, sherpa-onnx and model inputs, frozen source/hash/size/license/provenance/authentication scope, and missing/corrupt/replayed-input rejection before build. The four inspected packages do not satisfy remaining input families or those negative acquisition cases. Measured package sizes are observations, not an admitted producer manifest.

Runner labels/Xcode/SDKs, actual Intel capacity, both hosted target routes, exact tool versions, reviewed remote action pins, exact clean base/census and fresh independent plan acceptance remain separate admission work. Hosted workflows, protected environments, signing/notarization/draft and exact-artifact native/release gates remain NOT_RUN or unverified here.

No blocker to retaining these censuses as bounded prerequisite evidence. Next action is to resolve producer/build/source/license provenance and the other Task 0 admissions without changing `producer_admitted` to true based on this review.
