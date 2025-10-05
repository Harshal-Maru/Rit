use super::commit::read_index;
use super::utils::{find_repo_root, read_object};
use sha1::{Digest, Sha1};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;

pub fn run(target: &str) -> io::Result<()> {
    let repo_path = find_repo_root()?;
    let work_dir = repo_path.parent().unwrap();

    // --- START: SAFETY CHECK ---
    let index_entries = read_index(&repo_path)?;
    let mut modified_files = Vec::new();

    for entry in &index_entries {
        let file_path = work_dir.join(&entry.path);
        if !file_path.exists() {
            continue;
        }

        let contents = fs::read(&file_path)?;
        let blob_header = format!("blob {}\0", contents.len());
        let mut blob_data = Vec::new();
        blob_data.extend_from_slice(blob_header.as_bytes());
        blob_data.extend_from_slice(&contents);

        let mut hasher = Sha1::new();
        hasher.update(&blob_data);
        let current_hash = hex::encode(hasher.finalize());

        if current_hash != entry.sha1 {
            modified_files.push(entry.path.display().to_string());
        }
    }

    if !modified_files.is_empty() {
        let error_message = format!(
            "error: Your local changes to the following files would be overwritten by checkout:\n  {}\n\nPlease commit your changes or stash them before you switch branches.",
            modified_files.join("\n  ")
        );
        return Err(io::Error::other(error_message));
    }
    // --- END: SAFETY CHECK ---

    let commit_hash: String;

    let branch_path = repo_path.join("refs").join("heads").join(target);
    if branch_path.exists() {
        commit_hash = fs::read_to_string(&branch_path)?.trim().to_string();
        fs::write(
            repo_path.join("HEAD"),
            format!("ref: refs/heads/{}", target),
        )?;
        println!("Switched to branch '{}'", target);
    } else {
        commit_hash = target.to_string();
        fs::write(repo_path.join("HEAD"), &commit_hash)?;
        println!("Note: HEAD is now at {}", &commit_hash[..7]);
    }

    let tree_hash = get_tree_hash(&repo_path, &commit_hash)?;

    // Clear the working directory BEFORE restoring files
    clear_working_directory(work_dir)?;

    // Restore the working directory from the new tree
    restore_tree(&repo_path, &tree_hash, work_dir)?;

    Ok(())
}

fn get_tree_hash(repo_path: &Path, hash: &str) -> io::Result<String> {
    let commit_data = read_object(repo_path, hash)?;
    let content = String::from_utf8_lossy(&commit_data);
    content
        .lines()
        .find(|line| line.starts_with("tree "))
        .map(|line| line[5..].to_string())
        .ok_or_else(|| io::Error::other("Commit missing tree hash"))
}

fn clear_working_directory(work_dir: &Path) -> io::Result<()> {
    for entry in fs::read_dir(work_dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = path.file_name().unwrap_or_default();

        if file_name == ".rit" || file_name == ".git" {
            continue;
        }

        if path.is_dir() {
            fs::remove_dir_all(&path)?;
        } else {
            fs::remove_file(&path)?;
        }
    }
    Ok(())
}

fn restore_tree(repo_path: &Path, tree_hash: &str, target_dir: &Path) -> io::Result<()> {
    let tree_data = read_object(repo_path, tree_hash)?;
    let mut pos = 0;
    while pos < tree_data.len() {
        let mut end = pos;
        while end < tree_data.len() && tree_data[end] != 0 {
            end += 1;
        }
        if end >= tree_data.len() { break; }

        let entry_str = String::from_utf8_lossy(&tree_data[pos..end]);
        let (mode, filename) = entry_str.split_once(' ').ok_or_else(|| io::Error::other("Malformed tree entry"))?;
        
        let sha_start = end + 1;
        let sha_end = sha_start + 20;
        if sha_end > tree_data.len() { break; }

        let sha_bytes = &tree_data[sha_start..sha_end];
        let sha1 = hex::encode(sha_bytes);
        let path = target_dir.join(filename);

        if mode == "40000" {
            fs::create_dir_all(&path)?;
            restore_tree(repo_path, &sha1, &path)?;
        } else {
            let blob_data = read_object(repo_path, &sha1)?;
            let mut file = File::create(&path)?;
            file.write_all(&blob_data)?;
        }
        pos = sha_end;
    }
    Ok(())
}