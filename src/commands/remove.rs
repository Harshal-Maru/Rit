// In src/commands/remove.rs

use super::utils::find_repo_root;
use std::fs;
use std::io::{self};

pub fn run(path_str: &str, cached: bool) -> io::Result<()> {
    let repo_root = find_repo_root()?;
    let index_path = repo_root.join("index");
    let work_dir = repo_root.parent().unwrap();
    let file_path_to_remove = work_dir.join(path_str);

    if !index_path.exists() {
        // Nothing to remove if there's no index
        return Ok(());
    }

    // 1. Read all entries from the current index file.
    let index_content = fs::read_to_string(&index_path)?;
    let all_entries: Vec<&str> = index_content.lines().collect();
    let mut entry_was_removed = false;

    // 2. Filter out the entry that matches the path we want to remove.
    let new_entries: Vec<String> = all_entries
        .into_iter()
        .filter(|line| {
            let parts: Vec<&str> = line.splitn(3, ' ').collect();
            // The path is the last part of the index entry.
            if let Some(entry_path) = parts.last() && *entry_path == path_str {
                entry_was_removed = true;
                return false; // This is the line to remove, so filter it out.
            }
            true // Keep all other lines.
        })
        .map(|s| s.to_string())
        .collect();

    if !entry_was_removed {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("file not staged: {}", path_str),
        ));
    }

    // 3. Write the filtered entries back to the index file.
    // Ensure the file ends with a newline.
    let new_content = if new_entries.is_empty() {
        String::new()
    } else {
        new_entries.join("\n") + "\n"
    };
    fs::write(&index_path, new_content)?;

    // 4. If not --cached, remove the file from the working directory.
    if !cached {

        if file_path_to_remove.exists() {
            fs::remove_file(file_path_to_remove)?;
            println!("rm '{}'", path_str);
        }
    } else {
        // This is the message git uses for --cached
        println!("rm '{}'", path_str);
    }
    
    Ok(())
}