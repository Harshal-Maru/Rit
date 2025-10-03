use sha1::{Digest, Sha1};
use std::fs;
use std::io::{self};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::BTreeMap;

use super::utils::{find_repo_root, update_head, write_object};

pub(crate) struct IndexEntry {
    pub mode: String,
    pub sha1: String,
    pub path: PathBuf,
}

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
        
        let (mode, sha1, path) = if parts.len() == 2 {
            // Format: <sha1> <path>
            // Default mode to 100644 (regular file)
            ("100644".to_string(), parts[0].to_string(), PathBuf::from(parts[1]))
        } else if parts.len() == 3 {
            // Format: <mode> <sha1> <path>
            (parts[0].to_string(), parts[1].to_string(), PathBuf::from(parts[2]))
        } else {
            eprintln!("Warning: Skipping malformed line: '{}'", line);
            continue;
        };
        
        entries.push(IndexEntry { mode, sha1, path });
    }

    Ok(entries)
}

/// Build tree object recursively
pub fn write_tree(repo_path: &Path, index_entries: &[IndexEntry]) -> io::Result<String> {
    let entry_refs: Vec<&IndexEntry> = index_entries.iter().collect();
    build_tree_recursive(repo_path, &entry_refs, Path::new(""))
}

fn build_tree_recursive(
    repo_path: &Path,
    entries: &[&IndexEntry],
    current_dir: &Path,
) -> io::Result<String> {
    // Group entries by immediate child (file or directory)
    let mut files: Vec<&IndexEntry> = Vec::new();
    let mut subdirs: BTreeMap<String, Vec<&IndexEntry>> = BTreeMap::new();

    for entry in entries.iter().copied() {
        // Get relative path from current directory
        let rel_path = if current_dir.as_os_str().is_empty() {
            &entry.path
        } else {
            match entry.path.strip_prefix(current_dir) {
                Ok(p) => p,
                Err(_) => continue, // Not in this directory
            }
        };

        // Check if this entry is directly in current_dir or in a subdirectory
        let components: Vec<_> = rel_path.components().collect();
        if components.len() == 1 {
            // Direct file in current directory
            files.push(entry);
        } else if let Some(first) = components.first() {
            // File in a subdirectory
            let subdir_name = first.as_os_str().to_string_lossy().to_string();
            subdirs.entry(subdir_name).or_default().push(entry);
        }
    }

    // Build tree content
    let mut tree_entries: Vec<(String, String, Vec<u8>)> = Vec::new();

    // Add file entries
    for entry in files {
        let filename = entry.path.file_name()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid filename"))?
            .to_string_lossy()
            .to_string();
        
        let sha_bytes = hex::decode(&entry.sha1)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        
        tree_entries.push((entry.mode.clone(), filename, sha_bytes));
    }

    // Add subdirectory entries (recursively)
    for (subdir_name, subdir_entries) in subdirs {
        let subdir_path = current_dir.join(&subdir_name);
        let subtree_hash = build_tree_recursive(repo_path, &subdir_entries, &subdir_path)?;
        
        let sha_bytes = hex::decode(&subtree_hash)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        
        tree_entries.push(("40000".to_string(), subdir_name, sha_bytes));
    }

    // Sort entries (Git requires this)
    tree_entries.sort_by(|a, b| a.1.cmp(&b.1));

    // Build tree object content
    let mut tree_data = Vec::new();
    for (mode, name, sha_bytes) in tree_entries {
        tree_data.extend_from_slice(mode.as_bytes());
        tree_data.push(b' ');
        tree_data.extend_from_slice(name.as_bytes());
        tree_data.push(b'\0');
        tree_data.extend_from_slice(&sha_bytes);
    }

    // Add tree header
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
    let content = content.trim();
    
    if content.starts_with("ref: ") {
        let ref_path = &content[5..];
        let branch_path = repo_path.join(ref_path);
        
        if branch_path.exists() {
            let hash = fs::read_to_string(branch_path)?;
            return Ok(Some(hash.trim().to_string()));
        }
    } else if content.len() == 40 {
        // Direct SHA1 (detached HEAD)
        return Ok(Some(content.to_string()));
    }
    
    Ok(None)
}

/// Build commit content string
fn build_commit_content(tree_hash: &str, parent_hash: Option<&str>, message: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    let mut content = String::new();
    content.push_str(&format!("tree {}\n", tree_hash));
    
    if let Some(parent) = parent_hash {
        content.push_str(&format!("parent {}\n", parent));
    }
    
    content.push_str(&format!(
        "author Harshal <harshal@example.com> {} +0530\n",
        timestamp
    ));
    content.push_str(&format!(
        "committer Harshal <harshal@example.com> {} +0530\n",
        timestamp
    ));
    content.push_str("\n");
    content.push_str(message);
    
    content
}

