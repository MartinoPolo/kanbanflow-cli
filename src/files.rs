//! Turning server-supplied attachment names into safe, collision-free paths.
//!
//! Attachment names come from the API and are attacker-influenced on a shared
//! board, so they are reduced to a single harmless file name before any write.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// A path inside `dir` that neither an earlier download in this run nor an
/// earlier run already claimed.
///
/// Downloads truncate whatever they open, so a name already on disk has to be
/// skipped as well: re-running `--download-attachments` into the same directory
/// must never destroy the files the previous run saved.
pub fn unique_destination(dir: &Path, name: &str, used: &mut HashSet<String>) -> PathBuf {
    let base = safe_file_name(name);
    let (stem, extension) = match base.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() => (stem.to_string(), format!(".{extension}")),
        _ => (base.clone(), String::new()),
    };
    let mut candidate = base;
    let mut counter = 1;
    while !used.insert(candidate.clone()) || dir.join(&candidate).exists() {
        counter += 1;
        candidate = format!("{stem} ({counter}){extension}");
    }
    dir.join(candidate)
}

/// Reduce a server-supplied name to one file name: directory components are
/// dropped (traversal protection) and characters Windows rejects are replaced.
pub fn safe_file_name(name: &str) -> String {
    let last = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let sanitized: String = last
        .chars()
        .map(|character| match character {
            ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            other => other,
        })
        .collect();
    let trimmed = sanitized.trim().trim_matches('.');
    if trimmed.is_empty() {
        "attachment".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_file_name_strips_directories_and_illegal_characters() {
        assert_eq!(safe_file_name("../../evil.png"), "evil.png");
        assert_eq!(safe_file_name(r"C:\temp\shot.png"), "shot.png");
        assert_eq!(safe_file_name("a/b:c.png"), "b_c.png");
        assert_eq!(safe_file_name("  .. "), "attachment");
        assert_eq!(safe_file_name(""), "attachment");
    }

    #[test]
    fn duplicate_names_get_numbered_destinations() {
        let mut used = HashSet::new();
        let dir = Path::new("out");
        assert_eq!(
            unique_destination(dir, "chart.png", &mut used),
            dir.join("chart.png")
        );
        assert_eq!(
            unique_destination(dir, "chart.png", &mut used),
            dir.join("chart (2).png")
        );
        assert_eq!(
            unique_destination(dir, "chart.png", &mut used),
            dir.join("chart (3).png")
        );
        assert_eq!(
            unique_destination(dir, "notes", &mut used),
            dir.join("notes")
        );
        assert_eq!(
            unique_destination(dir, "notes", &mut used),
            dir.join("notes (2)")
        );
    }

    /// A second `--download-attachments` run into the same directory must number
    /// around what the first one saved instead of truncating it.
    #[test]
    fn files_left_by_an_earlier_run_are_never_overwritten() {
        let dir = std::env::temp_dir().join("kf-unique-destination-test");
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).expect("temp directory is creatable");
        std::fs::write(dir.join("chart.png"), b"first run").expect("temp file is writable");
        std::fs::write(dir.join("chart (2).png"), b"first run").expect("temp file is writable");

        let mut used = HashSet::new();
        assert_eq!(
            unique_destination(&dir, "chart.png", &mut used),
            dir.join("chart (3).png")
        );
        assert_eq!(
            unique_destination(&dir, "chart.png", &mut used),
            dir.join("chart (4).png")
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn dotfiles_keep_their_whole_name() {
        let mut used = HashSet::new();
        let dir = Path::new("out");
        assert_eq!(
            unique_destination(dir, ".gitignore", &mut used),
            dir.join("gitignore")
        );
    }
}
