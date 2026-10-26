use app_lib::database::repositories::meeting_context::MeetingContextRepository;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use tempfile::TempDir;

async fn database() -> (TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(dir.path().join("context.sqlite"))
                .create_if_missing(true)
                .foreign_keys(true),
        )
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    (dir, pool)
}

async fn meeting(pool: &SqlitePool, id: &str, title: &str) {
    sqlx::query("INSERT INTO meetings (id, title, created_at, updated_at) VALUES (?, ?, '2026-10-02T10:00:00Z', '2026-10-02T10:00:00Z')")
        .bind(id).bind(title).execute(pool).await.unwrap();
}

async fn transcript(pool: &SqlitePool, meeting: &str, id: &str, text: &str, start: Option<f64>) {
    sqlx::query("INSERT INTO transcripts (id, meeting_id, transcript, timestamp, audio_start_time) VALUES (?, ?, ?, '10:00:00', ?)")
        .bind(id).bind(meeting).bind(text).bind(start).execute(pool).await.unwrap();
}

#[tokio::test]
async fn context_contains_only_selected_meeting_and_full_transcript() {
    let (_dir, pool) = database().await;
    meeting(&pool, "selected", "Chosen meeting").await;
    meeting(&pool, "neighbor", "Private neighboring title").await;
    transcript(
        &pool,
        "neighbor",
        "neighbor-source",
        "Private neighboring content",
        Some(0.0),
    )
    .await;
    for index in (0..160).rev() {
        transcript(
            &pool,
            "selected",
            &format!("source-{index}"),
            &format!("Текст реплики {index}"),
            Some(index as f64 * 5.0),
        )
        .await;
    }
    let result = MeetingContextRepository::read(&pool, "selected")
        .await
        .unwrap();
    assert_eq!(result.meeting_id, "selected");
    assert_eq!(result.title, "Chosen meeting");
    assert_eq!(result.sources.len(), 160);
    assert_eq!(result.sources[120].transcript_id, "source-120");
    assert_eq!(result.sources[120].audio_start_time, Some(600.0));
    let serialized = serde_json::to_string(&result).unwrap();
    assert!(!serialized.contains("neighbor"));
    assert!(!serialized.contains("Private"));
    assert!(serialized.contains("Текст реплики 159"));
}

#[tokio::test]
async fn links_round_trip_untrusted_ids_and_remain_stable() {
    let (_dir, pool) = database().await;
    let id = "встреча?other=neighbor&x=#fragment";
    let source = "реплика&transcriptId=neighbor?#";
    meeting(&pool, id, "Selected").await;
    transcript(&pool, id, source, "Visible text", Some(42.5)).await;
    let first = MeetingContextRepository::read(&pool, id).await.unwrap();
    let second = MeetingContextRepository::read(&pool, id).await.unwrap();
    assert_eq!(first.sources[0].href, second.sources[0].href);
    let url = url::Url::parse(&format!("http://localhost{}", first.sources[0].href)).unwrap();
    let pairs: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
    assert_eq!(pairs.len(), 4);
    assert_eq!(pairs["id"], id);
    assert_eq!(pairs["transcriptId"], source);
    assert_eq!(pairs["at"], "42.5");
    assert!(url.fragment().is_none());
}

#[tokio::test]
async fn missing_deleted_or_blank_meeting_never_falls_back_to_another_meeting() {
    let (_dir, pool) = database().await;
    meeting(&pool, "neighbor", "Other meeting").await;
    transcript(&pool, "neighbor", "other", "Other text", None).await;
    assert!(matches!(
        MeetingContextRepository::read(&pool, "missing").await,
        Err(sqlx::Error::RowNotFound)
    ));
    assert!(matches!(
        MeetingContextRepository::read(&pool, "  ").await,
        Err(sqlx::Error::Protocol(_))
    ));
    meeting(&pool, "deleted", "Deleted").await;
    transcript(
        &pool,
        "deleted",
        "deleted-source",
        "Deleted text",
        Some(0.0),
    )
    .await;
    sqlx::query("DELETE FROM meetings WHERE id = 'deleted'")
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        MeetingContextRepository::read(&pool, "deleted").await,
        Err(sqlx::Error::RowNotFound)
    ));
}

#[tokio::test]
async fn empty_transcript_and_unusable_time_do_not_invent_evidence() {
    let (_dir, pool) = database().await;
    meeting(&pool, "empty", "Empty meeting").await;
    assert!(MeetingContextRepository::read(&pool, "empty")
        .await
        .unwrap()
        .sources
        .is_empty());
    transcript(&pool, "empty", "blank", " \n ", None).await;
    transcript(&pool, "empty", "untimed", "Actual text", Some(-1.0)).await;
    let result = MeetingContextRepository::read(&pool, "empty")
        .await
        .unwrap();
    assert_eq!(result.sources.len(), 1);
    assert_eq!(result.sources[0].transcript_id, "untimed");
    assert_eq!(result.sources[0].audio_start_time, None);
    assert!(!result.sources[0].href.contains("at="));
}
