use app_lib::database::manager::backup_sqlite_recovery_files;
use sqlx::{Row, SqlitePool};
use std::fs;

fn sidecar(database_path: &std::path::Path, suffix: &str) -> std::path::PathBuf {
    let mut value = database_path.as_os_str().to_os_string();
    value.push(suffix);
    value.into()
}

#[tokio::test]
async fn recovery_backup_preserves_a_real_committed_wal_transaction() {
    let root = tempfile::tempdir().unwrap();
    let database_path = root.path().join("meeting_minutes.sqlite");
    let database_url = format!("sqlite://{}?mode=rwc", database_path.display());
    let pool = SqlitePool::connect(&database_url).await.unwrap();

    sqlx::query("PRAGMA journal_mode = WAL")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("PRAGMA wal_autocheckpoint = 0")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("CREATE TABLE transcript_guard (text TEXT NOT NULL)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO transcript_guard (text) VALUES ('committed in WAL')")
        .execute(&pool)
        .await
        .unwrap();

    let wal_path = sidecar(&database_path, "-wal");
    let shm_path = sidecar(&database_path, "-shm");
    assert!(
        wal_path.is_file(),
        "WAL fixture must be a real SQLite sidecar"
    );
    assert!(
        shm_path.is_file(),
        "SHM fixture must be a real SQLite sidecar"
    );

    let original_database = fs::read(&database_path).unwrap();
    let original_wal = fs::read(&wal_path).unwrap();
    let original_shm = fs::read(&shm_path).unwrap();
    let backup = backup_sqlite_recovery_files(&database_path).unwrap();

    assert_eq!(fs::read(&database_path).unwrap(), original_database);
    assert_eq!(fs::read(&wal_path).unwrap(), original_wal);
    assert_eq!(fs::read(&shm_path).unwrap(), original_shm);
    assert_eq!(
        fs::read(backup.join("meeting_minutes.sqlite")).unwrap(),
        original_database
    );
    assert_eq!(
        fs::read(backup.join("meeting_minutes.sqlite-wal")).unwrap(),
        original_wal
    );
    assert_eq!(
        fs::read(backup.join("meeting_minutes.sqlite-shm")).unwrap(),
        original_shm
    );

    let backup_url = format!(
        "sqlite://{}?mode=ro",
        backup.join("meeting_minutes.sqlite").display()
    );
    let backup_pool = SqlitePool::connect(&backup_url).await.unwrap();
    let recovered = sqlx::query("SELECT text FROM transcript_guard")
        .fetch_one(&backup_pool)
        .await
        .unwrap()
        .get::<String, _>("text");
    assert_eq!(recovered, "committed in WAL");

    backup_pool.close().await;
    pool.close().await;
}

#[test]
fn backup_failure_leaves_database_and_sidecars_untouched() {
    let root = tempfile::tempdir().unwrap();
    let database_path = root.path().join("meeting_minutes.sqlite");
    let wal_path = sidecar(&database_path, "-wal");
    let shm_path = sidecar(&database_path, "-shm");
    fs::write(&database_path, b"database").unwrap();
    fs::write(&wal_path, b"latest committed transcript").unwrap();
    fs::write(&shm_path, b"shared state").unwrap();
    fs::write(root.path().join("database-recovery"), b"blocks directory").unwrap();

    assert!(backup_sqlite_recovery_files(&database_path).is_err());
    assert_eq!(fs::read(&database_path).unwrap(), b"database");
    assert_eq!(fs::read(&wal_path).unwrap(), b"latest committed transcript");
    assert_eq!(fs::read(&shm_path).unwrap(), b"shared state");
}
