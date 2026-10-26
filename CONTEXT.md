# Meetily macOS Fork Domain Context

This glossary defines the shared domain language for the macOS fork. The legacy product identity is provisional and does not define the final public identity.

## Language

| Term | Meaning |
| --- | --- |
| Meeting | The durable aggregate containing metadata and references to capture and processing outputs. |
| Recording Session | One attempt to capture microphone and system-audio tracks for a Meeting, with one stable identity and one lifecycle. |
| Processing Job | One background execution of transcribe, diarize, or summarize that belongs to one Meeting and has one stable identity and lifecycle. |
| Track | One independently durable audio stream; microphone and system audio remain separate Tracks until an explicit downstream operation combines them. |
| Snapshot | The complete authoritative view of one Recording Session or Processing Job at a sequence watermark. |
| Ordered Update | One message for exactly one Recording Session or Processing Job, carrying a strictly increasing sequence. |
| Recording Root | A user-selected external directory whose absolute location is configuration rather than Meeting metadata. |
| Relative Recording Path | A normalized path below a Recording Root that is never absolute and never contains `..`. |
| Model Artifact | A versioned local model file identified by its checksum, architecture support, installation state, and source. |
| Capability | A runtime or build fact stating that an operation can execute on the current architecture; it is not a user preference. |
