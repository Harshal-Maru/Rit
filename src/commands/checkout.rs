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

/// Restore tree object to given directory
fn restore_tree(repo_path: &Path, tree_hash: &str, target_dir: &Path) -> io::Result<()> {
    let tree_data = read_object(repo_path, tree_hash)?;

    // Skip "tree <size>\0"
    let null_pos = tree_data.iter().position(|&b| b == 0).unwrap();
    let mut pos = null_pos + 1;

    while pos < tree_data.len() {
        // Parse mode + filename until \0
        let mut end = pos;
        while tree_data[end] != 0 {
            end += 1;
        }
        let entry = String::from_utf8_lossy(&tree_data[pos..end]);
        let (mode, filename) = entry.split_once(' ').unwrap();

        // SHA1 = 20 bytes after \0
        let sha_start = end + 1;
        let sha_end = sha_start + 20;
        let sha_bytes = &tree_data[sha_start..sha_end];
        let sha1 = hex::encode(sha_bytes);

        let path = target_dir.join(filename);

        if mode == "40000" {
            // Tree/subdirectory
            fs::create_dir_all(&path)?;
            restore_tree(repo_path, &sha1, &path)?;
        } else {
            // Blob/file
            let blob_data = read_object(repo_path, &sha1)?;
            let null_pos = blob_data.iter().position(|&b| b == 0).unwrap();
            let file_content = &blob_data[(null_pos + 1)..];
            let mut file = File::create(&path)?;
            file.write_all(file_content)?;
        }

        pos = sha_end;
    }

    Ok(())
}
