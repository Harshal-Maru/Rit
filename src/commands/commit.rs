use sha1::{Digest, Sha1};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn run(message: &str) -> io::Result<()> {
    // 1. Locate repository root
    let repo_path = find_repo_root()?;

    // 2. Read the index
    let index_entries = read_index(&repo_path)?;

    if index_entries.is_empty() {
        println!("Nothing to commit");
        return Ok(());
    }

    // 3. Build tree object from index
    let tree_hash = write_tree(&repo_path, &index_entries)?;

    // 4. Get parent commit (if HEAD exists)
    let parent_hash = read_head(&repo_path)?;

    // 5. Build commit object content
    let commit_content = build_commit_content(tree_hash.as_str(), parent_hash.as_deref(), message);

    // 6. Hash commit object
    let mut hasher = Sha1::new();
    hasher.update(&commit_content);
    let commit_hash = hex::encode(hasher.finalize());

    // 7. Write commit object to objects/
    write_object(&repo_path, &commit_hash, commit_content.as_bytes())?;

    // 8. Update HEAD (branch reference)
    update_head(&repo_path, &commit_hash)?;

    println!("[main {}] {}", &commit_hash[..7], message);

    Ok(())
}

/// Walk upward to find .rit
fn find_repo_root() -> io::Result<std::path::PathBuf> {
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

/// Read index and parse into Vec<(hash, path)>
fn read_index(repo_path: &Path) -> io::Result<Vec<(String, String)>> {
    let index_path = repo_path.join("index");
    if !index_path.exists() {
        return Ok(Vec::new());
    }
    let data = fs::read_to_string(index_path)?;
    let entries = data
        .lines()
        .filter_map(|line| {
            let mut parts = line.splitn(2, ' ');
            Some((parts.next()?.to_string(), parts.next()?.to_string()))
        })
        .collect();
    Ok(entries)
}

/// Build tree object (flat for Phase 1)
fn write_tree(repo_path: &Path, index_entries: &[(String, String)]) -> io::Result<String> {
    // Format: "100644 filename\0<sha1_bytes>"
    let mut tree_data = Vec::new();
    for (sha1, path) in index_entries {
        let mode_path = format!("100644 {}\0", path);
        tree_data.extend_from_slice(mode_path.as_bytes());
        tree_data.extend_from_slice(&hex::decode(sha1).unwrap());
    }

    // Hash tree
    let header = format!("tree {}\0", tree_data.len());
    let mut obj_data = Vec::new();
    obj_data.extend_from_slice(header.as_bytes());
    obj_data.extend_from_slice(&tree_data);

    // Compute SHA1
    let mut hasher = Sha1::new();
    hasher.update(&obj_data);
    let tree_hash = hex::encode(hasher.finalize());

    // Write tree object
    write_object(repo_path, &tree_hash, &obj_data)?;

    Ok(tree_hash)
}

/// Read HEAD and return parent commit hash (if any)
fn read_head(repo_path: &Path) -> io::Result<Option<String>> {
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
    }
    Ok(None)
}

/// Build commit content string
fn build_commit_content(tree_hash: &str, parent_hash: Option<&str>, message: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let mut content = format!("tree {}\n", tree_hash);
    if let Some(parent) = parent_hash {
        content += &format!("parent {}\n", parent);
    }
    content += &format!("author Harshal <harshal@example.com> {} +0530\n", timestamp);
    content += &format!(
        "committer Harshal <harshal@example.com> {} +0530\n\n",
        timestamp
    );
    content += message;
    content
}

/// Write object (blob/tree/commit) to .rit/objects/
fn write_object(repo_path: &Path, hash: &str, data: &[u8]) -> io::Result<()> {
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

/// Update HEAD reference to new commit
fn update_head(repo_path: &Path, commit_hash: &str) -> io::Result<()> {
    let head_path = repo_path.join("HEAD");
    let content = fs::read_to_string(&head_path)?;
    if content.starts_with("ref: ") {
        let branch_path = repo_path.join(&content[5..].trim());
        fs::write(branch_path, commit_hash)?;
    } else {
        // detached HEAD (not handled in Phase 1)
    }
    Ok(())
}
