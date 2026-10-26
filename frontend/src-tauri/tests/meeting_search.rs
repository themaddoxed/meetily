use app_lib::api::TranscriptSegment;
use app_lib::database::repositories::meeting::MeetingsRepository;
use app_lib::database::repositories::search::SearchRepository;
use app_lib::database::repositories::summary::SummaryProcessesRepository;
use app_lib::database::repositories::transcript::TranscriptsRepository;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Connection, SqlitePool};
use tempfile::TempDir;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

struct TestDatabase {
    _directory: TempDir,
    path: std::path::PathBuf,
    pool: SqlitePool,
}

impl TestDatabase {
    async fn migrated() -> Self {
        let directory = tempfile::tempdir().expect("create temporary database directory");
        let path = directory.path().join("meeting_minutes.sqlite");
        let pool = open_pool(&path).await;
        MIGRATOR.run(&pool).await.expect("apply migrations");
        SearchRepository::rebuild(&pool)
            .await
            .expect("build initial search index");
        Self {
            _directory: directory,
            path,
            pool,
        }
    }

    async fn reopen(&mut self) {
        self.pool.close().await;
        self.pool = open_pool(&self.path).await;
        MIGRATOR.run(&self.pool).await.expect("reopen migrations");
        SearchRepository::rebuild(&self.pool)
            .await
            .expect("rebuild search index after reopen");
    }
}

async fn open_pool(path: &std::path::Path) -> SqlitePool {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .foreign_keys(true);
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .expect("open test database")
}

fn segment(text: &str, start: f64) -> TranscriptSegment {
    TranscriptSegment {
        id: "input-segment".to_string(),
        text: text.to_string(),
        timestamp: "2026-09-26T09:00:00Z".to_string(),
        audio_start_time: Some(start),
        audio_end_time: Some(start + 1.5),
        duration: Some(1.5),
        speaker_id: None,
        speaker_label: None,
        speaker_color: None,
        is_overlap: Some(false),
        diarization_status: Some("none".to_string()),
        diarization_method: None,
        diarization_confidence: None,
    }
}

#[tokio::test]
async fn search_covers_visible_title_transcript_and_summary_only() {
    let database = TestDatabase::migrated().await;
    let meeting_id = TranscriptsRepository::save_transcript(
        &database.pool,
        "Roadmap planning",
        &[segment("The launch date is Friday", 42.5)],
        Some("/tmp/roadmap-planning".to_string()),
    )
    .await
    .expect("save meeting");

    SummaryProcessesRepository::create_or_reset_process(&database.pool, &meeting_id)
        .await
        .expect("create summary process");
    SummaryProcessesRepository::save_manual_summary(
        &database.pool,
        &meeting_id,
        &serde_json::json!({
            "summary_json": [{
                "id": "summary-block",
                "type": "paragraph",
                "content": [
                    {"type": "text", "text": "Visible summary ", "styles": {}},
                    {"type": "text", "text": "decision", "styles": {}}
                ],
                "children": []
            }],
            "markdown": "Hidden stale markdown phrase",
            "english_cache": {"markdown": "Hidden translation phrase"},
            "internal_json": "Hidden implementation data"
        }),
        false,
    )
    .await
    .expect("save summary");

    let title_results = SearchRepository::search(&database.pool, "roadmap")
        .await
        .unwrap();
    assert!(title_results
        .iter()
        .any(|result| result.source_type == "title"));

    let transcript_results = SearchRepository::search(&database.pool, "launch date")
        .await
        .unwrap();
    let transcript = transcript_results
        .iter()
        .find(|result| result.source_type == "transcript")
        .expect("transcript result");
    assert_eq!(transcript.id, meeting_id);
    assert!(transcript.source_id.is_some());
    assert_eq!(transcript.audio_start_time, Some(42.5));
    assert!(transcript.match_context.contains("launch date"));

    let summary_results = SearchRepository::search(&database.pool, "summary decision")
        .await
        .unwrap();
    assert!(summary_results
        .iter()
        .any(|result| result.source_type == "summary"));

    assert!(
        SearchRepository::search(&database.pool, "hidden translation phrase")
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        SearchRepository::search(&database.pool, "hidden stale markdown phrase")
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        SearchRepository::search(&database.pool, "hidden implementation data")
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn markdown_summary_search_uses_rendered_text_not_formatting_or_link_urls() {
    let database = TestDatabase::migrated().await;
    let meeting_id = TranscriptsRepository::save_transcript(
        &database.pool,
        "Markdown summary",
        &[segment("Source transcript", 0.0)],
        None,
    )
    .await
    .expect("save meeting");

    SummaryProcessesRepository::create_or_reset_process(&database.pool, &meeting_id)
        .await
        .expect("create summary process");
    SummaryProcessesRepository::save_manual_summary(
        &database.pool,
        &meeting_id,
        &serde_json::json!({
            "markdown": "# Launch _date_: [decision](https://hidden.example/path); [plan][plan-ref]; code `a*b`; and https://visible.example\n\n[plan-ref]: https://hidden.reference.example/path"
        }),
        false,
    )
    .await
    .expect("save markdown summary");

    assert!(SearchRepository::search(&database.pool, "launch date")
        .await
        .unwrap()
        .iter()
        .any(|result| result.source_type == "summary"));
    assert!(SearchRepository::search(&database.pool, "decision")
        .await
        .unwrap()
        .iter()
        .any(|result| result.source_type == "summary"));
    assert!(SearchRepository::search(&database.pool, "hidden.example")
        .await
        .unwrap()
        .is_empty());
    assert!(SearchRepository::search(&database.pool, "plan-ref")
        .await
        .unwrap()
        .is_empty());
    assert!(SearchRepository::search(&database.pool, "visible.example")
        .await
        .unwrap()
        .iter()
        .any(|result| result.source_type == "summary"));
    assert!(SearchRepository::search(&database.pool, "code a*b")
        .await
        .unwrap()
        .iter()
        .any(|result| result.source_type == "summary"));
    assert!(SearchRepository::search(&database.pool, "code ab")
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn search_index_tracks_changes_and_deleted_meetings_after_reopen() {
    let mut database = TestDatabase::migrated().await;
    let meeting_id = TranscriptsRepository::save_transcript(
        &database.pool,
        "Old title",
        &[segment("Old transcript phrase", 5.0)],
        None,
    )
    .await
    .expect("save meeting");

    MeetingsRepository::update_meeting_title(&database.pool, &meeting_id, "New title")
        .await
        .expect("update title");
    assert!(SearchRepository::search(&database.pool, "new title")
        .await
        .unwrap()
        .iter()
        .any(|result| result.source_type == "title"));
    assert!(SearchRepository::search(&database.pool, "old title")
        .await
        .unwrap()
        .is_empty());

    let mut connection = database.pool.acquire().await.unwrap();
    let mut transaction = connection.begin().await.unwrap();
    TranscriptsRepository::replace_for_retranscription(
        &mut transaction,
        &meeting_id,
        &[segment("New transcript phrase", 8.0)],
    )
    .await
    .expect("replace transcript");
    transaction.commit().await.unwrap();
    drop(connection);

    assert!(
        SearchRepository::search(&database.pool, "new transcript phrase")
            .await
            .unwrap()
            .iter()
            .any(|result| result.source_type == "transcript")
    );
    assert!(
        SearchRepository::search(&database.pool, "old transcript phrase")
            .await
            .unwrap()
            .is_empty()
    );

    MeetingsRepository::delete_meeting(&database.pool, &meeting_id)
        .await
        .expect("delete meeting");
    database.reopen().await;
    assert!(SearchRepository::search(&database.pool, "new title")
        .await
        .unwrap()
        .is_empty());
    assert!(
        SearchRepository::search(&database.pool, "new transcript phrase")
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn manual_summary_and_search_keep_the_new_owner_when_old_generation_finishes() {
    let database = TestDatabase::migrated().await;
    let meeting_id = TranscriptsRepository::save_transcript(
        &database.pool,
        "Concurrent summary",
        &[segment("Unrelated transcript", 0.0)],
        None,
    )
    .await
    .unwrap();
    let (old_owner, _) = SummaryProcessesRepository::start_process(&database.pool, &meeting_id)
        .await
        .unwrap();
    SummaryProcessesRepository::save_manual_summary(
        &database.pool,
        &meeting_id,
        &serde_json::json!({"markdown": "Accepted manual decision"}),
        false,
    )
    .await
    .unwrap();
    let indexed: String = sqlx::query_scalar(
        "SELECT content FROM meeting_search_index WHERE meeting_id = ? AND source_type = 'summary'",
    )
    .bind(&meeting_id)
    .fetch_one(&database.pool)
    .await
    .unwrap();
    assert!(indexed.contains("Accepted manual decision"));
    let result = SummaryProcessesRepository::update_process_completed_for_run(
        &database.pool,
        &meeting_id,
        &old_owner,
        serde_json::json!({"markdown": "Outdated generated decision"}),
        1,
        0.5,
    )
    .await;
    assert!(matches!(result, Err(sqlx::Error::RowNotFound)));
    assert!(
        SearchRepository::search(&database.pool, "Outdated generated")
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        SearchRepository::search(&database.pool, "Accepted manual")
            .await
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn failed_search_refresh_rolls_back_manual_summary_acceptance() {
    let database = TestDatabase::migrated().await;
    let meeting_id = TranscriptsRepository::save_transcript(
        &database.pool,
        "Atomic summary",
        &[segment("Source transcript", 0.0)],
        None,
    )
    .await
    .unwrap();
    SummaryProcessesRepository::save_manual_summary(
        &database.pool,
        &meeting_id,
        &serde_json::json!({"markdown": "Previously accepted decision"}),
        false,
    )
    .await
    .unwrap();
    let owner_before: String =
        sqlx::query_scalar("SELECT lifecycle_run_id FROM meetings WHERE id = ?")
            .bind(&meeting_id)
            .fetch_one(&database.pool)
            .await
            .unwrap();
    sqlx::raw_sql(
        "CREATE TRIGGER reject_search_refresh BEFORE INSERT ON meeting_search_index
         BEGIN SELECT RAISE(ABORT, 'injected search refresh failure'); END;",
    )
    .execute(&database.pool)
    .await
    .unwrap();
    let result = SummaryProcessesRepository::save_manual_summary(
        &database.pool,
        &meeting_id,
        &serde_json::json!({"markdown": "Unaccepted replacement"}),
        false,
    )
    .await;
    assert!(result.is_err());
    let canonical: String =
        sqlx::query_scalar("SELECT result FROM summary_processes WHERE meeting_id = ?")
            .bind(&meeting_id)
            .fetch_one(&database.pool)
            .await
            .unwrap();
    assert!(canonical.contains("Previously accepted decision"));
    assert!(!canonical.contains("Unaccepted replacement"));
    let owner_after: String =
        sqlx::query_scalar("SELECT lifecycle_run_id FROM meetings WHERE id = ?")
            .bind(&meeting_id)
            .fetch_one(&database.pool)
            .await
            .unwrap();
    assert_eq!(owner_after, owner_before);
    let indexed: String = sqlx::query_scalar(
        "SELECT content FROM meeting_search_index WHERE meeting_id = ? AND source_type = 'summary'",
    )
    .bind(&meeting_id)
    .fetch_one(&database.pool)
    .await
    .unwrap();
    assert!(indexed.contains("Previously accepted decision"));
}
