# Meetily offline speaker diarization notices

Meetily 0.4.1 Alpha includes local-only speaker diarization components. The
corresponding model and native binary checksums are pinned in
`frontend/scripts/diarization-artifacts.js`.

## sherpa-onnx 1.13.2

- Project: https://github.com/k2-fsa/sherpa-onnx
- Pinned release: https://github.com/k2-fsa/sherpa-onnx/releases/tag/v1.13.2
- License: Apache License 2.0 (`licenses/Apache-2.0.txt`)

## pyannote segmentation 3.0 ONNX export

- Export repository: https://huggingface.co/csukuangfj/sherpa-onnx-pyannote-segmentation-3-0
- Pinned revision: `340b52f1f5cd12d45a30fa284691417eaad2ff92`
- Original model: https://huggingface.co/pyannote/segmentation-3.0
- License: MIT (`licenses/MIT-pyannote.txt`)
- Copyright: 2022 CNRS

## 3D-Speaker ERes2Net embedding model

- Project: https://github.com/modelscope/3D-Speaker
- License revision: `065629c313eaf1a01c65c640c46d77e61e9607b4`
- Model distributed by sherpa-onnx:
  `3dspeaker_speech_eres2net_base_sv_zh-cn_3dspeaker_16k.onnx`
- License: Apache License 2.0 (`licenses/Apache-2.0.txt`)

These components run on the user's device. Meetily does not upload meeting
audio to use them.
