use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Walk upward to find `.rit` repo root
pub fn find_repo_root() -> io::Result<PathBuf> {
    let mut dir = std::env::current_dir()?;
    loop {
        let candidate = dir.join(".rit");
        if candidate.exists() && candidate.is_dir() {
            return Ok(candidate);
        }
        if !dir.pop() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Not a Rit repository",
            ));
        }
    }
}

/// Write object (blob/tree/commit) to objects directory
pub fn write_object(repo_path: &Path, hash: &str, data: &[u8]) -> io::Result<()> {
    let (dir_name, file_name) = hash.split_at(2);
    let obj_dir = repo_path.join("objects").join(dir_name);
    fs::create_dir_all(&obj_dir)?;
    let obj_path = obj_dir.join(file_name);
    if !obj_path.exists() {
        let mut file = fs::File::create(obj_path)?;
        file.write_all(data)?;
    }
    Ok(())
}

/// Read object by hash
pub fn read_object(repo_path: &Path, hash: &str) -> io::Result<Vec<u8>> {
    let (dir_name, file_name) = hash.split_at(2);
    let obj_path = repo_path.join("objects").join(dir_name).join(file_name);
    fs::read(obj_path)
}

/// Get the current commit hash from HEAD (if any)
pub fn read_head_commit(repo_path: &Path) -> io::Result<Option<String>> {
    let head_path = repo_path.join("HEAD");
    if !head_path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(head_path)?;
    if content.starts_with("ref: ") {
        let branch_path = repo_path.join(&content[5..].trim());
        if branch_path.exists() {
            let hash = fs::read_to_string(branch_path)?;
            return Ok(Some(hash.trim().to_string()));
        }
    } else {
        // detached HEAD
        return Ok(Some(content.trim().to_string()));
    }
    Ok(None)
}

pub fn get_current_branch() -> io::Result<Option<String>> {
    let repo_root = find_repo_root()?;
    let head_path = repo_root.join("HEAD");

    // Read HEAD file as string
    let head_content = fs::read_to_string(head_path)?;

    if head_content.starts_with("ref: ") {
        // Extract branch name from "ref: refs/heads/<branch>"
        let branch = head_content
            .trim_start_matches("ref: refs/heads/")
            .trim()
            .to_string();
        Ok(Some(branch))
    } else {
        // Detached HEAD → contains a commit hash instead
        Ok(None)
    }
}

/// Update HEAD to point at new commit hash
pub fn update_head(repo_path: &Path, commit_hash: &str) -> io::Result<()> {
    let head_path = repo_path.join("HEAD");
    let content = fs::read_to_string(&head_path)?;
    if content.starts_with("ref: ") {
        let branch_path = repo_path.join(&content[5..].trim());
        fs::write(branch_path, commit_hash)?;
    }
    Ok(())
}
