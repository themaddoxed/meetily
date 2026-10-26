use log::info;
use tauri::{AppHandle, Emitter, Manager};

use super::legacy_fastapi::acquire_app_data_gate;
use super::manager::DatabaseManager;
use super::migration::ensure_no_incomplete_migrations;
use crate::state::AppState;

/// Initialize database on app startup
/// Handles first launch detection and conditional initialization
pub async fn initialize_database_on_startup(app: &AppHandle) -> Result<(), String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;
    let gate = acquire_app_data_gate(&app_data_dir)
        .await
        .map_err(|e| format!("Failed to acquire database gate: {e}"))?;
    if app.try_state::<AppState>().is_some() {
        return Err("Database state is already managed".to_string());
    }

    ensure_no_incomplete_migrations(&app_data_dir)
        .map_err(|e| format!("Refusing startup with incomplete data migration: {e}"))?;

    let is_first_launch = DatabaseManager::is_first_launch_already_gated(&app_data_dir)
        .await
        .map_err(|e| format!("Failed to check first launch status: {e}"))?;

    if is_first_launch {
        info!("First launch detected - will notify window when ready");

        // Delay event emission to ensure window is ready and React listeners are registered
        let app_handle = app.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            app_handle
                .emit("first-launch-detected", ())
                .expect("Failed to emit first-launch-detected event");
            info!("Emitted first-launch-detected after delay");
        });
    } else {
        std::fs::create_dir_all(&app_data_dir)
            .map_err(|e| format!("Failed to create app data dir: {e}"))?;
        let db_manager = DatabaseManager::new_from_app_handle_already_gated(app, &app_data_dir)
            .await
            .map_err(|e| format!("Failed to initialize database manager: {e}"))?;

        if !app.manage(AppState { db_manager }) {
            return Err("Database state became managed during startup".to_string());
        }
        info!("Database initialized successfully");
    }

    drop(gate);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use sqlx::Connection;
    use std::path::{Path, PathBuf};
    use std::str::FromStr;
    use std::sync::Arc;

    fn sidecar_path(path: &Path, suffix: &str) -> PathBuf {
        let mut sidecar = path.as_os_str().to_os_string();
        sidecar.push(suffix);
        sidecar.into()
    }

    fn trio(path: &Path) -> [Option<Vec<u8>>; 3] {
        [
            std::fs::read(path).ok(),
            std::fs::read(sidecar_path(path, "-wal")).ok(),
            std::fs::read(sidecar_path(path, "-shm")).ok(),
        ]
    }

    #[tokio::test]
    async fn concurrent_startup_init_import_installs_once_without_mutation() {
        let directory = tempfile::tempdir().unwrap();
        let app_data_dir = directory.path().join("app-data");
        std::fs::create_dir(&app_data_dir).unwrap();
        let current_path = app_data_dir.join("meeting_minutes.sqlite");
        let legacy_path = app_data_dir.join("meeting_minutes.db");
        let selected_source = directory.path().join("selected-source.db");

        let mut current = sqlx::SqliteConnection::connect_with(
            &SqliteConnectOptions::from_str(current_path.to_string_lossy().as_ref())
                .unwrap()
                .create_if_missing(true)
                .foreign_keys(true),
        )
        .await
        .unwrap();
        sqlx::migrate!("./migrations")
            .run(&mut current)
            .await
            .unwrap();
        current.close().await.unwrap();

        let source_pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::from_str(selected_source.to_string_lossy().as_ref())
                    .unwrap()
                    .create_if_missing(true)
                    .foreign_keys(true),
            )
            .await
            .unwrap();
        sqlx::raw_sql(include_str!(
            "../../tests/fixtures/legacy_fastapi_schema.sql"
        ))
        .execute(&source_pool)
        .await
        .unwrap();
        source_pool.close().await;
        std::fs::write(&legacy_path, b"migration-only legacy sentinel").unwrap();

        let managed = Arc::new(std::sync::Mutex::new(None::<DatabaseManager>));
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = tokio::sync::oneshot::channel();
        let startup_dir = app_data_dir.clone();
        let startup_current = current_path.clone();
        let startup_legacy = legacy_path.clone();
        let startup_managed = managed.clone();
        let startup = tokio::spawn(async move {
            let gate = acquire_app_data_gate(&startup_dir).await.unwrap();
            assert!(startup_managed.lock().unwrap().is_none());
            let manager =
                DatabaseManager::new_already_gated(&startup_current, &startup_legacy, &startup_dir)
                    .await
                    .unwrap();
            ready_tx.send(()).unwrap();
            release_rx.await.unwrap();
            let mut state = startup_managed.lock().unwrap();
            assert!(state.is_none());
            *state = Some(manager);
            drop(state);
            drop(gate);
        });

        ready_rx.await.unwrap();
        let source_before = trio(&selected_source);
        let current_before = trio(&current_path);
        let legacy_before = trio(&legacy_path);
        let artifacts_before: Vec<String> = std::fs::read_dir(&app_data_dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name.contains("candidate") || name.contains("import-v1"))
            .collect();

        let start = Arc::new(tokio::sync::Barrier::new(4));
        let check_managed = managed.clone();
        let check_dir = app_data_dir.clone();
        let check_start = start.clone();
        let check = tokio::spawn(async move {
            check_start.wait().await;
            let _gate = acquire_app_data_gate(&check_dir).await.unwrap();
            if check_managed.lock().unwrap().is_some() {
                Ok::<bool, sqlx::Error>(false)
            } else {
                DatabaseManager::is_first_launch_already_gated(&check_dir).await
            }
        });

        let init_managed = managed.clone();
        let init_dir = app_data_dir.clone();
        let init_current = current_path.clone();
        let init_legacy = legacy_path.clone();
        let init_start = start.clone();
        let init = tokio::spawn(async move {
            init_start.wait().await;
            let _gate = acquire_app_data_gate(&init_dir).await.unwrap();
            if init_managed.lock().unwrap().is_some() {
                return Err::<(), String>("already managed".to_string());
            }
            let manager =
                DatabaseManager::new_already_gated(&init_current, &init_legacy, &init_dir)
                    .await
                    .map_err(|error| error.to_string())?;
            let mut state = init_managed.lock().unwrap();
            if state.is_some() {
                return Err("state rejected duplicate".to_string());
            }
            *state = Some(manager);
            Ok(())
        });

        let import_managed = managed.clone();
        let import_dir = app_data_dir.clone();
        let import_source = selected_source.clone();
        let import_start = start.clone();
        let import = tokio::spawn(async move {
            import_start.wait().await;
            let _gate = acquire_app_data_gate(&import_dir).await.unwrap();
            if import_managed.lock().unwrap().is_some() {
                return Err::<(), String>("already managed".to_string());
            }
            let manager = DatabaseManager::import_legacy_database_to_app_data_already_gated(
                &import_source,
                &import_dir,
            )
            .await
            .map_err(|error| error.to_string())?;
            let mut state = import_managed.lock().unwrap();
            if state.is_some() {
                return Err("state rejected duplicate".to_string());
            }
            *state = Some(manager);
            Ok(())
        });

        start.wait().await;
        release_tx.send(()).unwrap();
        startup.await.unwrap();
        assert!(!check.await.unwrap().unwrap());
        assert!(init.await.unwrap().is_err());
        assert!(import.await.unwrap().is_err());

        let manager = managed.lock().unwrap().take().unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM _sqlx_migrations")
                .fetch_one(manager.pool())
                .await
                .unwrap(),
            sqlx::migrate!("./migrations").iter().count() as i64
        );
        assert_eq!(trio(&selected_source), source_before);
        assert_eq!(trio(&current_path), current_before);
        assert_eq!(trio(&legacy_path), legacy_before);
        let artifacts_after: Vec<String> = std::fs::read_dir(&app_data_dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name.contains("candidate") || name.contains("import-v1"))
            .collect();
        assert_eq!(artifacts_after, artifacts_before);
        manager.cleanup().await.unwrap();
    }
}
