use std::io;

use super::utils::{find_repo_root, read_object};

pub fn run(hash: &str) -> io::Result<()> {
    let repo_path = find_repo_root()?;

    // 1. Read the object (could be commit or tree)
    let obj_data = read_object(&repo_path, hash)?;
    let content = String::from_utf8_lossy(&obj_data);

    let mut tree_hash = hash.to_string();

    // If it's a commit, extract the tree line
    if content.starts_with("tree ") {
        for line in content.lines() {
            if line.starts_with("tree ") {
                tree_hash = line[5..].to_string();
                break;
            }
        }
    }

    // 2. Read tree object
    let tree_data = read_object(&repo_path, &tree_hash)?;

    // Tree objects start with "tree <size>\0"
    let null_pos = tree_data.iter().position(|&b| b == 0).unwrap();
    let mut pos = null_pos + 1;

    while pos < tree_data.len() {
        // Parse mode + filename until \0
        let mut end = pos;
        while tree_data[end] != 0 {
            end += 1;
        }
        let entry = String::from_utf8_lossy(&tree_data[pos..end]);
        let (mode, filename) = entry.split_once(' ').unwrap();

        // SHA1 = 20 bytes after the \0
        let sha_start = end + 1;
        let sha_end = sha_start + 20;
        let sha_bytes = &tree_data[sha_start..sha_end];
        let sha1 = hex::encode(sha_bytes);

        println!("{} blob {}    {}", mode, sha1, filename);

        pos = sha_end;
    }

    Ok(())
}
