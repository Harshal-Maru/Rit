use super::utils::{find_repo_root, get_current_branch};
use std::fs;
use std::io;
use std::io::BufRead;
use std::path::{Path, PathBuf};

pub fn run() -> io::Result<()> {
    let repo_root = find_repo_root()?;
    let work_dir = repo_root.parent().unwrap().to_path_buf();

    // 1. Print current branch or detached HEAD
    match get_current_branch()? {
        Some(branch) => {
            let head_ref = repo_root.join("refs").join("heads").join(&branch);
            if head_ref.exists() && fs::read_to_string(&head_ref)?.trim().is_empty() {
                println!("No commits yet on branch {}", branch);
            } else {
                println!("On branch {}", branch);
            }
        }
        None => {
            let head = fs::read_to_string(repo_root.join("HEAD"))?;
            println!("HEAD detached at {}", head.trim());
        }
    }

    println!();

    // 2. Show untracked files (everything in working dir except .rit)
    println!("Untracked files:");
    list_untracked(&work_dir)?;

    Ok(())
}

/// Recursively lists untracked files relative to `repo_root`
fn list_untracked(repo_root: &Path) -> io::Result<()> {
    let ignore_patterns = load_ritignore(repo_root)?;

    fn walk(path: &Path, repo_root: &Path, ignores: &[String]) -> io::Result<()> {
        let mut entries: Vec<PathBuf> = fs::read_dir(path)?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .collect();

        // Sort entries so output is consistent
        entries.sort();

        for entry in entries {
            let file_name = entry.file_name().and_then(|n| n.to_str()).unwrap_or("");

            // skip .rit or .git folder entirely
            if file_name == ".rit" || file_name == ".git" {
                continue;
            }

            // skip ignored files/dirs
            if is_ignored(&entry, repo_root, ignores) {
                continue;
            }

            if entry.is_dir() {
                walk(&entry, repo_root, ignores)?;
            } else if entry.is_file() {
                println!("  {}", entry.strip_prefix(repo_root).unwrap().display());
            }
        }

        Ok(())
    }

    walk(repo_root, repo_root, &ignore_patterns)
}

/// Load ignore patterns from `.ritignore`
fn load_ritignore(repo_root: &Path) -> io::Result<Vec<String>> {
    let ignore_file = repo_root.join(".ritignore");
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

/// Returns true if path matches any ignore pattern
fn is_ignored(path: &Path, repo_root: &Path, ignores: &[String]) -> bool {
    let rel = path.strip_prefix(repo_root).unwrap().to_string_lossy();

    for pat in ignores {
        if pat.ends_with('/') {
            // directory match
            if rel.starts_with(&pat[..pat.len() - 1]) {
                return true;
            }
        } else if pat.starts_with("*.") {
            // suffix match like *.lock
            if rel.ends_with(&pat[1..]) {
                return true;
            }
        } else if rel == *pat {
            // exact match
            return true;
        }
    }
    false
}
