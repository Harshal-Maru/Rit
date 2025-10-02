use super::utils::{find_repo_root, read_head_commit};
use std::fs;
use std::io::{self, Write};

pub fn run(branch_name: Option<&str>, is_creating: bool) -> io::Result<()> {
    if is_creating {
        if let Some(name) = branch_name {
            create_branch(name)?;
        } else {
            eprintln!("Error: branch name required for creation");
        }
    } else {
        show_branch()?;
    }
    Ok(())
}

fn create_branch(branch_name: &str) -> io::Result<()> {
    let current_repo_path = find_repo_root()?;
    let current_commit_hash = read_head_commit(&current_repo_path)?;

    let branch_path = current_repo_path
        .join("refs")
        .join("heads")
        .join(branch_name);

    if branch_path.exists() {
        println!("Branch '{}' already exists!", branch_name);
        return Ok(());
    }

    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(branch_path)?;

    if let Some(hash) = current_commit_hash {
        file.write_all(hash.as_bytes())?;
    }

    println!("Branch '{}' created!", branch_name);
    Ok(())
}

fn show_branch() -> io::Result<()> {
    let repo_path = find_repo_root()?;
    let heads_dir = repo_path.join("refs").join("heads");

    if !heads_dir.exists() {
        println!("No branches found");
        return Ok(());
    }

    // Read HEAD to see which branch we’re on
    let head_path = repo_path.join("HEAD");
    let head_content = fs::read_to_string(head_path).unwrap_or_default();
    let current_branch = if head_content.starts_with("ref: ") {
        Some(
            head_content
                .trim_start_matches("ref: refs/heads/")
                .trim()
                .to_string(),
        )
    } else {
        None
    };

    for entry in fs::read_dir(&heads_dir)? {
        let entry = entry?;
        let branch_name = entry.file_name().into_string().unwrap();
        if Some(&branch_name) == current_branch.as_ref() {
            println!("* {}", branch_name);
        } else {
            println!("  {}", branch_name);
        }
    }

    Ok(())
}
