use super::utils::{find_repo_root, is_ignored, load_ritignore, write_object};
use sha1::{Digest, Sha1};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
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

        // Skip ignored files/dirs - FIXED: pass repo_path instead of repo_root
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
    // Check ignore again just in case - FIXED: pass repo_path instead of repo_root
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

    write_object(repo_path, &hash, &blob_data)?;
    update_index(repo_root, repo_path, &hash, file_path)?;

    println!("added {}", file_path.display());
    Ok(())
}

/// Append <sha1> <filename> to index
fn update_index(
    repo_root: &Path,
    repo_path: &Path,
    hash: &str,
    file_path: &Path,
) -> io::Result<()> {
    // Relative path from repo root
    let relative_path = file_path.strip_prefix(repo_root).unwrap_or(file_path);
    let index_path = repo_path.join("index");

    let mut index = OpenOptions::new()
        .create(true)
        .append(true)
        .open(index_path)?;

    writeln!(index, "{} {}", hash, relative_path.display())?;
    Ok(())
}
