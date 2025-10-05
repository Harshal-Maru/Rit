use super::commit::{build_commit_content, write_tree, IndexEntry};
use super::utils::{find_repo_root, get_current_branch, read_head_commit, read_object, update_head, write_object};
use sha1::{Digest, Sha1};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::fs;
use std::io::{self};
use std::path::{Path, PathBuf};

pub fn run(branch_name: &str) -> io::Result<()> {
    let repo_root = find_repo_root()?;

    // --- 1. SETUP: Get commit hashes for both branches ---
    let current_branch = get_current_branch()?.ok_or_else(|| {
        io::Error::other("HEAD is detached, cannot merge")
    })?;

    let receiver_hash = read_head_commit(&repo_root)?.ok_or_else(|| {
        io::Error::other("Current branch has no commits")
    })?;

    let giver_branch_path = repo_root.join("refs").join("heads").join(branch_name);
    if !giver_branch_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Branch '{}' not found", branch_name),
        ));
    }
    let giver_hash = fs::read_to_string(giver_branch_path)?.trim().to_string();

    // --- 2. FIND ANCESTOR: Find the most recent common ancestor of the two commits ---
    let ancestor_hash = find_common_ancestor(&repo_root, &receiver_hash, &giver_hash)?.ok_or_else(|| {
        io::Error::other("No common ancestor found")
    })?;

    // --- 3. HANDLE MERGE SCENARIOS ---
    if ancestor_hash == giver_hash {
        println!("Already up to date.");
        return Ok(());
    }

    if ancestor_hash == receiver_hash {
        // This is a fast-forward merge.
        let branch_ref_path = repo_root.join("refs").join("heads").join(&current_branch);
        fs::write(branch_ref_path, &giver_hash)?;
        println!("Fast-forward merge. Updated branch '{}' to '{}'.", current_branch, &giver_hash[..7]);
        // TODO: Update working directory



        return Ok(());
    }
    
    // --- 4. THREE-WAY MERGE ---
    println!("Performing a three-way merge.");
    let receiver_tree = get_tree_hash(&repo_root, &receiver_hash)?;
    let giver_tree = get_tree_hash(&repo_root, &giver_hash)?;
    let ancestor_tree = get_tree_hash(&repo_root, &ancestor_hash)?;

    let merged_index_entries = merge_trees(&repo_root, &ancestor_tree, &receiver_tree, &giver_tree)?;

    // --- 5. CREATE MERGE COMMIT ---
    // The new tree from the merged content
    let merged_tree_hash = write_tree(&repo_root, &merged_index_entries)?;

    let commit_message = format!("Merge branch '{}' into {}", branch_name, current_branch);

    // Build a commit with TWO parents
    let mut parents = String::new();
    parents.push_str(&format!("parent {}\n", receiver_hash));
    parents.push_str(&format!("parent {}\n", giver_hash));

    let extra_headers = HashMap::new();
    let commit_content = build_commit_content(&merged_tree_hash, Some(&parents), &commit_message, &extra_headers);

    let mut hasher = Sha1::new();
    hasher.update(&commit_content);
    let commit_hash = hex::encode(hasher.finalize());

    write_object(&repo_root, &commit_hash, commit_content.as_bytes())?;
    update_head(&repo_root, &commit_hash)?;

    println!("Merge complete. Created merge commit {}", &commit_hash[..7]);
    // TODO: Update working directory and index with merged content
    Ok(())
}

/// Finds the most recent common ancestor of two commits using breadth-first search.
fn find_common_ancestor(repo_root: &Path, commit1: &str, commit2: &str) -> io::Result<Option<String>> {
    let mut parents1 = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(commit1.to_string());

    // Collect all ancestors of the first commit
    while let Some(hash) = queue.pop_front() {
        if parents1.insert(hash.clone()) {
            for parent in get_commit_parents(repo_root, &hash)? {
                queue.push_back(parent);
            }
        }
    }

    // Traverse ancestors of the second commit until a match is found
    queue.clear();
    queue.push_back(commit2.to_string());
    let mut visited2 = HashSet::new();

    while let Some(hash) = queue.pop_front() {
        if parents1.contains(&hash) {
            return Ok(Some(hash)); // Found the common ancestor
        }
        if visited2.insert(hash.clone()) {
            for parent in get_commit_parents(repo_root, &hash)? {
                queue.push_back(parent);
            }
        }
    }

    Ok(None)
}

/// Reads a commit object and returns a list of its parent hashes.
fn get_commit_parents(repo_root: &Path, hash: &str) -> io::Result<Vec<String>> {
    let commit_data = read_object(repo_root, hash)?;
    let content = String::from_utf8_lossy(&commit_data);
    Ok(content
        .lines()
        .filter(|line| line.starts_with("parent "))
        .map(|line| line[7..].to_string())
        .collect())
}

/// Reads a commit object and returns its root tree hash.
fn get_tree_hash(repo_root: &Path, hash: &str) -> io::Result<String> {
    let commit_data = read_object(repo_root, hash)?;
    let content = String::from_utf8_lossy(&commit_data);
    content
        .lines()
        .find(|line| line.starts_with("tree "))
        .map(|line| line[5..].to_string())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Commit missing tree hash"))
}

/// Parses a tree object into a map of {filename -> (mode, hash)}.
fn read_tree_entries(repo_root: &Path, tree_hash: &str) -> io::Result<HashMap<PathBuf, (String, String)>> {
    let tree_data = read_object(repo_root, tree_hash)?;
    let mut entries = HashMap::new();
    let mut pos = 0;
    while pos < tree_data.len() {
        let mut end = pos;
        while tree_data[end] != 0 { end += 1; }
        let entry_str = String::from_utf8_lossy(&tree_data[pos..end]);
        let (mode, filename) = entry_str.split_once(' ').unwrap();
        let sha_start = end + 1;
        let sha_end = sha_start + 20;
        let sha_bytes = &tree_data[sha_start..sha_end];
        let sha1 = hex::encode(sha_bytes);
        entries.insert(PathBuf::from(filename), (mode.to_string(), sha1));
        pos = sha_end;
    }
    Ok(entries)
}

/// Performs a simplified three-way merge of trees.
/// NOTE: This is a simplified implementation. It merges file lists but does not handle
/// content-level merges or recursive directory merges. It will error on conflicts.
fn merge_trees(
    repo_root: &Path,
    ancestor_tree: &str,
    receiver_tree: &str, // Our current branch (e.g., main)
    giver_tree: &str,    // The branch being merged in (e.g., feature)
) -> io::Result<Vec<IndexEntry>> {
    let ancestor_entries = read_tree_entries(repo_root, ancestor_tree)?;
    let receiver_entries = read_tree_entries(repo_root, receiver_tree)?;
    let giver_entries = read_tree_entries(repo_root, giver_tree)?;

    let mut merged_entries = BTreeMap::new();

    // Union of all file paths across the three trees
    let mut all_paths = HashSet::new();
    all_paths.extend(ancestor_entries.keys().cloned());
    all_paths.extend(receiver_entries.keys().cloned());
    all_paths.extend(giver_entries.keys().cloned());

    for path in all_paths {
        let ancestor = ancestor_entries.get(&path);
        let receiver = receiver_entries.get(&path);
        let giver = giver_entries.get(&path);

        match (ancestor, receiver, giver) {
            // No changes relative to ancestor
            (Some(a), Some(r), Some(g)) if a == r && a == g => { merged_entries.insert(path, a.clone()); }
            (None, Some(r), None) => { merged_entries.insert(path, r.clone()); }
            (None, None, Some(g)) => { merged_entries.insert(path, g.clone()); }

            // Changes in only one branch (clean merge)
            (Some(a), Some(r), Some(g)) if a == g && a != r => { merged_entries.insert(path, r.clone()); } // Only receiver changed
            (Some(a), Some(r), Some(g)) if a == r && a != g => { merged_entries.insert(path, g.clone()); } // Only giver changed
            (Some(a), None, Some(g)) if a == g => {} // Receiver deleted, giver unchanged -> delete
            (Some(a), Some(r), None) if a == r => {} // Giver deleted, receiver unchanged -> delete
            (None, Some(r), Some(g)) if r == g => { merged_entries.insert(path, r.clone()); } // Both added same file

            // CONFLICTS
            (Some(_), Some(r), Some(g)) if r.1 != g.1 => { // Modified differently
                 return Err(io::Error::other(format!("Merge conflict in {}", path.display())));
            }
            (None, Some(_), Some(_)) => { // Both added same file with different content
                return Err(io::Error::other(format!("Merge conflict: both added {}", path.display())));
            }
            (Some(_), Some(_), None) => { // Receiver modified, giver deleted
                return Err(io::Error::other(format!("Merge conflict: {} modified and deleted", path.display())));
            }
            (Some(_), None, Some(_)) => { // Giver modified, receiver deleted
                return Err(io::Error::other(format!("Merge conflict: {} modified and deleted", path.display())));
            }

            // Default to receiver's version if logic is incomplete
            (_, Some(r), _) => { merged_entries.insert(path, r.clone()); }
            _ => {}
        }
    }
    
    // Convert the merged BTreeMap back into a Vec<IndexEntry>
    Ok(merged_entries.into_iter().map(|(path, (mode, sha1))| {
        IndexEntry { mode, sha1, path }
    }).collect())
}