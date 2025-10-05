use super::utils::{find_repo_root, read_full_object, read_object};
use std::io;

pub fn run(hash: &str) -> io::Result<()> {
    let repo_path = find_repo_root()?;
    let mut tree_hash = hash.to_string();

    // 1. Read the full object data to inspect its header.
    let full_obj_data = read_full_object(&repo_path, hash)?;
    let obj_str = String::from_utf8_lossy(&full_obj_data);

    // 2. If it's a commit object, parse it to find the tree hash.
    if obj_str.starts_with("commit ")
        && let Some(pos) = full_obj_data.iter().position(|&b| b == 0) {
            let body = String::from_utf8_lossy(&full_obj_data[pos + 1..]);
            tree_hash = body
                .lines()
                .find(|line| line.starts_with("tree "))
                .map(|line| line[5..].to_string())
                .ok_or_else(|| io::Error::other("Commit object is missing a tree"))?;
        }

    // 3. Read the tree object's *content* (without header) to parse entries.
    let tree_data = read_object(&repo_path, &tree_hash)?;
    let mut pos = 0;
    while pos < tree_data.len() {
        let mut end = pos;
        while end < tree_data.len() && tree_data[end] != 0 { end += 1; }
        if end >= tree_data.len() { break; }

        let entry = String::from_utf8_lossy(&tree_data[pos..end]);
        let (mode, filename) = entry.split_once(' ').unwrap();

        let sha_start = end + 1;
        let sha_end = sha_start + 20;
        if sha_end > tree_data.len() { break; }

        let sha_bytes = &tree_data[sha_start..sha_end];
        let sha1 = hex::encode(sha_bytes);
        
        let obj_type = if mode == "40000" { "tree" } else { "blob" };

        println!("{} {} {}\t{}", mode, obj_type, sha1, filename);
        pos = sha_end;
    }

    Ok(())
}