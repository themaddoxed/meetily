//! Tauri boundary for the bundle-identity migration core.
//!
//! The commands keep the final identity explicit, write only below the
//! current app-data directory, and leave an incomplete journal behind when a
//! publish fails. A later call can resume that operation after the normal
//! startup guard has refused to open the profile.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

use super::migration::{
    acquire_migration_lock, check_migration_budget_at_path, ensure_real_directory, load_journal,
    migration_operation_paths, persist_journal, publish_manifest, resume_published_manifest,
    stage_manifest, update_journal_phase, validate_manifest, verify_published_manifest,
    MigrationBudget, MigrationError, MigrationFile, MigrationJournal, MigrationPhase,
    MigrationRoots,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationSourceRoots {
    pub legacy_app_data: PathBuf,
    pub legacy_named_config: PathBuf,
    pub legacy_named_data: PathBuf,
    pub web_export: PathBuf,
}

impl MigrationSourceRoots {
    fn into_roots(self, target_app_data: PathBuf) -> MigrationRoots {
        MigrationRoots {
            legacy_app_data: self.legacy_app_data,
            legacy_named_config: self.legacy_named_config,
            legacy_named_data: self.legacy_named_data,
            web_export: self.web_export,
            target_app_data,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareMigrationRequest {
    /// Supplying an ID makes retries idempotent. Omitting it creates a new
    /// operation and returns the generated UUID to the caller.
    pub operation_id: Option<Uuid>,
    pub source_identity: String,
    pub target_identity: String,
    pub roots: MigrationSourceRoots,
    pub files: Vec<MigrationFile>,
    pub budget: MigrationBudget,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishMigrationRequest {
    pub operation_id: Uuid,
    pub roots: MigrationSourceRoots,
    pub budget: MigrationBudget,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationStatus {
    pub operation_id: Uuid,
    pub source_identity: String,
    pub target_identity: String,
    pub phase: MigrationPhase,
    pub file_count: usize,
}

fn command_error(error: MigrationError) -> String {
    error.to_string()
}

fn target_app_data(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|error| format!("Failed to get app data dir: {error}"))
}

fn ensure_target_app_data(path: &Path) -> Result<(), MigrationError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            Err(MigrationError::UnsafeDestination(path.to_path_buf()))
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            ensure_real_directory(path).map(|_| ())
        }
        Err(error) => Err(MigrationError::Io(error)),
    }
}

fn journal_for_request(
    request: &PrepareMigrationRequest,
) -> Result<MigrationJournal, MigrationError> {
    let journal = MigrationJournal {
        format_version: super::migration::MIGRATION_FORMAT_VERSION,
        operation_id: request.operation_id.unwrap_or_else(Uuid::new_v4),
        source_identity: request.source_identity.clone(),
        target_identity: request.target_identity.clone(),
        phase: MigrationPhase::Preparing,
        files: request.files.clone(),
    };
    validate_manifest(&journal)?;
    Ok(journal)
}

fn prepare_migration(
    target_app_data: PathBuf,
    request: PrepareMigrationRequest,
) -> Result<MigrationJournal, MigrationError> {
    ensure_target_app_data(&target_app_data)?;
    let journal = journal_for_request(&request)?;
    let paths = migration_operation_paths(&target_app_data, journal.operation_id)?;
    let migration_root = paths
        .lock
        .parent()
        .ok_or_else(|| MigrationError::InvalidManifest("migration lock has no parent".into()))?;
    ensure_real_directory(migration_root)?;

    let _lock = acquire_migration_lock(&paths.lock, journal.operation_id)?;
    if paths.journal.exists() {
        let existing = load_journal(&paths.journal)?;
        if existing.source_identity != journal.source_identity
            || existing.target_identity != journal.target_identity
            || existing.files != journal.files
        {
            return Err(MigrationError::DestinationExists(paths.journal));
        }
        return match existing.phase {
            MigrationPhase::Prepared
            | MigrationPhase::Publishing
            | MigrationPhase::Verified
            | MigrationPhase::Completed => Ok(existing),
            MigrationPhase::Preparing => Err(MigrationError::InvalidManifest(
                "migration operation is already preparing; inspect or recover it before retrying"
                    .into(),
            )),
        };
    }

    ensure_real_directory(&paths.operation_root)?;
    ensure_real_directory(&paths.staging)?;
    let roots = request.roots.into_roots(target_app_data.clone());
    check_migration_budget_at_path(&target_app_data, request.budget)?;
    persist_journal(&paths.journal, &journal)?;
    stage_manifest(&journal, &roots, &paths.staging)?;
    update_journal_phase(&paths.journal, MigrationPhase::Prepared)?;
    load_journal(&paths.journal)
}

fn publish_migration(
    target_app_data: PathBuf,
    request: PublishMigrationRequest,
) -> Result<MigrationJournal, MigrationError> {
    ensure_target_app_data(&target_app_data)?;
    let paths = migration_operation_paths(&target_app_data, request.operation_id)?;
    let migration_root = paths
        .lock
        .parent()
        .ok_or_else(|| MigrationError::InvalidManifest("migration lock has no parent".into()))?;
    ensure_real_directory(migration_root)?;
    let _lock = acquire_migration_lock(&paths.lock, request.operation_id)?;
    let mut journal = load_journal(&paths.journal)?;
    if journal.operation_id != request.operation_id {
        return Err(MigrationError::InvalidManifest(
            "migration operation id does not match its journal".into(),
        ));
    }
    let roots = request.roots.into_roots(target_app_data.clone());

    match journal.phase {
        MigrationPhase::Preparing => {
            return Err(MigrationError::InvalidManifest(
                "migration must be prepared before publishing".into(),
            ));
        }
        MigrationPhase::Prepared => {
            check_migration_budget_at_path(&target_app_data, request.budget)?;
            update_journal_phase(&paths.journal, MigrationPhase::Publishing)?;
            journal.phase = MigrationPhase::Publishing;
            publish_manifest(&journal, &roots, &paths.staging)?;
        }
        MigrationPhase::Publishing => {
            check_migration_budget_at_path(&target_app_data, request.budget)?;
            resume_published_manifest(&journal, &roots, &paths.staging)?;
        }
        MigrationPhase::Verified => {
            verify_published_manifest(&journal, &target_app_data)?;
            update_journal_phase(&paths.journal, MigrationPhase::Completed)?;
            return load_journal(&paths.journal);
        }
        MigrationPhase::Completed => return Ok(journal),
    }

    verify_published_manifest(&journal, &target_app_data)?;
    update_journal_phase(&paths.journal, MigrationPhase::Verified)?;
    update_journal_phase(&paths.journal, MigrationPhase::Completed)?;
    load_journal(&paths.journal)
}

fn list_migration_status(target_app_data: PathBuf) -> Result<Vec<MigrationStatus>, MigrationError> {
    ensure_target_app_data(&target_app_data)?;
    let migration_root = target_app_data.join(super::migration::MIGRATION_NAMESPACE_DIR);
    let metadata = match fs::symlink_metadata(&migration_root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(MigrationError::Io(error)),
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(MigrationError::UnsafeJournal(migration_root));
    }

    let mut statuses = Vec::new();
    for entry in fs::read_dir(&migration_root)? {
        let entry = entry?;
        let path = entry.path();
        let entry_metadata = fs::symlink_metadata(&path)?;
        if entry_metadata.file_type().is_symlink() || !entry_metadata.is_dir() {
            continue;
        }
        let journal_path = path.join(super::migration::MIGRATION_JOURNAL_FILE);
        if !journal_path.exists() {
            continue;
        }
        let journal = load_journal(&journal_path)?;
        statuses.push(MigrationStatus {
            operation_id: journal.operation_id,
            source_identity: journal.source_identity,
            target_identity: journal.target_identity,
            phase: journal.phase,
            file_count: journal.files.len(),
        });
    }
    statuses.sort_by_key(|status| status.operation_id);
    Ok(statuses)
}

/// Stage and authenticate one explicit migration manifest. No destination
/// profile data is overwritten by this command.
#[tauri::command]
pub async fn prepare_profile_migration(
    app: AppHandle,
    request: PrepareMigrationRequest,
) -> Result<MigrationJournal, String> {
    let target = target_app_data(&app)?;
    tauri::async_runtime::spawn_blocking(move || prepare_migration(target, request))
        .await
        .map_err(|error| format!("Migration worker failed: {error}"))?
        .map_err(command_error)
}

/// Publish or resume one prepared migration. Existing destination files are
/// accepted only when their manifest size and SHA-256 match exactly.
#[tauri::command]
pub async fn publish_profile_migration(
    app: AppHandle,
    request: PublishMigrationRequest,
) -> Result<MigrationJournal, String> {
    let target = target_app_data(&app)?;
    tauri::async_runtime::spawn_blocking(move || publish_migration(target, request))
        .await
        .map_err(|error| format!("Migration worker failed: {error}"))?
        .map_err(command_error)
}

/// Return durable migration phases without opening the normal database.
#[tauri::command]
pub async fn get_profile_migration_status(app: AppHandle) -> Result<Vec<MigrationStatus>, String> {
    let target = target_app_data(&app)?;
    tauri::async_runtime::spawn_blocking(move || list_migration_status(target))
        .await
        .map_err(|error| format!("Migration worker failed: {error}"))?
        .map_err(command_error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn prepare_publish_and_resume_are_idempotent() {
        let directory = tempdir().unwrap();
        let target = directory.path().join("target");
        let legacy = directory.path().join("legacy");
        let config = directory.path().join("config");
        let named_data = directory.path().join("named-data");
        let web_export = directory.path().join("web-export");
        for root in [&legacy, &config, &named_data, &web_export] {
            fs::create_dir_all(root).unwrap();
        }
        let source = legacy.join("profile.txt");
        fs::write(&source, b"profile data").unwrap();
        let (byte_length, sha256) = super::super::migration::sha256_file(&source).unwrap();
        let operation_id = Uuid::new_v4();
        let request = PrepareMigrationRequest {
            operation_id: Some(operation_id),
            source_identity: "com.example.legacy".into(),
            target_identity: "com.example.new".into(),
            roots: MigrationSourceRoots {
                legacy_app_data: legacy.clone(),
                legacy_named_config: config.clone(),
                legacy_named_data: named_data.clone(),
                web_export: web_export.clone(),
            },
            files: vec![MigrationFile {
                namespace: super::super::migration::MigrationNamespace::LegacyAppData,
                source_relative_path: PathBuf::from("profile.txt"),
                target_relative_path: PathBuf::from("profile.txt"),
                byte_length,
                sha256,
            }],
            budget: MigrationBudget {
                sqlite_bytes: 0,
                file_bytes: byte_length,
                web_export_bytes: 0,
            },
        };

        let prepared = prepare_migration(target.clone(), request.clone()).unwrap();
        assert_eq!(prepared.phase, MigrationPhase::Prepared);
        let prepared_again = prepare_migration(target.clone(), request).unwrap();
        assert_eq!(prepared_again, prepared);

        let publish_request = PublishMigrationRequest {
            operation_id,
            roots: MigrationSourceRoots {
                legacy_app_data: legacy,
                legacy_named_config: config,
                legacy_named_data: named_data,
                web_export,
            },
            budget: MigrationBudget {
                sqlite_bytes: 0,
                file_bytes: byte_length,
                web_export_bytes: 0,
            },
        };
        let completed = publish_migration(target.clone(), publish_request.clone()).unwrap();
        assert_eq!(completed.phase, MigrationPhase::Completed);
        assert_eq!(
            fs::read(target.join("profile.txt")).unwrap(),
            b"profile data"
        );
        let completed_again = publish_migration(target, publish_request).unwrap();
        assert_eq!(completed_again.phase, MigrationPhase::Completed);
    }
}
