use super::utils::{find_repo_root, update_head, write_object};
use sha1::{Digest, Sha1};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::{self};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

// Helper function to read the .rit/config file
fn read_config(repo_path: &Path) -> io::Result<HashMap<String, String>> {
    let config_path = repo_path.join("config");
    let mut config = HashMap::new();
    if !config_path.exists() {
        return Ok(config);
    }

    let content = fs::read_to_string(config_path)?;
    for line in content.lines() {
        // Simple key = value parser, ignores sections like [user]
        if let Some((key, value)) = line.trim().split_once('=') {
            config.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    Ok(config)
}

pub(crate) struct IndexEntry {
    pub mode: String,
    pub sha1: String,
    pub path: PathBuf,
}

pub fn run(message: &str) -> io::Result<()> {
    // 1. Locate repository root and read config
    let repo_path = find_repo_root()?;
    let config = read_config(&repo_path)?;

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

    // 5. Build commit object content, now passing the config
    let commit_content = build_commit_content(&tree_hash, parent_hash.as_deref(), message, &config);

    // 6. Hash commit object to get its ID
    let mut hasher = Sha1::new();
    hasher.update(&commit_content);
    let commit_hash = hex::encode(hasher.finalize());

    // 7. Write commit object to the object database
    write_object(&repo_path, &commit_hash, commit_content.as_bytes())?;

    // 8. Update HEAD (the current branch) to point at the new commit
    update_head(&repo_path, &commit_hash)?;

    let current_branch = super::utils::get_current_branch()?.unwrap_or_else(|| "main".to_string());
    println!("[{} {}] {}", current_branch, &commit_hash[..7], message);
    Ok(())
}

/// Read index and parse into Vec<IndexEntry>
pub fn read_index(repo_path: &Path) -> io::Result<Vec<IndexEntry>> {
    let index_path = repo_path.join("index");
    if !index_path.exists() {
        return Ok(Vec::new());
    }
    let data = fs::read_to_string(index_path)?;
    let mut entries = Vec::new();
    for line in data.lines() {
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.splitn(3, ' ').collect();
        if parts.len() == 3 {
            entries.push(IndexEntry {
                mode: parts[0].to_string(),
                sha1: parts[1].to_string(),
                path: PathBuf::from(parts[2]),
            });
        }
    }
    Ok(entries)
}

/// Build tree object recursively from a flat list of index entries
pub fn write_tree(repo_path: &Path, index_entries: &[IndexEntry]) -> io::Result<String> {
    let entry_refs: Vec<&IndexEntry> = index_entries.iter().collect();
    build_tree_recursive(repo_path, &entry_refs, Path::new(""))
}

fn build_tree_recursive(
    repo_path: &Path,
    entries: &[&IndexEntry],
    current_dir: &Path,
) -> io::Result<String> {
    let mut files: Vec<&IndexEntry> = Vec::new();
    let mut subdirs: BTreeMap<String, Vec<&IndexEntry>> = BTreeMap::new();

    for &entry in entries {
        let rel_path = entry.path.strip_prefix(current_dir).unwrap();
        let mut components = rel_path.components();
        let first_component = components.next().unwrap().as_os_str().to_string_lossy();

        if components.next().is_none() {
            files.push(entry);
        } else {
            subdirs
                .entry(first_component.to_string())
                .or_default()
                .push(entry);
        }
    }

    let mut tree_entries: Vec<(String, String, Vec<u8>)> = Vec::new();
    for entry in files {
        let filename = entry
            .path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let sha_bytes = hex::decode(&entry.sha1).map_err(io::Error::other)?;
        tree_entries.push((entry.mode.clone(), filename, sha_bytes));
    }

    for (subdir_name, subdir_entries) in subdirs {
        let subdir_path = current_dir.join(&subdir_name);
        let subtree_hash = build_tree_recursive(repo_path, &subdir_entries, &subdir_path)?;
        let sha_bytes = hex::decode(&subtree_hash).map_err(io::Error::other)?;
        tree_entries.push(("40000".to_string(), subdir_name, sha_bytes));
    }

    tree_entries.sort_by(|a, b| a.1.cmp(&b.1));

    let mut tree_data = Vec::new();
    for (mode, name, sha_bytes) in tree_entries {
        tree_data.extend_from_slice(mode.as_bytes());
        tree_data.push(b' ');
        tree_data.extend_from_slice(name.as_bytes());
        tree_data.push(b'\0');
        tree_data.extend_from_slice(&sha_bytes);
    }

    let header = format!("tree {}\0", tree_data.len());
    let mut obj_data = Vec::new();
    obj_data.extend_from_slice(header.as_bytes());
    obj_data.extend_from_slice(&tree_data);

    let mut hasher = Sha1::new();
    hasher.update(&obj_data);
    let tree_hash = hex::encode(hasher.finalize());
    write_object(repo_path, &tree_hash, &obj_data)?;
    Ok(tree_hash)
}

/// Read HEAD and return parent commit hash (if any)
fn read_head(repo_path: &Path) -> io::Result<Option<String>> {
    let head_path = repo_path.join("HEAD");
    if !head_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(head_path)?.trim().to_string();

    if let Some(ref_path) = content.strip_prefix("ref: ") {
        let branch_path = repo_path.join(ref_path);
        if branch_path.exists() {
            let hash = fs::read_to_string(branch_path)?.trim().to_string();
            if hash.is_empty() {
                Ok(None)
            } else {
                Ok(Some(hash))
            }
        } else {
            Ok(None) // Branch file doesn't exist yet
        }
    } else if content.len() == 40 {
        Ok(Some(content)) // Detached HEAD
    } else {
        Ok(None)
    }
}

/// Build commit content string using author info from config
pub fn build_commit_content(
    tree_hash: &str,
    parent_hash: Option<&str>,
    message: &str,
    config: &HashMap<String, String>,
) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let author_name = config
        .get("name")
        .cloned()
        .unwrap_or_else(|| "User".to_string());
    let author_email = config
        .get("email")
        .cloned()
        .unwrap_or_else(|| "user@example.com".to_string());

    let author_line = format!(
        "author {} <{}> {} +0530",
        author_name, author_email, timestamp
    );
    let committer_line = format!(
        "committer {} <{}> {} +0530",
        author_name, author_email, timestamp
    );

    let mut content = String::new();
    content.push_str(&format!("tree {}\n", tree_hash));

    if let Some(parent) = parent_hash {
        // This logic handles both single parents (just a hash) and
        // merge commits (which pass a pre-formatted string with multiple "parent .." lines)
        if parent.contains("parent ") {
            content.push_str(parent);
        } else {
            content.push_str(&format!("parent {}\n", parent));
        }
    }

    content.push_str(&author_line);
    content.push('\n');
    content.push_str(&committer_line);
    content.push('\n');
    content.push('\n');
    content.push_str(message);

    content
}
