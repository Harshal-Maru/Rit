use std::io;

use super::utils::{find_repo_root, read_head_commit, read_object};

pub fn run() -> io::Result<()> {
    let repo_path = find_repo_root()?;
    let head_hash = read_head_commit(&repo_path)?;

    let mut commit_hash = head_hash.clone();
    while let Some(hash) = commit_hash {
        let commit_data = read_object(&repo_path, &hash)?;

        // Skip commit object header: "commit <size>\0"
        let content = String::from_utf8_lossy(&commit_data);
        
        // Split header from commit message
        let parts: Vec<&str> = content.splitn(2, "\n\n").collect();
        let header_lines = parts.first().unwrap_or(&"");
        let message = parts.get(1).unwrap_or(&"");

        println!("commit {}", hash);

        let mut parent: Option<String> = None;

        for line in header_lines.lines() {
            if let Some(tree_hash) = line.strip_prefix("tree ") {
                println!("Tree: {}", tree_hash);
            } else if let Some(parent_hash) = line.strip_prefix("parent ") {
                parent = Some(parent_hash.to_string());
            } else if line.starts_with("author ") {
                println!("{}", line);
            }
        }

        // Print commit message with indentation
        if !message.is_empty() {
            println!("\n    {}", message.trim());
        }

        println!();
        commit_hash = parent;
    }

    if commit_hash.is_none() && head_hash.is_none() {
        println!("No commits yet");
    }

    Ok(())
}
