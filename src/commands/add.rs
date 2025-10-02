use std::fs::{self, OpenOptions};
use std::path::{Path};
use std::io::{self, Write};
use sha1::{Sha1, Digest};

use super::utils::{find_repo_root, write_object };

pub fn run(file_path: &str) -> io::Result<()> {
    // 1. Locate repository root (.rit/)
    let repo_path = find_repo_root()?;
    
    // 2. Read file contents
    let contents = fs::read(file_path)?;
    
    // 3. Format blob ("blob <size>\0<content>")
    let blob_header = format!("blob {}\0", contents.len());
    let mut blob_data = Vec::new();
    blob_data.extend_from_slice(blob_header.as_bytes());
    blob_data.extend_from_slice(&contents);
    
    // 4. Hash with SHA-1
    let mut hasher = Sha1::new();
    hasher.update(&blob_data);
    let hash = hex::encode(hasher.finalize()); // 40-char hex
    
    // 5. Write blob to .rit/objects/aa/bb... file
    write_object(&repo_path, &hash, &blob_data)?;
    
    // 6. Update index (.rit/index)
    update_index(&repo_path, &hash, file_path)?;
    
    // 7. Print success
    println!("added {}", file_path);
    
    Ok(())
}


/// Append <sha1> <filename> to index
fn update_index(repo_path: &Path, hash: &str, file_path: &str) -> io::Result<()> {
    let index_path = repo_path.join("index");
    
    let mut index = OpenOptions::new()
        .create(true)
        .append(true)
        .open(index_path)?;
    
    writeln!(index, "{} {}", hash, file_path)?;
    
    Ok(())
}
