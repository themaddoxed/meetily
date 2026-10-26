# macOS acceleration

The current Rust manifest selects `whisper-rs` with `metal` and `coreml` on macOS.
The local-model helper has an optional `metal` feature, enabled explicitly by the
host-helper build in [BUILDING.md](BUILDING.md) and CI. Parakeet has its own ONNX
runtime path; its availability and performance require model/runtime checks.

Runtime selection in `whisper_engine/acceleration.rs` chooses CPU on Intel and
Metal on arm64. A compiled Core ML feature is not proof of an active Core ML
model/backend.

A compiled feature does not prove that inference used a GPU, that it is faster,
or that both CPU architectures work on an installed artifact. Those claims need
measurements and exact-artifact acceptance.

The old automatic GPU scripts and CUDA/Vulkan/OpenBLAS package commands are not
the supported configuration of this macOS product. Use the existing manifests
and [testing.md](testing.md); retain reproducible build and model evidence before
changing acceleration features.
