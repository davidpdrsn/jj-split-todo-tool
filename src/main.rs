use similar::{ChangeTag, TextDiff};
use std::{collections::HashSet, fs, io, path::Path};
use walkdir::WalkDir;

fn main() -> io::Result<()> {
    let args = std::env::args().collect::<Vec<_>>();

    let left_dir = Path::new(&args[1]);
    let right_dir = Path::new(&args[2]);

    let mut processed_files = HashSet::new();

    // Process all files in right directory
    for entry in WalkDir::new(right_dir).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            let right_path = entry.path();
            let relative_path = right_path.strip_prefix(right_dir).unwrap();
            let left_path = left_dir.join(relative_path);

            processed_files.insert(relative_path.to_path_buf());

            let left_content = fs::read_to_string(&left_path).unwrap_or_default();
            let right_content = fs::read_to_string(right_path)?;

            let output = process_diff(
                relative_path.to_str().unwrap_or("?"),
                &left_content,
                &right_content,
            );

            fs::write(right_path, output)?;
        }
    }

    // Process files that exist in left but not in right (deleted files)
    for entry in WalkDir::new(left_dir).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            let left_path = entry.path();
            let relative_path = left_path.strip_prefix(left_dir).unwrap();

            if !processed_files.contains(relative_path) {
                // File was deleted - restore lines that contain TODO (TODO deletions go to parent)
                let left_content = fs::read_to_string(left_path)?;
                let output = process_diff(relative_path.to_str().unwrap_or("?"), &left_content, "");

                if !output.is_empty() {
                    let right_path = right_dir.join(relative_path);
                    if let Some(parent) = right_path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(right_path, output)?;
                }
            }
        }
    }

    Ok(())
}

fn process_diff(filename: &str, left: &str, right: &str) -> String {
    let diff = TextDiff::from_lines(left, right);

    let changes: Vec<_> = diff.iter_all_changes().collect();

    let mut output = String::new();
    let mut i = 0;

    while i < changes.len() {
        let change = &changes[i];
        let line_preview: String = change.value().chars().take(50).collect();
        let line_preview = line_preview.trim_end();

        match change.tag() {
            ChangeTag::Equal => {
                // Unchanged line - keep it
                output.push_str(change.value());
                i += 1;
            }
            ChangeTag::Insert => {
                if change.value().contains("TODO") {
                    // Skip the TODO line (it goes to parent commit)
                    i += 1;

                    // Also skip any trailing blank lines that are part of this TODO block
                    while i < changes.len() {
                        let next = &changes[i];
                        if next.tag() == ChangeTag::Insert && next.value().trim().is_empty() {
                            i += 1;
                        } else {
                            break;
                        }
                    }
                } else {
                    // Non-TODO insert - keep it (goes to HEAD)
                    output.push_str(change.value());
                    i += 1;
                }
            }
            ChangeTag::Delete => {
                // Line removed from left - restore if it contains TODO (TODO deletion goes to parent)
                if change.value().contains("TODO") {
                    output.push_str(change.value());
                } else {
                }
                i += 1;
            }
        }
    }

    output
}
