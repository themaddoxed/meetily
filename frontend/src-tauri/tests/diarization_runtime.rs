use app_lib::diarization::{
    alignment::assign_speaker_to_transcript,
    audio::{decode_audio_to_mono_16k, DiarizationAudio, DiarizationAudioError},
    coordinator::{run_meeting_diarization_with_pool, DiarizationRunRegistry},
    runtime::{
        build_sherpa_config, normalize_engine_segments, sha256_file, verify_model_artifact,
        DiarizationRuntime, EngineSegment, ModelArtifact, RuntimeError, SherpaDiarizationRuntime,
        SHERPA_CLUSTER_THRESHOLD,
    },
    types::TranscriptWindow,
};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use std::sync::Arc;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

fn engine_segment(start: f64, end: f64, speaker: i32) -> EngineSegment {
    EngineSegment {
        start,
        end,
        speaker,
        confidence: None,
    }
}

#[derive(Clone)]
struct FakeRuntime {
    result: Result<Vec<EngineSegment>, RuntimeError>,
}

impl DiarizationRuntime for FakeRuntime {
    fn diarize(&self, _audio: &DiarizationAudio) -> Result<Vec<EngineSegment>, RuntimeError> {
        self.result.clone()
    }
}

fn fake_runtime(result: Result<Vec<EngineSegment>, RuntimeError>) -> Arc<dyn DiarizationRuntime> {
    Arc::new(FakeRuntime { result })
}

async fn file_database(path: &Path) -> SqlitePool {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();
    MIGRATOR.run(&pool).await.unwrap();
    pool
}

fn write_test_wav(path: &Path) {
    let mut writer = hound::WavWriter::create(
        path,
        hound::WavSpec {
            channels: 1,
            sample_rate: 16_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        },
    )
    .unwrap();
    for frame in 0..16_000 {
        let sample = (((frame as f32 / 16_000.0) * 220.0 * std::f32::consts::TAU).sin()
            * i16::MAX as f32
            * 0.1) as i16;
        writer.write_sample(sample).unwrap();
    }
    writer.finalize().unwrap();
}

async fn seed_coordinator_meeting(pool: &SqlitePool, folder: Option<&Path>) {
    sqlx::query(
        "INSERT INTO meetings (id, title, created_at, updated_at, folder_path)
         VALUES ('meeting-1', 'Meeting', '2026-01-01T00:00:00Z',
                 '2026-01-01T00:00:00Z', ?)",
    )
    .bind(folder.map(|path| path.to_string_lossy().into_owned()))
    .execute(pool)
    .await
    .unwrap();

    for (id, start, end) in [("transcript-1", 0.0, 1.0), ("transcript-2", 1.0, 2.0)] {
        sqlx::query(
            "INSERT INTO transcripts (
                id, meeting_id, transcript, timestamp, speaker,
                audio_start_time, audio_end_time, duration,
                speaker_id, speaker_label, speaker_color, is_overlap,
                diarization_status, diarization_method, diarization_confidence
             ) VALUES (?, 'meeting-1', 'Existing transcript', '2026-01-01T00:00:00Z',
                       'mic', ?, ?, 1.0, 'old-speaker', 'Old Speaker', '#111111', 0,
                       'provisional', 'old-engine', 0.4)",
        )
        .bind(id)
        .bind(start)
        .bind(end)
        .execute(pool)
        .await
        .unwrap();
    }

    sqlx::query(
        "INSERT INTO speaker_segments (
            id, meeting_id, source, start_time, end_time, speaker_id, speaker_label,
            confidence, is_overlap, diarization_status, diarization_method, created_at
         ) VALUES ('old-segment', 'meeting-1', 'old-source', 0.0, 2.0,
                   'old-speaker', 'Old Speaker', 0.4, 0, 'provisional', 'old-engine',
                   '2026-01-01T00:00:00Z')",
    )
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO meeting_diarization_status (
            meeting_id, enabled, live_enabled, post_call_enabled, current_status,
            quality_flags, processed_at, updated_at
         ) VALUES ('meeting-1', 1, 1, 0, 'provisional', 'old-quality',
                   '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
    )
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "UPDATE diarization_settings
         SET enabled = 1, mode = 'post_call_only', post_call_refinement_enabled = 1
         WHERE id = '1'",
    )
    .execute(pool)
    .await
    .unwrap();
}

type TranscriptSnapshotRow = (String, Option<String>, Option<String>, Option<String>);

#[derive(Debug, PartialEq)]
struct CoordinatorSnapshot {
    transcripts: Vec<TranscriptSnapshotRow>,
    segments: Vec<(String, Option<String>, Option<String>, String)>,
    status: Vec<(String, i64, String)>,
}

async fn coordinator_snapshot(pool: &SqlitePool) -> CoordinatorSnapshot {
    CoordinatorSnapshot {
        transcripts: sqlx::query_as(
            "SELECT id, speaker_id, speaker_label, diarization_method
             FROM transcripts ORDER BY id",
        )
        .fetch_all(pool)
        .await
        .unwrap(),
        segments: sqlx::query_as(
            "SELECT id, speaker_id, speaker_label, diarization_status
             FROM speaker_segments ORDER BY id",
        )
        .fetch_all(pool)
        .await
        .unwrap(),
        status: sqlx::query_as(
            "SELECT meeting_id, post_call_enabled, current_status
             FROM meeting_diarization_status ORDER BY meeting_id",
        )
        .fetch_all(pool)
        .await
        .unwrap(),
    }
}

#[test]
fn model_artifacts_require_exact_size_and_sha256() {
    let directory = tempfile::tempdir().unwrap();
    let model_path = directory.path().join("model.onnx");
    std::fs::write(&model_path, b"verified model bytes").unwrap();
    let artifact = ModelArtifact {
        path: model_path.clone(),
        size: 20,
        sha256: sha256_file(&model_path).unwrap(),
    };

    verify_model_artifact(&artifact).unwrap();

    std::fs::write(&model_path, b"tampered").unwrap();
    assert_eq!(
        verify_model_artifact(&artifact),
        Err(RuntimeError::ModelInvalid)
    );

    assert_eq!(
        verify_model_artifact(&ModelArtifact {
            path: directory.path().join("missing.onnx"),
            ..artifact
        }),
        Err(RuntimeError::ModelUnavailable)
    );
}

#[test]
fn audio_decode_rejects_missing_unsupported_and_empty_files() {
    let directory = tempfile::tempdir().unwrap();
    let missing = directory.path().join("missing.wav");
    assert_eq!(
        decode_audio_to_mono_16k(&missing),
        Err(DiarizationAudioError::Unavailable)
    );

    let unsupported = directory.path().join("audio.bin");
    std::fs::write(&unsupported, b"not audio").unwrap();
    assert_eq!(
        decode_audio_to_mono_16k(&unsupported),
        Err(DiarizationAudioError::Unsupported)
    );

    let empty = directory.path().join("empty.wav");
    std::fs::write(&empty, []).unwrap();
    assert_eq!(
        decode_audio_to_mono_16k(&empty),
        Err(DiarizationAudioError::Empty)
    );
}

#[test]
fn audio_decode_reuses_production_decoder_for_16khz_mono_output() {
    let directory = tempfile::tempdir().unwrap();
    let wav_path = directory.path().join("stereo-48k.wav");
    let mut writer = hound::WavWriter::create(
        &wav_path,
        hound::WavSpec {
            channels: 2,
            sample_rate: 48_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        },
    )
    .unwrap();
    for frame in 0..4_800 {
        let sample = (((frame as f32 / 48_000.0) * 440.0 * std::f32::consts::TAU).sin()
            * i16::MAX as f32
            * 0.2) as i16;
        writer.write_sample(sample).unwrap();
        writer.write_sample(sample).unwrap();
    }
    writer.finalize().unwrap();

    let audio = decode_audio_to_mono_16k(&wav_path).unwrap();
    assert_eq!(audio.sample_rate, 16_000);
    assert!(audio.samples.iter().all(|sample| sample.is_finite()));
    assert!((1_500..=1_700).contains(&audio.samples.len()));
}

#[test]
fn threshold_clustering_does_not_supply_a_participant_count() {
    let config = build_sherpa_config(
        PathBuf::from("segmentation.onnx"),
        PathBuf::from("embedding.onnx"),
        2,
    );

    assert_eq!(config.clustering.num_clusters, -1);
    assert_eq!(config.clustering.threshold, SHERPA_CLUSTER_THRESHOLD);
    assert_eq!(config.segmentation.num_threads, 2);
    assert_eq!(config.embedding.num_threads, 2);
}

#[test]
fn normalization_creates_stable_labels_colors_and_generic_overlap() {
    let normalized = normalize_engine_segments(
        "meeting-1",
        vec![engine_segment(0.0, 2.0, 9), engine_segment(1.0, 3.0, 2)],
    )
    .unwrap();

    assert_eq!(normalized.len(), 3);
    assert_eq!(normalized[0].speaker_id.as_deref(), Some("speaker-2"));
    assert_eq!(normalized[0].speaker_label.as_deref(), Some("Speaker 2"));
    assert_eq!(
        (normalized[0].start_time, normalized[0].end_time),
        (0.0, 1.0)
    );
    assert!(normalized[1].is_overlap);
    assert_eq!(normalized[1].speaker_id, None);
    assert_eq!(normalized[1].speaker_label, None);
    assert_eq!(
        (normalized[1].start_time, normalized[1].end_time),
        (1.0, 2.0)
    );
    assert_eq!(normalized[2].speaker_id.as_deref(), Some("speaker-1"));
    assert_eq!(normalized[2].speaker_label.as_deref(), Some("Speaker 1"));

    let first_assignment = assign_speaker_to_transcript(
        &TranscriptWindow {
            transcript_id: "transcript-1".to_string(),
            audio_start_time: Some(0.0),
            audio_end_time: Some(0.9),
        },
        &normalized,
        0.1,
        "sherpa_onnx_1_13_2",
    );
    let repeated_assignment = assign_speaker_to_transcript(
        &TranscriptWindow {
            transcript_id: "transcript-2".to_string(),
            audio_start_time: Some(0.1),
            audio_end_time: Some(0.8),
        },
        &normalized,
        0.1,
        "sherpa_onnx_1_13_2",
    );
    assert_eq!(
        first_assignment.speaker_color,
        repeated_assignment.speaker_color
    );
    assert!(first_assignment.speaker_color.is_some());
}

#[test]
fn normalization_rejects_empty_malformed_or_invalid_speaker_output() {
    assert_eq!(
        normalize_engine_segments("meeting-1", vec![]),
        Err(RuntimeError::NoSpeech)
    );
    for segment in [
        engine_segment(f64::NAN, 1.0, 0),
        engine_segment(0.0, f64::INFINITY, 0),
        engine_segment(-1.0, 1.0, 0),
        engine_segment(1.0, 1.0, 0),
        engine_segment(2.0, 1.0, 0),
        engine_segment(0.0, 1.0, -1),
    ] {
        assert_eq!(
            normalize_engine_segments("meeting-1", vec![segment]),
            Err(RuntimeError::InvalidOutput)
        );
    }
}

#[tokio::test]
async fn coordinator_preflight_returns_stable_codes_without_running_inference() {
    let directory = tempfile::tempdir().unwrap();
    let database_path = directory.path().join("preflight.sqlite");
    let pool = file_database(&database_path).await;
    let registry = DiarizationRunRegistry::default();
    let runtime = fake_runtime(Ok(vec![engine_segment(0.0, 1.0, 0)]));

    assert_eq!(
        run_meeting_diarization_with_pool(
            &pool,
            "missing-meeting",
            false,
            &registry,
            runtime.clone(),
        )
        .await,
        Err("meeting_not_found".to_string())
    );

    seed_coordinator_meeting(&pool, None).await;
    assert_eq!(
        run_meeting_diarization_with_pool(&pool, "meeting-1", false, &registry, runtime.clone(),)
            .await,
        Err("missing_recording".to_string())
    );

    let recording_folder = directory.path().join("recording");
    std::fs::create_dir(&recording_folder).unwrap();
    write_test_wav(&recording_folder.join("audio.wav"));
    sqlx::query("UPDATE meetings SET folder_path = ? WHERE id = 'meeting-1'")
        .bind(recording_folder.to_string_lossy().into_owned())
        .execute(&pool)
        .await
        .unwrap();

    std::fs::remove_file(recording_folder.join("audio.wav")).unwrap();
    assert_eq!(
        run_meeting_diarization_with_pool(&pool, "meeting-1", false, &registry, runtime.clone(),)
            .await,
        Err("missing_recording".to_string())
    );
    write_test_wav(&recording_folder.join("audio.wav"));

    assert_eq!(
        run_meeting_diarization_with_pool(&pool, "meeting-1", true, &registry, runtime.clone(),)
            .await,
        Err("recording_active".to_string())
    );

    sqlx::query("UPDATE diarization_settings SET enabled = 0, mode = 'off' WHERE id = '1'")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        run_meeting_diarization_with_pool(&pool, "meeting-1", false, &registry, runtime.clone(),)
            .await,
        Err("feature_disabled".to_string())
    );

    sqlx::query(
        "UPDATE diarization_settings
         SET enabled = 1, mode = 'post_call_only', post_call_refinement_enabled = 0
         WHERE id = '1'",
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        run_meeting_diarization_with_pool(&pool, "meeting-1", false, &registry, runtime.clone(),)
            .await,
        Err("feature_disabled".to_string())
    );

    sqlx::query(
        "UPDATE diarization_settings
         SET enabled = 1, mode = 'post_call_only', post_call_refinement_enabled = 1
         WHERE id = '1'",
    )
    .execute(&pool)
    .await
    .unwrap();
    let _active_run = registry.try_start("meeting-1").unwrap();
    assert_eq!(
        run_meeting_diarization_with_pool(&pool, "meeting-1", false, &registry, runtime).await,
        Err("already_running".to_string())
    );
}

#[tokio::test]
async fn primary_incomplete_recording_is_not_hidden_by_wav_fallback() {
    let directory = tempfile::tempdir().unwrap();
    let recording_folder = directory.path().join("recording");
    std::fs::create_dir(&recording_folder).unwrap();
    std::fs::write(recording_folder.join("audio.mp4"), []).unwrap();
    write_test_wav(&recording_folder.join("audio.wav"));

    let pool = file_database(&directory.path().join("incomplete.sqlite")).await;
    seed_coordinator_meeting(&pool, Some(&recording_folder)).await;
    let before = coordinator_snapshot(&pool).await;

    assert_eq!(
        run_meeting_diarization_with_pool(
            &pool,
            "meeting-1",
            false,
            &DiarizationRunRegistry::default(),
            fake_runtime(Ok(vec![engine_segment(0.0, 1.0, 0)])),
        )
        .await,
        Err("recording_incomplete".to_string())
    );
    assert_eq!(coordinator_snapshot(&pool).await, before);
}

#[tokio::test]
async fn every_runtime_failure_preserves_previous_data_after_reopen() {
    let cases = [
        (Ok(vec![]), "insufficient_speech"),
        (Err(RuntimeError::InferenceFailed), "inference_failed"),
        (Err(RuntimeError::ModelInvalid), "model_invalid"),
        (Ok(vec![engine_segment(f64::NAN, 1.0, 0)]), "invalid_output"),
    ];

    for (index, (runtime_result, expected_code)) in cases.into_iter().enumerate() {
        let directory = tempfile::tempdir().unwrap();
        let recording_folder = directory.path().join("recording");
        std::fs::create_dir(&recording_folder).unwrap();
        write_test_wav(&recording_folder.join("audio.wav"));
        let database_path = directory.path().join(format!("failure-{index}.sqlite"));
        let pool = file_database(&database_path).await;
        seed_coordinator_meeting(&pool, Some(&recording_folder)).await;
        let before = coordinator_snapshot(&pool).await;
        let registry = DiarizationRunRegistry::default();

        assert_eq!(
            run_meeting_diarization_with_pool(
                &pool,
                "meeting-1",
                false,
                &registry,
                fake_runtime(runtime_result),
            )
            .await,
            Err(expected_code.to_string())
        );

        pool.close().await;
        let reopened = file_database(&database_path).await;
        assert_eq!(coordinator_snapshot(&reopened).await, before);
        assert!(registry.try_start("meeting-1").is_ok());
    }
}

#[tokio::test]
async fn apply_failure_preserves_previous_data_after_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let recording_folder = directory.path().join("recording");
    std::fs::create_dir(&recording_folder).unwrap();
    write_test_wav(&recording_folder.join("audio.wav"));
    let database_path = directory.path().join("apply-failure.sqlite");
    let pool = file_database(&database_path).await;
    seed_coordinator_meeting(&pool, Some(&recording_folder)).await;
    let before = coordinator_snapshot(&pool).await;

    sqlx::query(
        "CREATE TRIGGER fail_diarization_apply
         BEFORE UPDATE OF speaker_id ON transcripts
         BEGIN SELECT RAISE(FAIL, 'forced transcript update failure'); END",
    )
    .execute(&pool)
    .await
    .unwrap();

    assert_eq!(
        run_meeting_diarization_with_pool(
            &pool,
            "meeting-1",
            false,
            &DiarizationRunRegistry::default(),
            fake_runtime(Ok(vec![engine_segment(0.0, 2.0, 0)])),
        )
        .await,
        Err("persistence_failed".to_string())
    );

    pool.close().await;
    let reopened = file_database(&database_path).await;
    assert_eq!(coordinator_snapshot(&reopened).await, before);
}

#[tokio::test]
async fn success_uses_wav_fallback_and_rerun_atomically_replaces_labels() {
    let directory = tempfile::tempdir().unwrap();
    let recording_folder = directory.path().join("recording");
    std::fs::create_dir(&recording_folder).unwrap();
    write_test_wav(&recording_folder.join("audio.wav"));
    let pool = file_database(&directory.path().join("success.sqlite")).await;
    seed_coordinator_meeting(&pool, Some(&recording_folder)).await;
    let registry = DiarizationRunRegistry::default();

    let first = run_meeting_diarization_with_pool(
        &pool,
        "meeting-1",
        false,
        &registry,
        fake_runtime(Ok(vec![engine_segment(0.0, 2.0, 7)])),
    )
    .await
    .unwrap();
    assert_eq!(first.segment_count, 1);
    assert_eq!(first.speaker_count, 1);

    let second = run_meeting_diarization_with_pool(
        &pool,
        "meeting-1",
        false,
        &registry,
        fake_runtime(Ok(vec![
            engine_segment(0.0, 1.0, 4),
            engine_segment(1.0, 2.0, 2),
        ])),
    )
    .await
    .unwrap();
    assert_eq!(second.segment_count, 2);
    assert_eq!(second.speaker_count, 2);

    let segment_rows = sqlx::query_as::<_, (Option<String>, Option<String>)>(
        "SELECT speaker_id, speaker_label FROM speaker_segments ORDER BY start_time",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        segment_rows,
        vec![
            (Some("speaker-2".to_string()), Some("Speaker 2".to_string())),
            (Some("speaker-1".to_string()), Some("Speaker 1".to_string())),
        ]
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM speaker_segments")
            .fetch_one(&pool)
            .await
            .unwrap(),
        2
    );
}

#[test]
#[ignore = "requires pinned production models and a generated macOS two-voice fixture"]
fn production_sherpa_runtime_detects_two_ordered_speakers_without_a_count() {
    let fixture_path = PathBuf::from(
        std::env::var("MEETILY_DIARIZATION_FIXTURE_PATH")
            .expect("MEETILY_DIARIZATION_FIXTURE_PATH must point to the generated fixture"),
    );
    let resource_root = PathBuf::from(
        std::env::var("MEETILY_DIARIZATION_RESOURCE_ROOT")
            .expect("MEETILY_DIARIZATION_RESOURCE_ROOT must point to packaged-style resources"),
    );

    let audio = decode_audio_to_mono_16k(&fixture_path).unwrap();
    let runtime = SherpaDiarizationRuntime::from_resource_root(&resource_root, 2).unwrap();
    let engine_segments = runtime.diarize(&audio).unwrap();
    assert!(engine_segments
        .windows(2)
        .all(|pair| pair[0].start <= pair[1].start));

    let normalized = normalize_engine_segments("production-fixture", engine_segments).unwrap();
    let speakers = normalized
        .iter()
        .filter_map(|segment| segment.speaker_id.as_deref())
        .collect::<std::collections::BTreeSet<_>>();
    assert!(
        speakers.len() >= 2,
        "expected at least two clusters, got {speakers:?}"
    );
    assert!(normalized
        .iter()
        .all(|segment| { segment.validate_for_meeting("production-fixture").is_ok() }));
    assert!(
        normalized
            .last()
            .is_some_and(|segment| segment.end_time >= 10.0),
        "expected useful fixture coverage, got {normalized:?}"
    );
    println!(
        "production diarization: speakers={}, segments={}, coverage_end={:.2}s",
        speakers.len(),
        normalized.len(),
        normalized.last().map_or(0.0, |segment| segment.end_time),
    );
}
