use super::defaults;
use super::types::Template;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

// Global storage for the bundled templates directory path.
static BUNDLED_TEMPLATES_DIR: Lazy<RwLock<Option<PathBuf>>> = Lazy::new(|| RwLock::new(None));

/// Set the bundled templates directory path (called once at app startup).
pub fn set_bundled_templates_dir(path: PathBuf) {
    info!("Bundled templates directory set to: {:?}", path);
    if let Ok(mut dir) = BUNDLED_TEMPLATES_DIR.write() {
        *dir = Some(path);
    }
}

/// Get the user's custom templates directory path.
///
/// Returns the platform-specific application data directory for custom templates:
/// - macOS: ~/Library/Application Support/Meetily/templates/
/// - Windows: %APPDATA%\\Meetily\\templates\\
/// - Linux: ~/.config/Meetily/templates/
fn get_custom_templates_dir() -> Option<PathBuf> {
    let mut path = dirs::data_dir()?;
    path.push("Meetily");
    path.push("templates");
    Some(path)
}

/// Validate an identifier before using it as a filename stem.
///
/// We deliberately reject unsafe path syntax instead of replacing characters.
/// Replacing characters can make two different user identifiers point to the same
/// file. Names generated from a template title use `slugify_template_name`, while
/// existing custom file stems remain usable for backwards compatibility.
fn validate_template_id(template_id: &str) -> Result<String, String> {
    let id = template_id.trim();
    if id.is_empty() {
        return Err("Template id cannot be empty".to_string());
    }
    if id == "." || id == ".." || Path::new(id).is_absolute() {
        return Err("Template id must be a filename, not a path".to_string());
    }
    if id.contains('/') || id.contains('\\') || id.contains('\0') {
        return Err("Template id cannot contain path separators".to_string());
    }
    if id.ends_with(".json") {
        return Err("Template id should omit the .json extension".to_string());
    }
    Ok(id.to_string())
}

fn template_path(directory: &Path, template_id: &str) -> Result<PathBuf, String> {
    let id = validate_template_id(template_id)?;
    Ok(directory.join(format!("{}.json", id)))
}

fn read_optional_template(directory: &Path, template_id: &str) -> Result<Option<String>, String> {
    let path = template_path(directory, template_id)?;
    match fs::read_to_string(&path) {
        Ok(content) => Ok(Some(content)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!(
            "Failed to read template '{}': {}",
            template_id, error
        )),
    }
}

/// Load raw template content using the normal custom > bundled > embedded order.
fn load_template_content_from_dirs(
    template_id: &str,
    custom_dir: Option<&Path>,
    bundled_dir: Option<&Path>,
) -> Result<String, String> {
    validate_template_id(template_id)?;

    if let Some(directory) = custom_dir {
        if let Some(content) = read_optional_template(directory, template_id)? {
            debug!("Using custom template for '{}'", template_id);
            return Ok(content);
        }
    }

    if let Some(directory) = bundled_dir {
        if let Some(content) = read_optional_template(directory, template_id)? {
            debug!("Using bundled template for '{}'", template_id);
            return Ok(content);
        }
    }

    if let Some(content) = defaults::get_builtin_template(template_id) {
        debug!("Using embedded template for '{}'", template_id);
        return Ok(content.to_string());
    }

    Err(format!(
        "Template '{}' not found. Available templates: {}",
        template_id,
        list_template_ids().join(", ")
    ))
}

/// Load a template from the user's custom templates directory.
fn load_custom_template(template_id: &str) -> Result<Option<String>, String> {
    let Some(custom_dir) = get_custom_templates_dir() else {
        return Ok(None);
    };
    read_optional_template(&custom_dir, template_id)
}

/// Load and parse a template by identifier.
///
/// This function implements the fallback strategy required by the product:
/// custom files first, bundled resources second, and embedded defaults last.
pub fn get_template(template_id: &str) -> Result<Template, String> {
    info!("Loading template: {}", template_id);

    let bundled_dir = BUNDLED_TEMPLATES_DIR
        .read()
        .ok()
        .and_then(|dir| dir.clone());
    let custom_content = load_custom_template(template_id)?;
    let json_content = if let Some(content) = custom_content {
        content
    } else {
        load_template_content_from_dirs(template_id, None, bundled_dir.as_deref())?
    };

    validate_and_parse_template(&json_content)
}

/// Validate and parse template JSON before it is used or persisted.
pub fn validate_and_parse_template(json_content: &str) -> Result<Template, String> {
    let template: Template = serde_json::from_str(json_content).map_err(|error| {
        format!(
            "Template JSON is invalid: {}. Check the name, description, and sections.",
            error
        )
    })?;

    template.validate()?;
    Ok(template)
}

fn parse_template_value(json_content: &str) -> Result<Value, String> {
    let value: Value = serde_json::from_str(json_content).map_err(|error| {
        format!(
            "Template JSON is invalid: {}. Check the name, description, and sections.",
            error
        )
    })?;

    if !value.is_object() {
        return Err(
            "Template must be a JSON object with name, description, and sections".to_string(),
        );
    }
    Ok(value)
}

fn merge_section_values(existing: Option<&Value>, incoming: &Value) -> Value {
    let Some(incoming_sections) = incoming.as_array() else {
        return incoming.clone();
    };
    let existing_sections = existing.and_then(Value::as_array);

    Value::Array(
        incoming_sections
            .iter()
            .enumerate()
            .map(|(index, incoming_section)| {
                let Some(incoming_object) = incoming_section.as_object() else {
                    return incoming_section.clone();
                };
                let existing_object = existing_sections
                    .and_then(|sections| {
                        sections.iter().find_map(|section| {
                            let section_object = section.as_object()?;
                            (section_object.get("title") == incoming_object.get("title"))
                                .then_some(section_object)
                        })
                    })
                    .or_else(|| {
                        existing_sections
                            .and_then(|sections| sections.get(index))
                            .and_then(Value::as_object)
                    });
                let Some(existing_object) = existing_object else {
                    return incoming_section.clone();
                };

                let mut merged = existing_object.clone();
                for (key, value) in incoming_object {
                    let is_clearable_optional_field =
                        matches!(key.as_str(), "item_format" | "example_item_format")
                            && (value.is_null()
                                || value
                                    .as_str()
                                    .map(|text| text.trim().is_empty())
                                    .unwrap_or(false));
                    if is_clearable_optional_field {
                        merged.remove(key);
                    } else {
                        merged.insert(key.clone(), value.clone());
                    }
                }
                Value::Object(merged)
            })
            .collect(),
    )
}

/// Merge a structured edit into the old JSON object while retaining unknown
/// user fields at the template and section levels.
fn merge_template_values(existing: &Value, incoming: &Value) -> Result<Value, String> {
    let Some(existing_object) = existing.as_object() else {
        return Err("The existing template is not a JSON object".to_string());
    };
    let Some(incoming_object) = incoming.as_object() else {
        return Err("Template must be a JSON object".to_string());
    };

    let mut merged = existing_object.clone();
    for (key, value) in incoming_object {
        if key == "sections" {
            merged.insert(
                key.clone(),
                merge_section_values(existing_object.get(key), value),
            );
        } else {
            merged.insert(key.clone(), value.clone());
        }
    }
    Ok(Value::Object(merged))
}

fn template_json_for_write(
    path: &Path,
    incoming_json: &str,
    preserve_unknown_fields: bool,
) -> Result<String, String> {
    let incoming = parse_template_value(incoming_json)?;
    let value = if preserve_unknown_fields && path.exists() {
        let existing_json = fs::read_to_string(path).map_err(|error| {
            format!(
                "Failed to read the existing template before editing: {}",
                error
            )
        })?;
        let existing = parse_template_value(&existing_json)?;
        merge_template_values(&existing, &incoming)?
    } else {
        incoming
    };

    let value_json = serde_json::to_string(&value)
        .map_err(|error| format!("Failed to prepare template JSON: {}", error))?;
    validate_and_parse_template(&value_json)?;

    serde_json::to_string_pretty(&value)
        .map(|pretty| format!("{}\n", pretty))
        .map_err(|error| format!("Failed to format template JSON: {}", error))
}

/// Replace a file atomically. A failed write leaves the previous file untouched.
fn replace_file_atomically(path: &Path, content: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "Template path has no parent directory".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Failed to create templates directory: {}", error))?;

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "Template filename is not valid UTF-8".to_string())?;
    let temporary_path = parent.join(format!(".{}.{}.tmp", file_name, Uuid::new_v4()));

    let write_result = (|| {
        let mut temporary_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)
            .map_err(|error| format!("Failed to create temporary template file: {}", error))?;
        temporary_file
            .write_all(content.as_bytes())
            .map_err(|error| format!("Failed to write template: {}", error))?;
        temporary_file
            .sync_all()
            .map_err(|error| format!("Failed to flush template: {}", error))?;
        fs::rename(&temporary_path, path)
            .map_err(|error| format!("Failed to replace template file: {}", error))?;
        Ok(())
    })();

    if write_result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    write_result
}

fn slugify_template_name(name: &str) -> Result<String, String> {
    let mut slug = String::new();
    let mut separator_pending = false;

    for character in name.trim().chars() {
        if character.is_alphanumeric() {
            if separator_pending && !slug.is_empty() {
                slug.push('-');
            }
            for lower in character.to_lowercase() {
                slug.push(lower);
            }
            separator_pending = false;
        } else {
            separator_pending = true;
        }
    }

    let slug: String = slug.chars().take(80).collect();
    if slug.is_empty() {
        return Err("Template name must contain at least one letter or number".to_string());
    }
    Ok(slug)
}

fn unique_template_id(directory: &Path, requested_id: &str) -> Result<String, String> {
    let requested = validate_template_id(requested_id)?;
    let mut candidate = requested.clone();
    let mut suffix = 2_u32;

    while template_path(directory, &candidate)?.exists() {
        candidate = format!("{}-{}", requested, suffix);
        suffix = suffix
            .checked_add(1)
            .ok_or_else(|| "Could not find an unused template id".to_string())?;
    }

    Ok(candidate)
}

fn create_custom_template_at(
    directory: &Path,
    requested_id: Option<&str>,
    json_content: &str,
) -> Result<String, String> {
    let template = validate_and_parse_template(json_content)?;
    let requested_id = match requested_id {
        Some(id) => validate_template_id(id)?,
        None => slugify_template_name(&template.name)?,
    };
    let id = unique_template_id(directory, &requested_id)?;
    let path = template_path(directory, &id)?;
    let prepared = template_json_for_write(&path, json_content, false)?;
    replace_file_atomically(&path, &prepared)?;
    info!("Created custom template '{}' at {:?}", id, path);
    Ok(id)
}

fn update_custom_template_at(
    directory: &Path,
    template_id: &str,
    json_content: &str,
) -> Result<String, String> {
    let id = validate_template_id(template_id)?;
    let path = template_path(directory, &id)?;
    if !path.is_file() {
        return Err(format!(
            "Template '{}' is not a custom template. Copy a built-in template before editing it.",
            id
        ));
    }

    let prepared = template_json_for_write(&path, json_content, true)?;
    replace_file_atomically(&path, &prepared)?;
    info!("Updated custom template '{}' at {:?}", id, path);
    Ok(id)
}

fn copy_template_at(
    custom_directory: &Path,
    bundled_directory: Option<&Path>,
    source_template_id: &str,
    requested_id: Option<&str>,
) -> Result<String, String> {
    let source_id = validate_template_id(source_template_id)?;
    let source_json =
        load_template_content_from_dirs(&source_id, Some(custom_directory), bundled_directory)?;
    let source_template = validate_and_parse_template(&source_json)?;
    let requested_id = match requested_id {
        Some(id) => validate_template_id(id)?,
        None => format!("{}-copy", slugify_template_name(&source_template.name)?),
    };
    let id = unique_template_id(custom_directory, &requested_id)?;
    let path = template_path(custom_directory, &id)?;
    replace_file_atomically(&path, &source_json)?;
    info!(
        "Copied template '{}' to custom template '{}' at {:?}",
        source_id, id, path
    );
    Ok(id)
}

fn delete_custom_template_at(directory: &Path, template_id: &str) -> Result<(), String> {
    let id = validate_template_id(template_id)?;
    let path = template_path(directory, &id)?;
    if !path.is_file() {
        return Err(format!("Custom template '{}' was not found", id));
    }
    fs::remove_file(&path).map_err(|error| format!("Failed to delete template '{}': {}", id, error))
}

/// Create a custom template using a collision-safe id.
pub fn create_custom_template(
    requested_id: Option<&str>,
    json_content: &str,
) -> Result<String, String> {
    let directory = get_custom_templates_dir()
        .ok_or_else(|| "Could not resolve the custom templates directory".to_string())?;
    create_custom_template_at(&directory, requested_id, json_content)
}

/// Copy a built-in, bundled, or custom template into the custom directory.
pub fn copy_template(
    source_template_id: &str,
    requested_id: Option<&str>,
) -> Result<String, String> {
    let custom_directory = get_custom_templates_dir()
        .ok_or_else(|| "Could not resolve the custom templates directory".to_string())?;
    let bundled_directory = BUNDLED_TEMPLATES_DIR
        .read()
        .ok()
        .and_then(|dir| dir.clone());
    copy_template_at(
        &custom_directory,
        bundled_directory.as_deref(),
        source_template_id,
        requested_id,
    )
}

/// Update an existing custom template while retaining unknown user fields.
pub fn update_custom_template(template_id: &str, json_content: &str) -> Result<String, String> {
    let directory = get_custom_templates_dir()
        .ok_or_else(|| "Could not resolve the custom templates directory".to_string())?;
    update_custom_template_at(&directory, template_id, json_content)
}

/// Backwards-compatible explicit overwrite operation for callers that already
/// know the target id. New UI code should use create/update/copy separately.
pub fn save_custom_template(template_id: &str, json_content: &str) -> Result<String, String> {
    let directory = get_custom_templates_dir()
        .ok_or_else(|| "Could not resolve the custom templates directory".to_string())?;
    let id = validate_template_id(template_id)?;
    let path = template_path(&directory, &id)?;
    let prepared = template_json_for_write(&path, json_content, path.exists())?;
    replace_file_atomically(&path, &prepared)?;
    Ok(id)
}

/// Delete a custom template. Removing an override reveals the bundled or
/// embedded template with the same id again.
pub fn delete_custom_template(template_id: &str) -> Result<(), String> {
    let directory = get_custom_templates_dir()
        .ok_or_else(|| "Could not resolve the custom templates directory".to_string())?;
    delete_custom_template_at(&directory, template_id)
}

/// Returns true if the given template id has a user-defined custom file.
pub fn is_custom_template(template_id: &str) -> bool {
    let Ok(id) = validate_template_id(template_id) else {
        return false;
    };
    get_custom_templates_dir()
        .map(|directory| {
            template_path(&directory, &id)
                .map(|path| path.is_file())
                .unwrap_or(false)
        })
        .unwrap_or(false)
}

/// List all available template identifiers.
pub fn list_template_ids() -> Vec<String> {
    let mut ids: Vec<String> = defaults::list_builtin_template_ids()
        .into_iter()
        .map(str::to_string)
        .collect();

    let mut add_directory_ids = |directory: &Path| {
        let entries = match fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(error) => {
                warn!(
                    "Failed to read templates directory {:?}: {}",
                    directory, error
                );
                return;
            }
        };

        for entry in entries.flatten() {
            if !entry
                .file_type()
                .map(|kind| kind.is_file())
                .unwrap_or(false)
            {
                continue;
            }
            let Some(filename) = entry.file_name().to_str().map(str::to_string) else {
                continue;
            };
            let Some(id) = filename.strip_suffix(".json") else {
                continue;
            };
            if !ids.iter().any(|known| known == id) {
                ids.push(id.to_string());
            }
        }
    };

    if let Ok(bundled_directory) = BUNDLED_TEMPLATES_DIR.read() {
        if let Some(directory) = bundled_directory.as_ref().filter(|path| path.exists()) {
            add_directory_ids(directory);
        }
    }
    if let Some(directory) = get_custom_templates_dir().filter(|path| path.exists()) {
        add_directory_ids(&directory);
    }

    ids.sort();
    ids
}

/// List all valid templates with their metadata.
pub fn list_templates() -> Vec<(String, String, String)> {
    let mut templates = Vec::new();

    for id in list_template_ids() {
        match get_template(&id) {
            Ok(template) => templates.push((id, template.name, template.description)),
            Err(error) => warn!("Failed to load template '{}': {}", id, error),
        }
    }

    templates
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    const FALLBACK_LEGACY_TEMPLATE: &str = r#"{
  "name": "Контрольные решения",
  "description": "Синтетический старый пользовательский шаблон для проверки сохранности правок.",
  "sections": [
    {
      "title": "Решения",
      "instruction": "Перечисли только принятые решения из текста встречи. Не добавляй догадок.",
      "format": "list",
      "example_item_format": "Решение — основание"
    }
  ],
  "user_note": "Сохранить это дополнительное поле при открытии и редактировании старого файла."
}"#;

    fn legacy_template_json() -> String {
        fs::read_to_string(
            "/private/tmp/meetily-first-batch-20260908/batch-fixtures/legacy-custom-template.json",
        )
        .unwrap_or_else(|_| FALLBACK_LEGACY_TEMPLATE.to_string())
    }

    fn basic_template_json(name: &str) -> String {
        serde_json::json!({
            "name": name,
            "description": "A test template",
            "sections": [{
                "title": "Summary",
                "instruction": "Summarize the meeting",
                "format": "paragraph"
            }]
        })
        .to_string()
    }

    #[test]
    fn test_get_builtin_template() {
        let template = get_template("daily_standup");
        assert!(template.is_ok());

        let template = template.unwrap();
        assert_eq!(template.name, "Daily Standup");
        assert!(!template.sections.is_empty());
    }

    #[test]
    fn test_get_nonexistent_template() {
        let result = get_template("nonexistent_template");
        assert!(result.is_err());
    }

    #[test]
    fn test_list_template_ids() {
        let ids = list_template_ids();
        assert!(ids.contains(&"daily_standup".to_string()));
        assert!(ids.contains(&"standard_meeting".to_string()));
    }

    #[test]
    fn test_validate_invalid_json() {
        let result = validate_and_parse_template("invalid json");
        assert!(result.is_err());
    }

    #[test]
    fn custom_template_lifecycle_preserves_legacy_fields_and_fallback() {
        let directory = tempdir().unwrap();
        let legacy = legacy_template_json();

        let created_id = create_custom_template_at(directory.path(), None, &legacy).unwrap();
        let created_path = template_path(directory.path(), &created_id).unwrap();
        let original_bytes = fs::read(&created_path).unwrap();

        // Reading an old file is read-only and does not rewrite its bytes.
        let loaded =
            load_template_content_from_dirs(&created_id, Some(directory.path()), None).unwrap();
        assert_eq!(loaded.as_bytes(), original_bytes.as_slice());
        let parsed = validate_and_parse_template(&loaded).unwrap();
        assert_eq!(
            parsed.sections[0].example_item_format.as_deref(),
            Some("Решение — основание")
        );

        let edited = basic_template_json("Контрольные решения обновлены");
        update_custom_template_at(directory.path(), &created_id, &edited).unwrap();
        let reopened = fs::read_to_string(&created_path).unwrap();
        let reopened_value: Value = serde_json::from_str(&reopened).unwrap();
        assert_eq!(
            reopened_value.get("user_note").and_then(Value::as_str),
            Some("Сохранить это дополнительное поле при открытии и редактировании старого файла.")
        );
        assert_eq!(
            reopened_value["sections"][0]
                .get("example_item_format")
                .and_then(Value::as_str),
            Some("Решение — основание")
        );

        let second_id = create_custom_template_at(directory.path(), None, &legacy).unwrap();
        assert_ne!(created_id, second_id);
        assert!(second_id.ends_with("-2"));

        let copied_id =
            copy_template_at(directory.path(), None, &created_id, Some("copied-template")).unwrap();
        assert_eq!(copied_id, "copied-template");
        assert_eq!(
            fs::read_to_string(template_path(directory.path(), &copied_id).unwrap()).unwrap(),
            reopened
        );

        let override_id = create_custom_template_at(
            directory.path(),
            Some("standard_meeting"),
            &basic_template_json("Custom standard"),
        )
        .unwrap();
        assert_eq!(override_id, "standard_meeting");
        assert_eq!(
            validate_and_parse_template(
                &load_template_content_from_dirs(&override_id, Some(directory.path()), None)
                    .unwrap()
            )
            .unwrap()
            .name,
            "Custom standard"
        );
        delete_custom_template_at(directory.path(), &override_id).unwrap();
        assert_eq!(
            validate_and_parse_template(
                &load_template_content_from_dirs(&override_id, Some(directory.path()), None)
                    .unwrap()
            )
            .unwrap()
            .name,
            "Standard Meeting Notes"
        );
    }

    #[test]
    fn custom_template_update_clears_known_optional_fields_and_keeps_unknown_fields() {
        let directory = tempdir().unwrap();
        let original = serde_json::json!({
            "name": "Tasks",
            "description": "Assigned work",
            "sections": [{
                "title": "Action items",
                "instruction": "List assigned work",
                "format": "list",
                "item_format": "OLD ITEM",
                "example_item_format": "OLD EXAMPLE",
                "section_note": "Keep this unknown section field"
            }],
            "user_note": "Keep this unknown template field"
        })
        .to_string();
        let id = create_custom_template_at(directory.path(), Some("tasks"), &original).unwrap();

        let cleared = serde_json::json!({
            "name": "Tasks updated",
            "description": "Assigned work",
            "sections": [{
                "title": "Action items",
                "instruction": "List assigned work",
                "format": "list",
                "item_format": null,
                "example_item_format": null
            }]
        })
        .to_string();
        update_custom_template_at(directory.path(), &id, &cleared).unwrap();

        let reopened: Value = serde_json::from_str(
            &fs::read_to_string(template_path(directory.path(), &id).unwrap()).unwrap(),
        )
        .unwrap();
        let reopened_section = &reopened["sections"][0];
        assert!(reopened_section.get("item_format").is_none());
        assert!(reopened_section.get("example_item_format").is_none());
        assert_eq!(
            reopened_section.get("section_note").and_then(Value::as_str),
            Some("Keep this unknown section field")
        );
        assert_eq!(
            reopened.get("user_note").and_then(Value::as_str),
            Some("Keep this unknown template field")
        );
    }

    #[test]
    fn failed_atomic_write_keeps_existing_file_and_rejects_unsafe_ids() {
        let directory = tempdir().unwrap();
        let existing_path = directory.path().join("existing.json");
        fs::write(&existing_path, "previous content").unwrap();
        let failed_path = existing_path.join("child.json");
        assert!(replace_file_atomically(&failed_path, "new content").is_err());
        assert_eq!(
            fs::read_to_string(&existing_path).unwrap(),
            "previous content"
        );

        let error = create_custom_template_at(
            directory.path(),
            Some("../escape"),
            &basic_template_json("Unsafe"),
        )
        .unwrap_err();
        assert!(error.contains("filename") || error.contains("separators"));
        assert!(!directory.path().join("escape.json").exists());
    }
}
