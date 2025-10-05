// In src/cli.rs

use clap::{Parser, Subcommand};

/// Rit: A simple, Git-like version control system written in Rust.
///
/// This tool allows you to initialize repositories, track changes to files,
/// commit snapshots of your work, and manage branches.
#[derive(Parser, Debug)]
#[command(author, version, long_about = None)] // We'll let the multi-line comment serve as the long_about
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initializes a new Rit repository in the current directory.
    ///
    /// This command creates a new '.rit' subdirectory, which will contain all the
    /// necessary files for your repository, including the object database and refs.
    Init,

    /// Adds file contents to the staging area (the index).
    ///
    /// This command updates the index to match the current content of the files in
    /// your working directory. You can specify a single file, a directory, or use '.'
    /// to add all changes in the current directory.
    #[command(after_help = "EXAMPLES:\n    rit add src/main.rs\n    rit add .")]
    Add {
        /// The path to the file or directory to add
        path: String,
    },

    /// Records a snapshot of the staged changes to the repository.
    ///
    /// Commits are permanent snapshots of your project's history. Each commit
    /// has a unique hash, an author, a timestamp, and a message.
    #[command(after_help = "EXAMPLE:\n    rit commit -m \"feat: Implement the new login page\"")]
    Commit {
        /// The commit message, describing the changes made
        #[arg(short, long)]
        message: String,
    },

    /// Displays the commit history of the current branch.
    ///
    /// Traverses the commit graph backwards from the current HEAD, showing the
    /// author, date, and message for each commit.
    Log,

    /// Lists the contents of a given tree object.
    ///
    /// A tree object represents a directory in the repository. This command will
    /// show the blobs (files) and other trees (subdirectories) it contains.
    LsTree {
        /// The 40-character SHA-1 hash of the tree or a commit object
        hash: String,
    },

    /// Switches the current HEAD to a specified commit or branch.
    ///
    /// This command updates the files in your working directory to match the
    /// version stored in the target commit or branch.
    /// WARNING: This can overwrite uncommitted changes in your working directory.
    Checkout {
        /// The commit hash or branch name to switch to
        target: String,
    },

    /// Manages branches in the repository.
    ///
    /// When run without arguments, it lists all local branches.
    /// Use the '-c' or '--create' flag to create a new branch from the current HEAD.
    Branch {
        /// Creates a new branch with the given name
        #[arg(short, long)]
        create: Option<String>,
    },

    /// Shows the status of the working directory and the staging area.
    ///
    /// This command displays which files have been modified, which are staged
    /// for the next commit, and which files are new and untracked by Rit.
    Status,

    /// Gets or sets user-specific configuration options, like name and email.
    ///
    /// This information is used to identify the author of commits.
    Config {
        /// The configuration key to get or set (e.g., user.name)
        key: String,

        /// The value to set for the given key
        value: Option<String>,
    },

    /// Removes files from the staging area (the index).
    ///
    /// This can be used to unstage a file or to both unstage and delete a file
    /// from the working directory.
    #[command(
        alias = "rm",
        after_help = "EXAMPLES:\n    rit rm --cached src/temp.log\n    rit rm src/mistake.txt"
    )]
    Remove {
        /// The path of the file to remove from the index
        path: String,

        /// Use this flag to unstage the file but keep it on disk
        #[arg(long, short)]
        cached: bool,
    },

    /// Shows changes between the working tree and the index.
    Diff {
        /// The specific file to diff. If not provided, shows all changes.
        path: Option<String>,
    },

    Merge {
        /// The name of the branch to merge in
        branch: String,
    },
}
