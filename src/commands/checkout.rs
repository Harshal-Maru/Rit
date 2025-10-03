use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use super::utils::{find_repo_root, read_object};

pub fn run(target: &str) -> io::Result<()> {
    let repo_path = find_repo_root()?;

    let commit_hash: String;

    // 1. Check if target is a branch
    let branch_path = repo_path.join("refs").join("heads").join(target);
    if branch_path.exists() {
        commit_hash = fs::read_to_string(&branch_path)?.trim().to_string();
        fs::write(
            repo_path.join("HEAD"),
            format!("ref: refs/heads/{}", target),
        )?;
        println!("Switched to branch '{}'", target);
    } else {
        // 2. Otherwise assume it's a commit hash (detached HEAD)
        commit_hash = target.to_string();
        fs::write(repo_path.join("HEAD"), &commit_hash)?;
        println!("Note: HEAD is now at {}", &commit_hash[..7]);
    }

    // 3. Read commit object to get tree hash
    let commit_data = read_object(&repo_path, &commit_hash)?;
    let content = String::from_utf8_lossy(&commit_data);

    let tree_hash = content
        .lines()
        .find(|line| line.starts_with("tree "))
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Commit missing tree"))?;
    let tree_hash = &tree_hash[5..];

    // 4. Restore tree
    restore_tree(&repo_path, tree_hash, &PathBuf::from("."))?;

    Ok(())
}

/// Recursively restores a tree to the filesystem
fn restore_tree(repo_path: &Path, tree_hash: &str, target_dir: &Path) -> io::Result<()> {
    let tree_data = read_object(repo_path, tree_hash)?;
    let mut pos = 0;

    while pos < tree_data.len() {
        // Find end of mode+filename (until null byte)
        let mut end = pos;
        while end < tree_data.len() && tree_data[end] != 0 {
            end += 1;
        }

        if end >= tree_data.len() {
            break; // malformed tree entry
        }

        let entry = String::from_utf8_lossy(&tree_data[pos..end]);
        let (mode, filename) = entry.split_once(' ').unwrap();

        // SHA1 = 20 bytes after null byte
        let sha_start = end + 1;
        let sha_end = sha_start + 20;
        if sha_end > tree_data.len() {
            break; // malformed SHA
        }
        let sha_bytes = &tree_data[sha_start..sha_end];
        let sha1 = hex::encode(sha_bytes);

        let path = target_dir.join(filename);

        if mode == "40000" {
            // Directory → recurse
            fs::create_dir_all(&path)?;
            restore_tree(repo_path, &sha1, &path)?;
        } else {
            // Blob → write file directly
            let blob_data = read_object(repo_path, &sha1)?;
            let mut file = File::create(&path)?;
            file.write_all(&blob_data)?; // already stripped header
        }

        pos = sha_end;
    }

    Ok(())
}
