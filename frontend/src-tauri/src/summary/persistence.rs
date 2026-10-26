use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Atomically publish the generated Markdown summary beside its recording.
pub fn save_summary_markdown(folder: &Path, markdown: &str) -> Result<PathBuf, String> {
    if markdown.trim().is_empty() {
        return Err("Generated summary is empty; the previous summary was preserved".to_string());
    }
    if markdown.contains('\0') {
        return Err("Generated summary contains an invalid NUL character; the previous summary was preserved".to_string());
    }
    if !folder.is_dir() {
        return Err(format!(
            "Recording folder '{}' is unavailable; the previous summary was preserved",
            folder.display()
        ));
    }

    let destination = folder.join("summary.md");
    replace_summary_bytes(folder, &destination, markdown.as_bytes())?;
    Ok(destination)
}

fn replace_summary_bytes(folder: &Path, destination: &Path, content: &[u8]) -> Result<(), String> {
    let temporary = folder.join(format!(".summary.{}.tmp", Uuid::new_v4()));
    let write_result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|error| format!("Failed to stage summary: {}", error))?;
        file.write_all(content)
            .map_err(|error| format!("Failed to stage summary: {}", error))?;
        file.sync_all()
            .map_err(|error| format!("Failed to flush staged summary: {}", error))?;
        fs::rename(&temporary, destination)
            .map_err(|error| format!("Failed to publish summary.md atomically: {}", error))?;
        sync_parent_directory(folder);
        Ok::<(), String>(())
    })();

    if let Err(error) = write_result {
        let _ = fs::remove_file(&temporary);
        return Err(format!("{}; the previous summary was preserved", error));
    }

    Ok(())
}

fn sync_parent_directory(folder: &Path) {
    // Directory sync is supported on Unix and helps make the rename durable.
    // Some platforms/filesystems reject opening directories; the atomic rename
    // still provides the no-duplicate/no-partial-file guarantee there.
    if let Ok(directory) = File::open(folder) {
        let _ = directory.sync_all();
    }
}

#[cfg(test)]
mod tests {
    use super::save_summary_markdown;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn repeated_saves_replace_one_summary_file() {
        let folder = tempdir().expect("temporary recording folder");

        let first = save_summary_markdown(folder.path(), "# First summary\n").unwrap();
        let second = save_summary_markdown(folder.path(), "# Second summary\n").unwrap();

        assert_eq!(first, folder.path().join("summary.md"));
        assert_eq!(second, first);
        assert_eq!(fs::read_to_string(&second).unwrap(), "# Second summary\n");
        assert_eq!(fs::read_dir(folder.path()).unwrap().count(), 1);
    }

    #[test]
    fn invalid_output_preserves_previous_summary() {
        let folder = tempdir().expect("temporary recording folder");
        let summary_path = save_summary_markdown(folder.path(), "# Previous summary\n").unwrap();

        let error = save_summary_markdown(folder.path(), "   \n\t").unwrap_err();

        assert!(error.contains("empty"));
        assert_eq!(
            fs::read_to_string(summary_path).unwrap(),
            "# Previous summary\n"
        );
    }

    #[test]
    fn unavailable_recording_folder_does_not_create_output() {
        let parent = tempdir().expect("temporary parent folder");
        let file_path = parent.path().join("recording-placeholder");
        fs::write(&file_path, "existing").unwrap();

        let error = save_summary_markdown(&file_path, "# Summary\n").unwrap_err();

        assert!(error.contains("Recording folder"));
        assert_eq!(fs::read_to_string(&file_path).unwrap(), "existing");
        assert_eq!(fs::read_dir(parent.path()).unwrap().count(), 1);
    }
}
