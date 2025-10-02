use std::io;

use super::utils::{find_repo_root,  read_object, read_head_commit};

pub fn run() -> io::Result<()> {
    let repo_path = find_repo_root()?;
    let head_hash = read_head_commit(&repo_path)?;

    let mut commit_hash = head_hash.clone();
    while let Some(hash) = commit_hash {
        let commit_data = read_object(&repo_path, &hash)?;
        println!("commit {}", hash);

        // Parse commit content (UTF-8 for simplicity)
        let content = String::from_utf8_lossy(&commit_data);
        let mut parent: Option<String> = None;

        for line in content.lines() {
            if line.starts_with("tree ") {
                println!("Tree: {}", &line[5..]);
            } else if line.starts_with("parent ") {
                parent = Some(line[7..].to_string());
            } else if line.starts_with("author ") {
                println!("{}", line);
            } else if line.is_empty() {
                // Everything after this is the message
                let msg: Vec<&str> = content.splitn(2, "\n\n").collect();
                if msg.len() == 2 {
                    println!("\n    {}", msg[1].trim());
                }
                break;
            }
        }

        println!();
        commit_hash = parent;
    }
    if commit_hash.is_none() && head_hash.is_none() {
        println!("No commits yet");
    }

    Ok(())
}

