use super::utils::{find_repo_root, get_current_branch, load_ritignore, is_ignored};
use super::commit::read_index;
use sha1::{Digest, Sha1};
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub fn run() -> io::Result<()> {
    let repo_root = find_repo_root()?;
    let work_dir = repo_root.parent().unwrap().to_path_buf();
    
    // 1. Print current branch or detached HEAD
    let has_commits = match get_current_branch()? {
        Some(branch) => {
            let head_ref = repo_root.join("refs").join("heads").join(&branch);
            if head_ref.exists() && !fs::read_to_string(&head_ref)?.trim().is_empty() {
                println!("On branch {}", branch);
                true
            } else {
                println!("On branch {}", branch);
                println!("No commits yet");
                false
            }
        }
        None => {
            let head = fs::read_to_string(repo_root.join("HEAD"))?;
            println!("HEAD detached at {}", head.trim());
            true
        }
    };
    
    // 2. Read index to get tracked files
    let index_entries = read_index(&repo_root)?;
    
    // Build a set of tracked file paths (normalized)
    let mut tracked_files: HashSet<PathBuf> = HashSet::new();
    for entry in &index_entries {
        let normalized = entry.path.to_string_lossy();
        let normalized = normalized.trim_start_matches("./");
        tracked_files.insert(PathBuf::from(normalized));
    }
    
    // 3. If there are no commits yet, all index entries are staged for initial commit
    if !has_commits {
        println!();
        if !index_entries.is_empty() {
            println!("Changes to be committed:");
            println!("  (use \"cargo run rm --cached <file>...\" to unstage)");
            println!();
            for entry in &index_entries {
                let normalized = entry.path.to_string_lossy();
                let normalized_path = normalized.trim_start_matches("./");
                println!("  new file:   {}", normalized_path);
            }
            println!();
        }
        
        // Check for untracked files
        let ignore_patterns = load_ritignore(&repo_root)?;
        let mut untracked_files = Vec::new();
        collect_untracked(&work_dir, &work_dir, &repo_root, &tracked_files, &ignore_patterns, &mut untracked_files)?;
        
        if !untracked_files.is_empty() {
            println!("Untracked files:");
            println!("  (use \"cargo run add <file>...\" to include in what will be committed)");
            println!();
            for file in &untracked_files {
                println!("  {}", file);
            }
            println!();
        }
        
        return Ok(());
    }
    
    // 4. Check for modified files (tracked but changed)
    let mut modified_files = Vec::new();
    for entry in &index_entries {
        let normalized = entry.path.to_string_lossy();
        let normalized_path = normalized.trim_start_matches("./");
        let file_path = work_dir.join(normalized_path);
        
        if file_path.exists() {
            // Compute current hash
            let contents = fs::read(&file_path)?;
            let blob_header = format!("blob {}\0", contents.len());
            let mut blob_data = Vec::new();
            blob_data.extend_from_slice(blob_header.as_bytes());
            blob_data.extend_from_slice(&contents);
            
            let mut hasher = Sha1::new();
            hasher.update(&blob_data);
            let current_hash = hex::encode(hasher.finalize());
            
            if current_hash != entry.sha1 {
                modified_files.push(normalized_path.to_string());
            }
        }
    }
    
    // 5. Find untracked files
    let ignore_patterns = load_ritignore(&repo_root)?;
    let mut untracked_files = Vec::new();
    collect_untracked(&work_dir, &work_dir, &repo_root, &tracked_files, &ignore_patterns, &mut untracked_files)?;
    
    // 6. Display results
    println!();
    
    if !modified_files.is_empty() {
        println!("Changes not staged for commit:");
        println!("  (use \"cargo run add <file>...\" to update what will be committed)");
        println!();
        for file in &modified_files {
            println!("  modified:   {}", file);
        }
        println!();
    }
    
    if !untracked_files.is_empty() {
        println!("Untracked files:");
        println!("  (use \"cargo run add <file>...\" to include in what will be committed)");
        println!();
        for file in &untracked_files {
            println!("  {}", file);
        }
        println!();
    }
    
    if modified_files.is_empty() && untracked_files.is_empty() {
        println!("nothing to commit, working tree clean");
    }
    
    Ok(())
}

/// Recursively collect untracked files
fn collect_untracked(
    path: &Path,
    work_dir: &Path,
    repo_root: &Path,
    tracked_files: &HashSet<PathBuf>,
    ignores: &[String],
    untracked: &mut Vec<String>,
) -> io::Result<()> {
    let mut entries: Vec<PathBuf> = fs::read_dir(path)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .collect();
    
    entries.sort();
    
    for entry in entries {
        let file_name = entry.file_name().and_then(|n| n.to_str()).unwrap_or("");
        
        // Skip .rit or .git folder
        if file_name == ".rit" || file_name == ".git" {
            continue;
        }
        
        // Skip ignored files/dirs
        if is_ignored(&entry, repo_root, ignores) {
            continue;
        }
        
        if entry.is_dir() {
            collect_untracked(&entry, work_dir, repo_root, tracked_files, ignores, untracked)?;
        } else if entry.is_file() {
            let rel_path = entry.strip_prefix(work_dir).unwrap();
            
            // Check if this file is tracked
            if !tracked_files.contains(rel_path) {
                untracked.push(rel_path.display().to_string());
            }
        }
    }
    
    Ok(())
}