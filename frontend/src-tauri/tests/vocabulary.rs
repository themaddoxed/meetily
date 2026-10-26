use app_lib::api::TranscriptSegment;
use app_lib::audio::transcription::{TranscriptResult, TranscriptionProvider, WhisperProvider};
use app_lib::database::repositories::setting::{merge_vocabulary, SettingsRepository};
use app_lib::database::repositories::transcript::TranscriptsRepository;
use app_lib::whisper_engine::WhisperEngine;
use sqlx::migrate::Migrator;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::TempDir;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

struct TestDatabase {
    _directory: TempDir,
    path: PathBuf,
    pool: SqlitePool,
}

impl TestDatabase {
    async fn migrated() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("vocabulary.sqlite");
        let pool = open_pool(&path).await;
        MIGRATOR.run(&pool).await.unwrap();
        Self {
            _directory: directory,
            path,
            pool,
        }
    }

    async fn reopen(&mut self) {
        self.pool.close().await;
        self.pool = open_pool(&self.path).await;
        MIGRATOR.run(&self.pool).await.unwrap();
    }
}

async fn open_pool(path: &Path) -> SqlitePool {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .foreign_keys(true);
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap()
}

#[tokio::test]
async fn vocabulary_survives_reopen_and_meeting_empty_override_inherits_global() {
    let mut database = TestDatabase::migrated().await;
    sqlx::query(
        "INSERT INTO meetings (id, title, created_at, updated_at)
         VALUES ('meeting-1', 'Vocabulary meeting', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
    )
    .execute(&database.pool)
    .await
    .unwrap();

    SettingsRepository::save_global_vocabulary(&database.pool, Some("Global, Shared"))
        .await
        .unwrap();
    SettingsRepository::save_meeting_vocabulary(
        &database.pool,
        "meeting-1",
        Some("Meeting, shared"),
    )
    .await
    .unwrap();

    assert_eq!(
        SettingsRepository::get_effective_vocabulary(&database.pool, Some("meeting-1"))
            .await
            .unwrap(),
        Some("Meeting, shared, Global".to_string())
    );

    database.reopen().await;
    assert_eq!(
        SettingsRepository::get_effective_vocabulary(&database.pool, Some("meeting-1"))
            .await
            .unwrap(),
        Some("Meeting, shared, Global".to_string())
    );

    SettingsRepository::save_meeting_vocabulary(&database.pool, "meeting-1", Some(""))
        .await
        .unwrap();
    assert_eq!(
        SettingsRepository::get_effective_vocabulary(&database.pool, Some("meeting-1"))
            .await
            .unwrap(),
        Some("Global, Shared".to_string())
    );
}

#[tokio::test]
async fn empty_global_vocabulary_deletes_the_saved_value() {
    let database = TestDatabase::migrated().await;
    SettingsRepository::save_global_vocabulary(&database.pool, Some("Acme"))
        .await
        .unwrap();
    SettingsRepository::save_global_vocabulary(&database.pool, Some("\n,  "))
        .await
        .unwrap();

    assert_eq!(
        SettingsRepository::get_global_vocabulary(&database.pool)
            .await
            .unwrap(),
        None
    );
}

fn test_segment(text: &str) -> TranscriptSegment {
    TranscriptSegment {
        id: "segment-1".into(),
        text: text.into(),
        timestamp: "2026-09-08T09:00:00Z".into(),
        audio_start_time: Some(0.0),
        audio_end_time: Some(1.0),
        duration: Some(1.0),
        speaker_id: None,
        speaker_label: None,
        speaker_color: None,
        is_overlap: Some(false),
        diarization_status: Some("none".into()),
        diarization_method: None,
        diarization_confidence: None,
    }
}

#[tokio::test]
async fn recording_vocabulary_uses_the_same_id_from_start_snapshot_to_retranscription() {
    let database = TestDatabase::migrated().await;
    let meeting_id = "meeting-recording-identity";

    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM meetings WHERE id = ?")
            .bind(meeting_id)
            .fetch_one(&database.pool)
            .await
            .unwrap(),
        0
    );

    let saved_id = TranscriptsRepository::save_transcript_with_id_and_started_at_and_vocabulary(
        &database.pool,
        meeting_id,
        "Recorded meeting",
        &[test_segment("captured")],
        Some("/tmp/recording-identity".into()),
        Some(1_725_000_123_456),
        Some("Xantoris, Neworon"),
    )
    .await
    .unwrap();
    assert_eq!(saved_id, meeting_id);
    assert_eq!(
        SettingsRepository::get_effective_vocabulary(&database.pool, Some(meeting_id))
            .await
            .unwrap(),
        Some("Xantoris, Neworon".to_string())
    );

    // Retranscription reads the same row that the stop path created; it does
    // not need a second generated UUID.
    assert_eq!(
        SettingsRepository::get_meeting_vocabulary(&database.pool, meeting_id)
            .await
            .unwrap(),
        Some("Xantoris, Neworon".to_string())
    );

    // Empty vocabulary still leaves no vocabulary row and does not require a
    // pre-existing meetings row before the stop transaction.
    let empty_id = TranscriptsRepository::save_transcript_with_id_and_started_at_and_vocabulary(
        &database.pool,
        "meeting-empty-identity",
        "Empty vocabulary meeting",
        &[test_segment("captured")],
        None,
        None,
        None,
    )
    .await
    .unwrap();
    assert_eq!(
        SettingsRepository::get_meeting_vocabulary(&database.pool, &empty_id)
            .await
            .unwrap(),
        None
    );
}

fn read_mono_16k_wav(path: &Path) -> Vec<f32> {
    let mut reader = hound::WavReader::open(path).expect("open vocabulary WAV");
    let spec = reader.spec();
    assert_eq!(spec.channels, 1, "the opt-in fixture must be mono");
    assert_eq!(
        spec.sample_rate, 16_000,
        "the opt-in fixture must be 16 kHz"
    );
    assert_eq!(spec.sample_format, hound::SampleFormat::Int);

    reader
        .samples::<i16>()
        .map(|sample| sample.expect("read vocabulary WAV sample") as f32 / i16::MAX as f32)
        .collect()
}

fn print_runtime_result(label: &str, result: &TranscriptResult) {
    println!(
        "{label}: confidence={:?}, partial={}, text={:?}",
        result.confidence, result.is_partial, result.text
    );
}

/// Runs the real Whisper model through the provider adapter and the direct
/// retranscription engine seam with the same resolved vocabulary. This is
/// intentionally opt-in because it needs a large model file and takes several
/// minutes on CPU. It does not stand in for a full Tauri recording-session
/// or retranscription-command run.
#[tokio::test]
#[ignore = "requires MEETILY_VOCABULARY_MODEL_PATH and MEETILY_VOCABULARY_AUDIO_PATH"]
async fn vocabulary_runtime_paths_use_the_same_resolved_terms() {
    let model_path = std::env::var_os("MEETILY_VOCABULARY_MODEL_PATH")
        .map(PathBuf::from)
        .expect("set MEETILY_VOCABULARY_MODEL_PATH to an existing ggml model");
    let audio_path = std::env::var_os("MEETILY_VOCABULARY_AUDIO_PATH")
        .map(PathBuf::from)
        .expect("set MEETILY_VOCABULARY_AUDIO_PATH to an existing mono 16 kHz WAV");
    assert!(
        model_path.is_file(),
        "model path does not exist: {}",
        model_path.display()
    );
    assert!(
        audio_path.is_file(),
        "audio path does not exist: {}",
        audio_path.display()
    );

    let model_name =
        std::env::var("MEETILY_VOCABULARY_MODEL_NAME").unwrap_or_else(|_| "base".into());
    let global_vocabulary =
        std::env::var("MEETILY_GLOBAL_VOCABULARY").unwrap_or_else(|_| "сарматоид".into());
    let meeting_vocabulary =
        std::env::var("MEETILY_MEETING_VOCABULARY").unwrap_or_else(|_| "Ксанторис, Новорон".into());
    let resolved_vocabulary = merge_vocabulary(Some(&global_vocabulary), Some(&meeting_vocabulary))
        .expect("runtime fixture vocabulary should not be empty");
    let samples = read_mono_16k_wav(&audio_path);

    let model_dir = model_path.parent().expect("model path has a parent");
    let engine = Arc::new(
        WhisperEngine::new_with_models_dir(Some(model_dir.to_path_buf()))
            .expect("create Whisper engine"),
    );
    engine
        .discover_models()
        .await
        .expect("discover Whisper models");
    engine
        .load_model(&model_name)
        .await
        .expect("load Whisper model");

    // This provider call is the live provider seam. Run it with and without
    // the hint so the evidence captures behavior rather than asserting spelling.
    let provider = WhisperProvider::new(engine.clone());
    let live_without_hint = provider
        .transcribe(samples.clone(), Some("ru".into()))
        .await
        .expect("live provider transcription without vocabulary");
    let live_with_hint = provider
        .transcribe_with_vocabulary(
            samples.clone(),
            Some("ru".into()),
            Some(&resolved_vocabulary),
        )
        .await
        .expect("live provider transcription with vocabulary");

    // These direct engine calls are the retranscription seam used for stored
    // audio. They use the same resolved vocabulary value as the live call.
    let (batch_without_text, batch_without_confidence, batch_without_partial) = engine
        .transcribe_audio_with_confidence(samples.clone(), Some("ru".into()))
        .await
        .expect("retranscription without vocabulary");
    let (batch_with_text, batch_with_confidence, batch_with_partial) = engine
        .transcribe_audio_with_confidence_and_vocabulary(
            samples,
            Some("ru".into()),
            Some(&resolved_vocabulary),
        )
        .await
        .expect("retranscription with vocabulary");

    println!(
        "resolved_vocabulary_terms={}",
        resolved_vocabulary.split(',').count()
    );
    print_runtime_result("live_without_hint", &live_without_hint);
    print_runtime_result("live_with_hint", &live_with_hint);
    println!(
        "batch_without_hint: confidence={batch_without_confidence:?}, partial={batch_without_partial}, text={batch_without_text:?}"
    );
    println!(
        "batch_with_hint: confidence={batch_with_confidence:?}, partial={batch_with_partial}, text={batch_with_text:?}"
    );
}
