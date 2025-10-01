use std::env::current_dir;
use std::fs::{create_dir_all, File};
use std::io::Write;

pub fn run() -> std::io::Result<()> {
    
    // Get current directory and append .rit
    let mut rit_path = current_dir()?;
    rit_path.push(".rit");

    // Check if .rit already exists
    if rit_path.exists() && rit_path.is_dir() {
        println!("Already a Rit repository");
        return Ok(());
    }

    // Create objects and refs/heads directories
    let objects_dir = rit_path.join("objects");
    let refs_heads_dir = rit_path.join("refs").join("heads");

    create_dir_all(&objects_dir)?;
    create_dir_all(&refs_heads_dir)?;

    // Create HEAD file pointing to main branch
    let head_file_path = rit_path.join("HEAD");
    let mut head_file = File::create(head_file_path)?;
    head_file.write_all(b"ref: refs/heads/main")?;

    println!("Rit repository successfully initialized");
    Ok(())
}
