use crate::summary::templates;
use serde::{Deserialize, Serialize};
use tauri::Runtime;
use tracing::{info, warn};

/// Template metadata for the selector and editor.
#[derive(Debug, Serialize, Deserialize)]
pub struct TemplateInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub is_custom: bool,
}

/// Full structured template data used by the editor.
#[derive(Debug, Serialize, Deserialize)]
pub struct TemplateDetails {
    pub id: String,
    pub name: String,
    pub description: String,
    pub sections: Vec<templates::TemplateSection>,
    pub is_custom: bool,
}

/// Lists all valid templates from custom files, bundled resources, and embedded defaults.
#[tauri::command]
pub async fn api_list_templates<R: Runtime>(
    _app: tauri::AppHandle<R>,
) -> Result<Vec<TemplateInfo>, String> {
    info!("api_list_templates called");

    let template_infos = templates::list_templates()
        .into_iter()
        .map(|(id, name, description)| TemplateInfo {
            is_custom: templates::is_custom_template(&id),
            id,
            name,
            description,
        })
        .collect::<Vec<_>>();

    info!("Found {} available templates", template_infos.len());
    Ok(template_infos)
}

/// Gets the complete structured data for one template.
#[tauri::command]
pub async fn api_get_template_details<R: Runtime>(
    _app: tauri::AppHandle<R>,
    template_id: String,
) -> Result<TemplateDetails, String> {
    info!(
        "api_get_template_details called for template_id: {}",
        template_id
    );

    let template = templates::get_template(&template_id)?;
    Ok(TemplateDetails {
        id: template_id.clone(),
        name: template.name,
        description: template.description,
        sections: template.sections,
        is_custom: templates::is_custom_template(&template_id),
    })
}

/// Validates a structured template JSON string without writing it.
#[tauri::command]
pub async fn api_validate_template<R: Runtime>(
    _app: tauri::AppHandle<R>,
    template_json: String,
) -> Result<String, String> {
    info!("api_validate_template called");

    match templates::validate_and_parse_template(&template_json) {
        Ok(template) => {
            info!("Template '{}' validated successfully", template.name);
            Ok(template.name)
        }
        Err(error) => {
            warn!("Template validation failed: {}", error);
            Err(error)
        }
    }
}

/// Creates a custom template. When no id is supplied, the backend derives one
/// from the name and adds a numeric suffix if that id already exists.
#[tauri::command]
pub async fn api_create_custom_template<R: Runtime>(
    _app: tauri::AppHandle<R>,
    template_id: Option<String>,
    template_json: String,
) -> Result<String, String> {
    info!("api_create_custom_template called");
    templates::create_custom_template(template_id.as_deref(), &template_json)
}

/// Copies any available template into a new custom file. The target id is
/// collision-safe and never overwrites an existing custom file.
#[tauri::command]
pub async fn api_copy_template<R: Runtime>(
    _app: tauri::AppHandle<R>,
    source_template_id: String,
    requested_template_id: Option<String>,
) -> Result<String, String> {
    info!(
        "api_copy_template called for source '{}'",
        source_template_id
    );
    templates::copy_template(&source_template_id, requested_template_id.as_deref())
}

/// Updates an existing custom file. Built-in and bundled templates must be
/// copied first, which keeps their files immutable.
#[tauri::command]
pub async fn api_update_custom_template<R: Runtime>(
    _app: tauri::AppHandle<R>,
    template_id: String,
    template_json: String,
) -> Result<String, String> {
    info!(
        "api_update_custom_template called for template '{}'",
        template_id
    );
    templates::update_custom_template(&template_id, &template_json)
}

/// Backwards-compatible explicit overwrite operation. New UI code uses the
/// create, copy, and update commands to make intent clear.
#[tauri::command]
pub async fn api_save_custom_template<R: Runtime>(
    _app: tauri::AppHandle<R>,
    template_id: String,
    template_json: String,
) -> Result<String, String> {
    info!("api_save_custom_template called for id: {}", template_id);
    templates::save_custom_template(&template_id, &template_json)
}

/// Deletes a custom user template. Removing a custom override restores the
/// bundled or embedded template with the same id.
#[tauri::command]
pub async fn api_delete_custom_template<R: Runtime>(
    _app: tauri::AppHandle<R>,
    template_id: String,
) -> Result<(), String> {
    info!("api_delete_custom_template called for id: {}", template_id);
    templates::delete_custom_template(&template_id)
}

/// Returns whether a template id has a custom file and can be edited/deleted.
#[tauri::command]
pub async fn api_is_custom_template<R: Runtime>(
    _app: tauri::AppHandle<R>,
    template_id: String,
) -> Result<bool, String> {
    Ok(templates::is_custom_template(&template_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_template_returns_name() {
        let valid_json = r#"
        {
            "name": "Test Template",
            "description": "A test template",
            "sections": [
                {
                    "title": "Summary",
                    "instruction": "Provide a summary",
                    "format": "paragraph"
                }
            ]
        }"#;

        let result = templates::validate_and_parse_template(valid_json).unwrap();
        assert_eq!(result.name, "Test Template");
    }

    #[test]
    fn validate_template_returns_useful_error() {
        let result = templates::validate_and_parse_template("invalid json");
        let error = result.unwrap_err();
        assert!(error.contains("Template JSON is invalid"));
    }
}
