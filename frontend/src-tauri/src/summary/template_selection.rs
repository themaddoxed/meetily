use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager, Runtime};
use uuid::Uuid;

pub const DEFAULT_TEMPLATE_ID: &str = "standard_meeting";
const SELECTION_FILE_NAME: &str = "selected_summary_template.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateSelection {
    pub template_id: String,
    pub replaced: bool,
    pub previous_template_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredTemplateSelection {
    template_id: String,
}

pub fn selection_path_for_app<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|directory| directory.join(SELECTION_FILE_NAME))
        .map_err(|error| {
            format!(
                "Could not resolve the application data directory: {}",
                error
            )
        })
}

pub fn load_selection_at_app<R: Runtime>(
    app: &AppHandle<R>,
    available_template_ids: &[String],
) -> Result<TemplateSelection, String> {
    let path = selection_path_for_app(app)?;
    load_selection_at(&path, available_template_ids)
}

pub fn save_selection_at_app<R: Runtime>(
    app: &AppHandle<R>,
    template_id: &str,
    available_template_ids: &[String],
) -> Result<TemplateSelection, String> {
    let path = selection_path_for_app(app)?;
    save_selection_at(&path, template_id, available_template_ids)
}

pub fn resolve_template_id_at_app<R: Runtime>(
    app: &AppHandle<R>,
    requested_template_id: Option<&str>,
    available_template_ids: &[String],
) -> Result<String, String> {
    let path = selection_path_for_app(app)?;
    resolve_template_id_at(&path, requested_template_id, available_template_ids)
}

pub fn resolve_template_id_at(
    path: &Path,
    requested_template_id: Option<&str>,
    available_template_ids: &[String],
) -> Result<String, String> {
    let selection = load_selection_at(path, available_template_ids)?;
    if let Some(requested) = requested_template_id
        .map(str::trim)
        .filter(|id| !id.is_empty())
    {
        if available_template_ids.iter().any(|id| id == requested) {
            return Ok(requested.to_string());
        }
    }
    Ok(selection.template_id)
}

pub fn load_selection_at(
    path: &Path,
    available_template_ids: &[String],
) -> Result<TemplateSelection, String> {
    let stored = match fs::read_to_string(path) {
        Ok(content) => serde_json::from_str::<StoredTemplateSelection>(&content).ok(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            return Err(format!(
                "Failed to read selected summary template: {}",
                error
            ));
        }
    };
    let previous_template_id = stored
        .as_ref()
        .map(|selection| selection.template_id.clone());
    let selected = stored
        .as_ref()
        .map(|selection| selection.template_id.as_str())
        .filter(|id| {
            available_template_ids
                .iter()
                .any(|available| available == id)
        })
        .map(str::to_string)
        .unwrap_or_else(|| fallback_template_id(available_template_ids));
    let replaced = previous_template_id.as_deref() != Some(selected.as_str());

    if replaced || !path.is_file() {
        write_selection_atomically(path, &selected)?;
    }

    Ok(TemplateSelection {
        template_id: selected,
        replaced,
        previous_template_id,
    })
}

pub fn save_selection_at(
    path: &Path,
    template_id: &str,
    available_template_ids: &[String],
) -> Result<TemplateSelection, String> {
    let template_id = template_id.trim();
    if !available_template_ids.iter().any(|id| id == template_id) {
        return Err(format!(
            "Summary template '{}' is not available",
            template_id
        ));
    }

    let previous_template_id = match fs::read_to_string(path) {
        Ok(content) => serde_json::from_str::<StoredTemplateSelection>(&content)
            .ok()
            .map(|selection| selection.template_id),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            return Err(format!(
                "Failed to read selected summary template: {}",
                error
            ));
        }
    };
    write_selection_atomically(path, template_id)?;

    Ok(TemplateSelection {
        template_id: template_id.to_string(),
        replaced: false,
        previous_template_id,
    })
}

fn fallback_template_id(available_template_ids: &[String]) -> String {
    available_template_ids
        .iter()
        .find(|id| id.as_str() == DEFAULT_TEMPLATE_ID)
        .cloned()
        .or_else(|| available_template_ids.first().cloned())
        .unwrap_or_else(|| DEFAULT_TEMPLATE_ID.to_string())
}

fn write_selection_atomically(path: &Path, template_id: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "Selected summary template path has no parent directory".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Failed to create template selection directory: {}", error))?;
    let temporary = parent.join(format!(".{}.{}.tmp", SELECTION_FILE_NAME, Uuid::new_v4()));
    let content = serde_json::to_vec_pretty(&StoredTemplateSelection {
        template_id: template_id.to_string(),
    })
    .map_err(|error| format!("Failed to serialize selected summary template: {}", error))?;

    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|error| format!("Failed to stage selected summary template: {}", error))?;
        file.write_all(&content)
            .map_err(|error| format!("Failed to stage selected summary template: {}", error))?;
        file.sync_all()
            .map_err(|error| format!("Failed to flush selected summary template: {}", error))?;
        fs::rename(&temporary, path)
            .map_err(|error| format!("Failed to save selected summary template: {}", error))?;
        Ok::<(), String>(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::{load_selection_at, save_selection_at, DEFAULT_TEMPLATE_ID};
    use std::fs;
    use tempfile::tempdir;

    fn available(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|id| (*id).to_string()).collect()
    }

    #[test]
    fn missing_selection_uses_and_persists_standard_template() {
        let folder = tempdir().unwrap();
        let path = folder.path().join("selected.json");
        let result =
            load_selection_at(&path, &available(&["daily_standup", DEFAULT_TEMPLATE_ID])).unwrap();

        assert_eq!(result.template_id, DEFAULT_TEMPLATE_ID);
        assert!(result.replaced);
        assert!(result.previous_template_id.is_none());
        assert!(path.is_file());
        assert!(fs::read_to_string(path)
            .unwrap()
            .contains(DEFAULT_TEMPLATE_ID));
    }

    #[test]
    fn persisted_custom_selection_reopens_without_replacement() {
        let folder = tempdir().unwrap();
        let path = folder.path().join("selected.json");
        let ids = available(&["daily_standup", DEFAULT_TEMPLATE_ID, "custom"]);

        save_selection_at(&path, "custom", &ids).unwrap();
        let reopened = load_selection_at(&path, &ids).unwrap();

        assert_eq!(reopened.template_id, "custom");
        assert!(!reopened.replaced);
        assert_eq!(reopened.previous_template_id.as_deref(), Some("custom"));
    }

    #[test]
    fn deleted_selection_falls_back_and_reports_replacement() {
        let folder = tempdir().unwrap();
        let path = folder.path().join("selected.json");
        let with_custom = available(&[DEFAULT_TEMPLATE_ID, "custom"]);

        save_selection_at(&path, "custom", &with_custom).unwrap();
        let reopened = load_selection_at(&path, &available(&[DEFAULT_TEMPLATE_ID])).unwrap();

        assert_eq!(reopened.template_id, DEFAULT_TEMPLATE_ID);
        assert!(reopened.replaced);
        assert_eq!(reopened.previous_template_id.as_deref(), Some("custom"));
        assert!(fs::read_to_string(path)
            .unwrap()
            .contains(DEFAULT_TEMPLATE_ID));
    }
}
