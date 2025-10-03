use super::utils::{find_repo_root, is_ignored, load_ritignore, write_object};
use sha1::{Digest, Sha1};
use std::fs::{self};
use std::io::{self};
use std::path::{Path, PathBuf};

pub fn run(path: &str) -> io::Result<()> {
    let repo_path = find_repo_root()?;
    let repo_root = repo_path.parent().unwrap_or(&repo_path);
    let path = PathBuf::from(path);
    
    // Load ignore patterns
    let ignore_patterns = load_ritignore(&repo_path)?;
    
    if path.is_file() {
        add_file(repo_root, &repo_path, &path, &ignore_patterns)?;
    } else if path.is_dir() {
        add_dir(repo_root, &repo_path, &path, &ignore_patterns)?;
    } else {
        eprintln!("Path '{}' does not exist", path.display());
    }
    
    Ok(())
}

/// Recursively add a directory
fn add_dir(
    repo_root: &Path,
    repo_path: &Path,
    dir_path: &Path,
    ignores: &[String],
) -> io::Result<()> {
    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        
        // Skip .rit folder
        if name == ".rit" {
            continue;
        }
        
        // Skip ignored files/dirs
        if is_ignored(&path, repo_path, ignores) {
            continue;
        }
        
        if path.is_file() {
            add_file(repo_root, repo_path, &path, ignores)?;
        } else if path.is_dir() {
            add_dir(repo_root, repo_path, &path, ignores)?;
        }
    }
    
    Ok(())
}

/// Add a single file
fn add_file(
    repo_root: &Path,
    repo_path: &Path,
    file_path: &Path,
    ignores: &[String],
) -> io::Result<()> {
    // Check ignore again just in case
    if is_ignored(file_path, repo_path, ignores) {
        return Ok(());
    }
    
    let contents = fs::read(file_path)?;
    let blob_header = format!("blob {}\0", contents.len());
    let mut blob_data = Vec::new();
    blob_data.extend_from_slice(blob_header.as_bytes());
    blob_data.extend_from_slice(&contents);
    
    let mut hasher = Sha1::new();
    hasher.update(&blob_data);
    let hash = hex::encode(hasher.finalize());
    
    // Get file mode
    let mode = get_file_mode(file_path)?;
    
    // Check if file is already in index with same hash
    let relative_path = file_path.strip_prefix(repo_root).unwrap_or(file_path);
    let path_str = relative_path.to_string_lossy();
    let normalized_path = path_str.trim_start_matches("./");
    
    // Read current index to check if update is needed
    let index_path = repo_path.join("index");
    let mut needs_update = true;
    
    if index_path.exists() {
        let content = fs::read_to_string(&index_path)?;
        for line in content.lines() {
            let parts: Vec<&str> = line.splitn(3, ' ').collect();
            if parts.len() == 3 && parts[2] == normalized_path {
                // File is already in index - check if hash changed
                if parts[1] == hash && parts[0] == mode {
                    needs_update = false;
                    break;
                }
            }
        }
    }
    
    if !needs_update {
        // File unchanged, skip
        return Ok(());
    }
    
    // Write blob object
    write_object(repo_path, &hash, &blob_data)?;
    
    // Update index
    update_index(repo_root, repo_path, &mode, &hash, file_path)?;
    
    println!("added {}", file_path.display());
    
    Ok(())
}

/// Get file mode (permissions)
fn get_file_mode(file_path: &Path) -> io::Result<String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(file_path)?;
        let permissions = metadata.permissions();
        let mode = permissions.mode();
        
        // Check if executable
        if mode & 0o111 != 0 {
            Ok("100755".to_string()) // Executable file
        } else {
            Ok("100644".to_string()) // Regular file
        }
    }
    
    #[cfg(not(unix))]
    {
        // On Windows, default to regular file
        Ok("100644".to_string())
    }
}

/// Append <mode> <sha1> <filename> to index
fn update_index(
    repo_root: &Path,
    repo_path: &Path,
    mode: &str,
    hash: &str,
    file_path: &Path,
) -> io::Result<()> {
    // Relative path from repo root
    let relative_path = file_path.strip_prefix(repo_root).unwrap_or(file_path);
    
    // Normalize path - remove any "./" prefix
    let path_str = relative_path.to_string_lossy();
    let normalized_path = path_str.trim_start_matches("./");
    
    let index_path = repo_path.join("index");
    
    // Read existing index entries
    let mut entries = Vec::new();
    if index_path.exists() {
        let content = fs::read_to_string(&index_path)?;
        for line in content.lines() {
            if line.is_empty() {
                continue;
            }
            
            // Parse existing entry
            let parts: Vec<&str> = line.splitn(3, ' ').collect();
            if parts.len() == 3 {
                let existing_path = parts[2];
                // Skip if this is the file we're updating
                if existing_path != normalized_path {
                    entries.push(line.to_string());
                }
            }
        }
    }
    
    // Add the new/updated entry
    entries.push(format!("{} {} {}", mode, hash, normalized_path));
    
    // Write all entries back
    fs::write(&index_path, entries.join("\n") + "\n")?;
    
    Ok(())
}