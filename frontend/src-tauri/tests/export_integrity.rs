use app_lib::export::{
    normalize_markdown_destination, sanitize_export_file_name, write_export_atomically,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn export_suggested_name_is_safe_and_has_one_markdown_extension() {
    assert_eq!(
        sanitize_export_file_name("  Release / Plan.md  "),
        "Release - Plan.md"
    );
    assert_eq!(sanitize_export_file_name("notes.MD"), "notes.md");
    assert_eq!(sanitize_export_file_name("Встреча 👋"), "Встреча 👋.md");
}

#[test]
fn empty_or_unsafe_suggested_name_uses_a_stable_fallback() {
    assert_eq!(sanitize_export_file_name("  \n\t  "), "meeting.md");
    assert_eq!(sanitize_export_file_name("../"), "meeting.md");
}

#[test]
fn selected_destination_is_normalized_to_markdown() {
    assert_eq!(
        normalize_markdown_destination(PathBuf::from("/tmp/meeting")),
        PathBuf::from("/tmp/meeting.md")
    );
    assert_eq!(
        normalize_markdown_destination(PathBuf::from("/tmp/meeting.txt")),
        PathBuf::from("/tmp/meeting.md")
    );
    assert_eq!(
        normalize_markdown_destination(PathBuf::from("/tmp/meeting.MD")),
        PathBuf::from("/tmp/meeting.MD")
    );
}

#[test]
fn export_writes_exact_utf8_content_and_leaves_no_temporary_file() {
    let directory = tempfile::tempdir().unwrap();
    let destination = directory.path().join("meeting.md");
    let content = "# Встреча\n\nAlice: Привет 👋\n\n## Summary\nРешение принято.\n";

    write_export_atomically(&destination, content.as_bytes()).unwrap();

    assert_eq!(fs::read(&destination).unwrap(), content.as_bytes());
    let entries: Vec<_> = fs::read_dir(directory.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(entries, vec![destination.file_name().unwrap()]);
}

#[test]
fn export_atomically_replaces_an_existing_file() {
    let directory = tempfile::tempdir().unwrap();
    let destination = directory.path().join("meeting.txt");
    fs::write(&destination, b"old complete transcript").unwrap();

    write_export_atomically(&destination, b"new complete transcript").unwrap();

    assert_eq!(fs::read(&destination).unwrap(), b"new complete transcript");
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[test]
fn export_creates_a_missing_parent_directory() {
    let directory = tempfile::tempdir().unwrap();
    let destination = directory.path().join("exports/meeting/summary.md");

    write_export_atomically(&destination, b"offline summary").unwrap();

    assert_eq!(fs::read(&destination).unwrap(), b"offline summary");
    assert_eq!(
        fs::read_dir(destination.parent().unwrap()).unwrap().count(),
        1
    );
}

#[test]
fn invalid_destination_preserves_existing_sibling_and_cleans_its_temp_file() {
    let directory = tempfile::tempdir().unwrap();
    let sibling = directory.path().join("saved-meeting.md");
    let invalid_destination = directory.path().join("cannot-replace-directory");
    fs::write(&sibling, b"primary meeting data").unwrap();
    fs::create_dir(&invalid_destination).unwrap();

    let error = write_export_atomically(&invalid_destination, b"new export").unwrap_err();

    assert!(!error.to_string().is_empty());
    assert_eq!(fs::read(&sibling).unwrap(), b"primary meeting data");
    let entries: Vec<_> = fs::read_dir(directory.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(entries.len(), 2);
    assert!(entries.contains(&sibling.file_name().unwrap().to_os_string()));
    assert!(entries.contains(&invalid_destination.file_name().unwrap().to_os_string()));
}
