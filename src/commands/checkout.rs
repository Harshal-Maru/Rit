use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use super::commit::read_index;
use super::utils::{find_repo_root, read_object};
use sha1::{Digest, Sha1};

pub fn run(target: &str) -> io::Result<()> {
    let repo_path = find_repo_root()?;
    let work_dir = repo_path.parent().unwrap();

    // Get the list of tracked files from the index.
    let index_entries = read_index(&repo_path)?;
    let mut modified_files = Vec::new();

    // Check each tracked file for modifications against the working directory.
    for entry in &index_entries {
        let file_path = work_dir.join(&entry.path);
        if !file_path.exists() {
            continue; // Skip files that were deleted locally for this check.
        }

        // Hash the file in the working directory.
        let contents = fs::read(&file_path)?;
        let blob_header = format!("blob {}\0", contents.len());
        let mut blob_data = Vec::new();
        blob_data.extend_from_slice(blob_header.as_bytes());
        blob_data.extend_from_slice(&contents);

        let mut hasher = Sha1::new();
        hasher.update(&blob_data);
        let current_hash = hex::encode(hasher.finalize());

        // If the hash is different from the index, the file has been modified.
        if current_hash != entry.sha1 {
            modified_files.push(entry.path.display().to_string());
        }
    }

    // If any files were modified, abort the checkout to prevent data loss.
    if !modified_files.is_empty() {
        let error_message = format!(
            "error: Your local changes to the following files would be overwritten by checkout:\n  {}\n\nPlease commit your changes or stash them before you switch branches.",
            modified_files.join("\n  ")
        );
        return Err(io::Error::new(io::ErrorKind::Other, error_message));
    }

    let commit_hash: String;

    // 1. Check if the target is a branch name.
    let branch_path = repo_path.join("refs").join("heads").join(target);
    if branch_path.exists() {
        commit_hash = fs::read_to_string(&branch_path)?.trim().to_string();
        fs::write(
            repo_path.join("HEAD"),
            format!("ref: refs/heads/{}", target),
        )?;
        println!("Switched to branch '{}'", target);
    } else {
        // 2. Otherwise, assume it's a commit hash (detached HEAD).
        commit_hash = target.to_string();
        fs::write(repo_path.join("HEAD"), &commit_hash)?;
        println!("Note: HEAD is now at {}", &commit_hash[..7]);
    }

    // 3. Read the commit object to get the root tree hash.
    let commit_data = read_object(&repo_path, &commit_hash)?;
    let content = String::from_utf8_lossy(&commit_data);

    let tree_hash = content
        .lines()
        .find(|line| line.starts_with("tree "))
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Commit missing tree"))?;
    let tree_hash = &tree_hash[5..];

    // 4. Restore the working directory from the tree object.
    restore_tree(&repo_path, tree_hash, &work_dir)?;

    Ok(())
}

/// Recursively restores a tree to the filesystem.
fn restore_tree(repo_path: &Path, tree_hash: &str, target_dir: &Path) -> io::Result<()> {
    let tree_data = read_object(repo_path, tree_hash)?;
    let mut pos = 0;

    while pos < tree_data.len() {
        // Find end of mode+filename (until null byte).
        let mut end = pos;
        while end < tree_data.len() && tree_data[end] != 0 {
            end += 1;
        }

        if end >= tree_data.len() {
            break; // malformed tree entry
        }

        let entry_str = String::from_utf8_lossy(&tree_data[pos..end]);
        let (mode, filename) = entry_str
            .split_once(' ')
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Malformed tree entry"))?;

        // SHA1 is 20 bytes after the null byte.
        let sha_start = end + 1;
        let sha_end = sha_start + 20;
        if sha_end > tree_data.len() {
            break; // malformed SHA
        }
        let sha_bytes = &tree_data[sha_start..sha_end];
        let sha1 = hex::encode(sha_bytes);

        let path = target_dir.join(filename);

        if mode == "40000" {
            // Directory: create it and recurse.
            fs::create_dir_all(&path)?;
            restore_tree(repo_path, &sha1, &path)?;
        } else {
            // Blob: write the file's content.
            let blob_data = read_object(repo_path, &sha1)?;
            let mut file = File::create(&path)?;
            file.write_all(&blob_data)?;
        }

        pos = sha_end;
    }

    Ok(())
}
