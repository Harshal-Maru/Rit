// In commands/config.rs

use super::utils::find_repo_root;
use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, Write};

/// The new run function signature matches how it's called in main.rs
pub fn run(key: &str, value: Option<&str>) -> io::Result<()> {
    if let Some(val) = value {
        // If a value is provided, we set the config key.
        set_config(key, val)?;
    } else {
        // If no value is provided, we get the config key.
        get_config(key)?;
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
                current_section = line[1..line.len() - 1].to_string();
            } else if let Some((key, value)) = line.split_once('=') {
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
    let mut config = read_config_file()?;
    config.insert(key.to_string(), value.to_string());

    let mut file = fs::File::create(config_path)?;
    writeln!(file, "[user]")?;
    if let Some(name) = config.get("user.name") {
        writeln!(file, "  name = {}", name)?;
    }
    if let Some(email) = config.get("user.email") {
        writeln!(file, "  email = {}", email)?;
    }
    
    // We don't need a "Set..." message here anymore, clap handles feedback.
    Ok(())
}

/// Gets and prints a configuration value by its key
fn get_config(key: &str) -> io::Result<()> {
    let config = read_config_file()?;
    if let Some(value) = config.get(key) {
        println!("{}", value);
    }
    Ok(())
}