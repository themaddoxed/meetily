use app_lib::audio::incremental_saver::{cleanup_checkpoints, recover_audio_from_checkpoints};
use app_lib::audio::recording_saver::{RecordingSaver, TranscriptSegment};
#[cfg(target_os = "macos")]
use app_lib::audio::{
    incremental_saver::IncrementalAudioSaver,
    recording_state::{AudioChunk, DeviceType},
};
use serde_json::Value;
use std::path::Path;

fn transcript_segment(sequence_id: u64, text: &str) -> TranscriptSegment {
    let start = sequence_id as f64;
    TranscriptSegment {
        id: format!("segment-{sequence_id}"),
        text: text.to_string(),
        audio_start_time: start,
        audio_end_time: start + 1.0,
        duration: 1.0,
        display_time: format!("[00:{sequence_id:02}]"),
        confidence: 0.95,
        sequence_id,
    }
}

fn read_transcript(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).expect("read persisted transcript"))
        .expect("parse persisted transcript")
}

fn initialize_transcript_only_saver(root: &Path, meeting_name: &str) -> RecordingSaver {
    let mut saver = RecordingSaver::new();
    saver.set_save_folder(root.to_path_buf());
    saver.set_meeting_name(Some(meeting_name.to_string()));
    let sender = saver
        .start_accumulation(false)
        .expect("initialize meeting folder");
    drop(sender);
    saver
}

#[tokio::test]
async fn adding_segments_writes_complete_transcripts_json() {
    let root = tempfile::tempdir().expect("temporary recording root");
    let saver = initialize_transcript_only_saver(root.path(), "Incremental transcript");
    let meeting_folder = saver.get_meeting_folder().expect("meeting folder").clone();

    saver.add_transcript_segment(transcript_segment(1, "First persisted segment"));
    saver.add_transcript_segment(transcript_segment(2, "Second persisted segment"));

    let transcript_path = meeting_folder.join("transcripts.json");
    let persisted = read_transcript(&transcript_path);
    assert_eq!(persisted["total_segments"], 2);
    assert_eq!(persisted["segments"][0]["text"], "First persisted segment");
    assert_eq!(persisted["segments"][1]["text"], "Second persisted segment");
    assert!(!meeting_folder.join(".transcripts.json.tmp").exists());
}

#[tokio::test]
async fn successfully_written_segments_survive_saver_drop_and_file_reopen() {
    let root = tempfile::tempdir().expect("temporary recording root");
    let meeting_folder = {
        let saver = initialize_transcript_only_saver(root.path(), "Crash recovery");
        let meeting_folder = saver.get_meeting_folder().expect("meeting folder").clone();
        saver.add_transcript_segment(transcript_segment(1, "Persist before process exit"));
        saver.add_transcript_segment(transcript_segment(2, "Persist every completed write"));
        meeting_folder
    };

    let persisted = read_transcript(&meeting_folder.join("transcripts.json"));
    let segments = persisted["segments"]
        .as_array()
        .expect("persisted segments");
    assert_eq!(segments.len(), 2);
    assert_eq!(segments[0]["sequence_id"], 1);
    assert_eq!(segments[1]["sequence_id"], 2);
}

#[tokio::test]
async fn failed_incremental_write_preserves_previous_valid_transcript_file() {
    let root = tempfile::tempdir().expect("temporary recording root");
    let saver = initialize_transcript_only_saver(root.path(), "Atomic transcript");
    let meeting_folder = saver.get_meeting_folder().expect("meeting folder").clone();
    let transcript_path = meeting_folder.join("transcripts.json");

    saver.add_transcript_segment(transcript_segment(1, "Known good transcript"));
    let known_good = std::fs::read(&transcript_path).expect("known-good transcript bytes");

    let temp_path = meeting_folder.join(".transcripts.json.tmp");
    std::fs::create_dir(&temp_path).expect("block temporary-file write with a directory");
    saver.add_transcript_segment(transcript_segment(2, "Write must fail safely"));

    assert_eq!(
        std::fs::read(&transcript_path).expect("previous transcript still readable"),
        known_good
    );
    let persisted = read_transcript(&transcript_path);
    assert_eq!(persisted["total_segments"], 1);
    assert_eq!(persisted["segments"][0]["text"], "Known good transcript");
}

#[tokio::test]
async fn recovery_without_audio_chunks_preserves_neighboring_meeting_files() {
    let root = tempfile::tempdir().expect("temporary meeting folder");
    let meeting_folder = root.path();
    let checkpoints = meeting_folder.join(".checkpoints");
    std::fs::create_dir(&checkpoints).expect("checkpoint directory");
    std::fs::write(meeting_folder.join("metadata.json"), b"metadata").expect("metadata fixture");
    std::fs::write(meeting_folder.join("transcripts.json"), b"transcript")
        .expect("transcript fixture");

    let status =
        recover_audio_from_checkpoints(meeting_folder.to_string_lossy().into_owned(), 48_000)
            .await
            .expect("empty recovery is a valid no-op");

    assert_eq!(status.status, "none");
    assert_eq!(
        std::fs::read(meeting_folder.join("metadata.json")).unwrap(),
        b"metadata"
    );
    assert_eq!(
        std::fs::read(meeting_folder.join("transcripts.json")).unwrap(),
        b"transcript"
    );
    assert!(checkpoints.is_dir());
}

#[tokio::test]
#[cfg(target_os = "macos")]
#[ignore = "requires packaged FFmpeg integration environment"]
async fn successful_checkpoint_recovery_preserves_neighboring_meeting_files() {
    let root = tempfile::tempdir().expect("temporary meeting folder");
    let meeting_folder = root.path();
    let checkpoints = meeting_folder.join(".checkpoints");
    std::fs::create_dir(&checkpoints).expect("checkpoint directory");
    std::fs::write(meeting_folder.join("metadata.json"), b"metadata").expect("metadata fixture");
    std::fs::write(meeting_folder.join("transcripts.json"), b"transcript")
        .expect("transcript fixture");

    let mut saver = IncrementalAudioSaver::new(meeting_folder.to_path_buf(), 48_000)
        .expect("incremental saver");
    saver
        .add_chunk(AudioChunk {
            data: vec![0.1; 48_000 * 30],
            sample_rate: 48_000,
            timestamp: 0.0,
            chunk_id: 0,
            device_type: DeviceType::Microphone,
            span: None,
        })
        .expect("write an actual audio checkpoint");
    assert_eq!(saver.get_checkpoint_count(), 1);
    drop(saver);

    let status =
        recover_audio_from_checkpoints(meeting_folder.to_string_lossy().into_owned(), 48_000)
            .await
            .expect("recover checkpoint audio");

    assert_eq!(status.status, "success");
    assert_eq!(status.chunk_count, 1);
    assert!(meeting_folder.join("audio.mp4").is_file());
    assert_eq!(
        std::fs::read(meeting_folder.join("metadata.json")).unwrap(),
        b"metadata"
    );
    assert_eq!(
        std::fs::read(meeting_folder.join("transcripts.json")).unwrap(),
        b"transcript"
    );
}

#[tokio::test]
async fn checkpoint_cleanup_removes_only_checkpoint_directory() {
    let root = tempfile::tempdir().expect("temporary meeting folder");
    let meeting_folder = root.path();
    let checkpoints = meeting_folder.join(".checkpoints");
    std::fs::create_dir(&checkpoints).expect("checkpoint directory");
    std::fs::write(checkpoints.join("audio_chunk_000.mp4"), b"checkpoint")
        .expect("checkpoint fixture");
    std::fs::write(meeting_folder.join("metadata.json"), b"metadata").expect("metadata fixture");
    std::fs::write(meeting_folder.join("transcripts.json"), b"transcript")
        .expect("transcript fixture");

    cleanup_checkpoints(meeting_folder.to_string_lossy().into_owned())
        .await
        .expect("checkpoint cleanup");

    assert!(!checkpoints.exists());
    assert_eq!(
        std::fs::read(meeting_folder.join("metadata.json")).unwrap(),
        b"metadata"
    );
    assert_eq!(
        std::fs::read(meeting_folder.join("transcripts.json")).unwrap(),
        b"transcript"
    );
}
