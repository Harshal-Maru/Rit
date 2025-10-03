use super::utils::{find_repo_root, read_object};
use std::io;

pub fn run(hash: &str) -> io::Result<()> {
    let repo_path = find_repo_root()?;

    // 1. Read the object (commit or tree)
    let obj_data = read_object(&repo_path, hash)?;
    let mut tree_hash = hash.to_string();

    // Check if object is a commit (skip "commit <size>\0" header)
    let content = String::from_utf8_lossy(&obj_data);
    if content.starts_with("tree ") || content.starts_with("commit ") {
        // Skip header if commit object
        let null_pos = obj_data.iter().position(|&b| b == 0).unwrap_or(0);
        let body = &obj_data[null_pos + 1..];
        let body_str = String::from_utf8_lossy(body);

        for line in body_str.lines() {
            if line.starts_with("tree ") {
                tree_hash = line[5..].to_string();
                break;
            }
        }
    }

    // 2. Read tree object (already stripped header)
    let tree_data = read_object(&repo_path, &tree_hash)?;
    let mut pos = 0;

    while pos < tree_data.len() {
        // Find end of mode+filename
        let mut end = pos;
        while end < tree_data.len() && tree_data[end] != 0 {
            end += 1;
        }

        if end >= tree_data.len() {
            break; // malformed entry
        }

        let entry = String::from_utf8_lossy(&tree_data[pos..end]);
        let (mode, filename) = entry.split_once(' ').unwrap();

        let sha_start = end + 1;
        let sha_end = sha_start + 20;

        if sha_end > tree_data.len() {
            eprintln!("Warning: tree entry '{}' has invalid SHA1 length", filename);
            break;
        }

        let sha_bytes = &tree_data[sha_start..sha_end];
        let sha1 = hex::encode(sha_bytes);

        // Determine object type based on mode
        let obj_type = match mode {
            "40000" => "tree",
            "120000" => "symlink",
            _ if mode.starts_with("100") => "blob",
            _ => "blob", // default fallback
        };

        println!("{} {} {} {}", mode, obj_type, sha1, filename);

        pos = sha_end;
    }

    Ok(())
}
