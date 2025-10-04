// Create a new file: src/cli.rs

use clap::{Parser, Subcommand};

/// A Git-like version control system written in Rust
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize a new Rit repository
    Init,
    
    /// Add file contents to the index.
    /// 
    /// You can use '.' to add all files and directories.
    Add {
        /// The path to the file or directory to add
        #[arg(short, long)]
        path: String,
    },
    
    /// Record changes to the repository
    /// 
    /// use -m flag to commit with a message
    Commit {
        /// The commit message
        #[arg(short, long)]
        message: String,
    },

    /// Show the commit history
    Log,

    /// List the contents of a tree object
    LsTree {
        /// The hash of the tree or commit
        #[arg(short, long)]
        hash: String,
    },

    /// Switch branches or restore working tree files
    Checkout {
        /// The commit hash or branch to switch to
        #[arg(short, long)]
        target: String,
    },

    /// List, create, or delete branches
    Branch {
        /// The name of the new branch to create
        #[arg(short, long)]
        create: Option<String>,
    },

    /// Show the working tree status
    Status,

    /// Get or set user configuration
    Config {
        /// The configuration key (e.g., user.name)
        #[arg(short, long)]
        key: String,
        
        /// The value to set for the key
        #[arg(short, long)]
        value: Option<String>,
    },
}