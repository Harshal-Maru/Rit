// Create a new file: src/commands/diff.rs
use super::commit::read_index;
use super::utils::{find_repo_root, read_object};
use std::fs;
use std::io;
use colored::Colorize;

pub fn run(path: Option<&str>) -> io::Result<()> {
    let repo_root = find_repo_root()?;
    let work_dir = repo_root.parent().unwrap();
    let index_entries = read_index(&repo_root)?;

    // 1. Create a HashMap for easy lookup of staged files (path -> sha1)
    let mut staged_files = std::collections::HashMap::new();
    for entry in index_entries {
        staged_files.insert(entry.path, entry.sha1);
    }

    // 2. Determine which files to diff (all tracked files or just one)
    let files_to_diff: Vec<_> = if let Some(file_path) = path {
        vec![std::path::PathBuf::from(file_path)]
    } else {
        staged_files.keys().cloned().collect()
    };

    // 3. For each file, generate and print the diff
    for file_path in files_to_diff {
        if let Some(sha1) = staged_files.get(&file_path) {
            // Read the staged version (blob object)
            let staged_content_bytes = read_object(&repo_root, sha1)?;
            let staged_content = String::from_utf8_lossy(&staged_content_bytes);

            // Read the working directory version
            let working_path = work_dir.join(&file_path);
            let working_content = fs::read_to_string(working_path).unwrap_or_default();

            // Compare and print the diff if there are changes
            if staged_content != working_content {
                println!(
                    "diff --rit a/{} b/{}",
                    file_path.display(),
                    file_path.display()
                );

                let diff = dissimilar::diff(&staged_content, &working_content);

                for chunk in diff {
                    match chunk {
                        dissimilar::Chunk::Equal(text) => {
                            // Print lines that are the same, prefixed with a space
                            for line in text.lines() {
                                println!(" {}", line);
                            }
                        }
                        dissimilar::Chunk::Delete(text) => {
                            // Print deleted lines in red, prefixed with a '-'
                            for line in text.lines() {
                                println!("{}", format!("-{}", line).red());
                            }
                        }
                        dissimilar::Chunk::Insert(text) => {
                            // Print added lines in green, prefixed with a '+'
                            for line in text.lines() {
                                println!("{}", format!("+{}", line).green());
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
