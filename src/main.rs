// In your updated main.rs

mod cli;
mod commands; // Add this module

use clap::Parser;
use cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    let result = match &cli.command {
        Commands::Init => commands::init::run(),
        Commands::Add { path } => commands::add::run(path),
        Commands::Commit { message } => commands::commit::run(message),
        Commands::Log => commands::log::run(),
        Commands::LsTree { hash } => commands::ls_tree::run(hash),
        Commands::Checkout { target } => commands::checkout::run(target),
        Commands::Status => commands::status::run(),

        Commands::Branch { create } => {
            // Check if the -c flag was used
            if let Some(branch_name) = create {
                commands::branch::run(Some(branch_name), true) // Create branch
            } else {
                commands::branch::run(None, false) // List branches
            }
        }

        Commands::Config { key, value } => {
            // The `value` from clap is an Option<String>.
            // We need to pass an Option<&str> to our run function.
            // `.as_deref()` is a perfect and concise way to do this conversion.
            commands::config::run(key, value.as_deref())
        }
        // In src/main.rs -> inside the match &cli.command { ... } block
        Commands::Remove { path, cached } => commands::remove::run(path, *cached),

        Commands::Diff { path } => commands::diff::run(path.as_deref()),
        
        Commands::Merge { branch } => commands::merge::run(branch),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
    }
}
