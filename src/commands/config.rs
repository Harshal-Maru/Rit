// In commands/config.rs

use super::utils::find_repo_root;
use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, Write};

/// Main entry point for the `config` command
pub fn run(cmd: &str) -> io::Result<()> {
    if cmd == "setconfig" {
        println!("Enter the username: ");
        let mut username = String::new();
        io::stdin().read_line(&mut username).expect("Invalid username!");
        // Trim the newline character from the input
        let username = username.trim();

        println!("\nEnter the email: ");
        let mut email = String::new();
        io::stdin().read_line(&mut email).expect("invalid email!");
        // Trim the newline character from the input
        let email = email.trim();

        // Call set_config for each value separately
        set_config("user.name", username)?;
        set_config("user.email", email)?;

    } else if cmd == "getconfig" {
        // Use the full key to get the config value
        get_config("user.name")?;
        get_config("user.email")?;
    }

    Ok(())
}

/// Reads the config file into a HashMap, understanding [sections]
fn read_config_file() -> io::Result<HashMap<String, String>> {
    let repo_root = find_repo_root()?;
    let config_path = repo_root.join("config");
    let mut config = HashMap::new();
    let mut current_section = String::new();

    if config_path.exists() {
        let file = fs::File::open(config_path)?;
        let reader = io::BufReader::new(file);

        for line in reader.lines() {
            let line = line?.trim().to_string();
            if line.starts_with('[') && line.ends_with(']') {
                // We found a new section, like [user]
                current_section = line[1..line.len() - 1].to_string();
            } else if let Some((key, value)) = line.split_once('=') {
                // This is a key-value pair
                let full_key = format!("{}.{}", current_section, key.trim());
                config.insert(full_key, value.trim().to_string());
            }
        }
    }
    Ok(config)
}

/// Sets a single configuration key-value pair
fn set_config(key: &str, value: &str) -> io::Result<()> {
    let repo_root = find_repo_root()?;
    let config_path = repo_root.join("config");

    // Read all existing config values first
    let mut config = read_config_file()?;
    // Insert or update the new value
    config.insert(key.to_string(), value.to_string());

    // Re-write the entire config file to save the changes
    let mut file = fs::File::create(config_path)?;
    writeln!(file, "[user]")?;
    if let Some(name) = config.get("user.name") {
        writeln!(file, "  name = {}", name)?;
    }
    if let Some(email) = config.get("user.email") {
        writeln!(file, "  email = {}", email)?;
    }
    
    println!("Set {} = {}", key, value);
    Ok(())
}

/// Gets and prints a configuration value by its key
fn get_config(key: &str) -> io::Result<()> {
    let config = read_config_file()?;

    if let Some(value) = config.get(key) {
        // Print in a more readable format
        println!("{}: {}", key, value);
    } else {
        println!("{}: not set", key);
    }
    Ok(())
}