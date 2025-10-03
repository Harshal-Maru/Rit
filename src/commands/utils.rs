use flate2::{Compression, read::ZlibDecoder, write::ZlibEncoder};
use std::fs;
use std::io::{self, BufRead, Read, Write};
use std::path::{Path, PathBuf};
use glob::Pattern;

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

/// Write object (blob/tree/commit) to objects directory (compressed)
pub fn write_object(repo_path: &Path, hash: &str, data: &[u8]) -> io::Result<()> {
    let (dir_name, file_name) = hash.split_at(2);
    let obj_dir = repo_path.join("objects").join(dir_name);
    fs::create_dir_all(&obj_dir)?;
    let obj_path = obj_dir.join(file_name);

    if !obj_path.exists() {
        write_compressed(data, &obj_path)?;
    }
    Ok(())
}

/// Read object by hash (decompressed and strip header)
pub fn read_object(repo_path: &Path, hash: &str) -> io::Result<Vec<u8>> {
    let (dir_name, file_name) = hash.split_at(2);
    let obj_path = repo_path.join("objects").join(dir_name).join(file_name);
    let data = read_compressed(&obj_path)?;

    // Strip header: "<type> <size>\0"
    if let Some(pos) = data.iter().position(|&b| b == 0) {
        Ok(data[pos + 1..].to_vec()) // return only content
    } else {
        Ok(data) // fallback: no header found
    }
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

    let head_content = fs::read_to_string(head_path)?;

    if head_content.starts_with("ref: ") {
        let branch = head_content
            .trim_start_matches("ref: refs/heads/")
            .trim()
            .to_string();
        Ok(Some(branch))
    } else {
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

/// Compress and write object data to disk
pub fn write_compressed(data: &[u8], object_path: &Path) -> io::Result<()> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    let compressed = encoder.finish()?;
    fs::write(object_path, compressed)?;
    Ok(())
}

/// Read and decompress an object from disk
pub fn read_compressed(object_path: &Path) -> io::Result<Vec<u8>> {
    let compressed = fs::read(object_path)?;
    let mut decoder = ZlibDecoder::new(&compressed[..]);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;
    Ok(decompressed)
}

/// Load ignore patterns from `.ritignore`
pub fn load_ritignore(repo_path: &Path) -> io::Result<Vec<String>> {
    let ignore_file = repo_path.parent().unwrap_or(repo_path).join(".ritignore");
    if !ignore_file.exists() {
        return Ok(Vec::new());
    }

    let file = fs::File::open(ignore_file)?;
    let reader = io::BufReader::new(file);

    Ok(reader
        .lines()
        .filter_map(Result::ok)
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect())
}

/// Check if path should be ignored
pub fn is_ignored(path: &Path, repo_path: &Path, ignores: &[String]) -> bool {
    let work_dir = repo_path.parent().unwrap_or(repo_path);
    let rel_path = match path.strip_prefix(work_dir) {
        Ok(p) => p,
        Err(_) => path,
    };

    // always ignore .rit
    if rel_path.starts_with(".rit") {
        return true;
    }

    // Normalize the relative path (remove ./ prefix and convert to string)
    let rel_path_str = rel_path.to_string_lossy();
    let normalized_path = rel_path_str.trim_start_matches("./");
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    for pat in ignores {
        let pat = pat.trim();
        if pat.is_empty() || pat.starts_with('#') {
            continue;
        }

        if pat.ends_with('/') {
            // directory pattern - check if path is inside this directory
            let dir_name = &pat[..pat.len()-1];
            
            // Check if the path starts with the directory name
            if normalized_path.starts_with(dir_name) {
                // Make sure it's actually inside the directory
                // (e.g., "target/foo" matches "target/", but "targetfoo" doesn't)
                let after_dir = &normalized_path[dir_name.len()..];
                if after_dir.is_empty() || after_dir.starts_with('/') || after_dir.starts_with('\\') {
                    return true;
                }
            }
        } else if pat.contains('*') {
            // match glob against basename
            if let Ok(p) = Pattern::new(pat) {
                if p.matches(file_name) {
                    return true;
                }
                // Also try matching against the full relative path
                if p.matches(normalized_path) {
                    return true;
                }
            }
        } else {
            // exact match relative path
            if normalized_path == pat || rel_path_str == pat {
                return true;
            }
        }
    }

    false
}