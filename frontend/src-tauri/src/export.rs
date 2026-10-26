use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::{AppHandle, Runtime};
use tauri_plugin_dialog::DialogExt;

static NEXT_EXPORT_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ExportMeetingOutcome {
    Saved,
    Cancelled,
}

pub fn sanitize_export_file_name(suggested_name: &str) -> String {
    let replaced = suggested_name
        .trim()
        .chars()
        .map(|character| match character {
            '/' | '\\' | ':' => '-',
            character if character.is_control() => ' ',
            character => character,
        })
        .collect::<String>();
    let collapsed = replaced.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = collapsed.trim_matches([' ', '.', '-', '_']);
    let stem = if trimmed.to_ascii_lowercase().ends_with(".md") {
        &trimmed[..trimmed.len() - 3]
    } else {
        trimmed
    };
    let stem = stem.trim_matches([' ', '.', '-', '_']);

    if stem.is_empty() {
        "meeting.md".to_string()
    } else {
        format!("{stem}.md")
    }
}

pub fn normalize_markdown_destination(mut destination: PathBuf) -> PathBuf {
    let has_markdown_extension = destination
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("md"));

    if !has_markdown_extension {
        destination.set_extension("md");
    }

    destination
}

#[tauri::command]
pub async fn export_meeting_markdown<R: Runtime>(
    app: AppHandle<R>,
    suggested_name: String,
    content: String,
) -> Result<ExportMeetingOutcome, String> {
    let file_name = sanitize_export_file_name(&suggested_name);
    let selected_path = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .add_filter("Markdown", &["md"])
            .set_file_name(&file_name)
            .blocking_save_file()
    })
    .await
    .map_err(|error| {
        log::error!("Meeting export dialog task failed: {error}");
        "Unable to export meeting".to_string()
    })?;

    let Some(selected_path) = selected_path else {
        return Ok(ExportMeetingOutcome::Cancelled);
    };

    let destination = normalize_markdown_destination(PathBuf::from(selected_path.to_string()));
    write_export_atomically(&destination, content.as_bytes()).map_err(|error| {
        log::error!("Meeting export write failed: {error}");
        "Unable to export meeting".to_string()
    })?;

    Ok(ExportMeetingOutcome::Saved)
}

pub fn write_export_atomically(destination: &Path, content: &[u8]) -> io::Result<()> {
    let parent = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;

    let (mut temporary_file, temporary_path) = create_temporary_file(destination, parent)?;
    let result = (|| {
        temporary_file.write_all(content)?;
        temporary_file.sync_all()?;
        drop(temporary_file);
        fs::rename(&temporary_path, destination)
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }

    result
}

fn create_temporary_file(destination: &Path, parent: &Path) -> io::Result<(fs::File, PathBuf)> {
    let file_name = destination.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "export destination has no file name",
        )
    })?;

    for _ in 0..32 {
        let id = NEXT_EXPORT_ID.fetch_add(1, Ordering::Relaxed);
        let temporary_name = format!(
            ".{}.meetily-export-{}-{}.tmp",
            file_name.to_string_lossy(),
            std::process::id(),
            id
        );
        let temporary_path = parent.join(temporary_name);

        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)
        {
            Ok(file) => return Ok((file, temporary_path)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate a unique export temporary file",
    ))
}
