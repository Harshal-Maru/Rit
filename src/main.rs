mod commands;

use std::env;

fn print_usage() {
    println!("Usage:");
    println!("  rit init                       Initialize a new repository");
    println!("  rit add <file>                 Add file to staging area");
    println!("  rit commit -m \"message\"        commit staged changes");
    println!("  rit log                        Show commit history");
    println!("  rit ls-tree <hash>             List tree contents of a commit");
    println!("  rit checkout <commit>          Checkout a specific commit");
    println!("  rit branch                     List branches");
    println!("  rit branch -c <name>           Create a new branch");
}


fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return;
    }

    match args[1].as_str() {
        "init" => {
            if let Err(e) = commands::init::run() {
                eprintln!("Error: {}", e);
            }
        }
        "add" => {
            if args.len() < 3 {
                eprintln!("Error: specify a file to add");
                return;
            }
            let file_path = &args[2];
            if let Err(e) = commands::add::run(file_path) {
                eprintln!("Error adding file: {}", e);
            }
        }
        "commit" => {
            if args.len() < 4 || args[2] != "-m" {
                eprintln!("Error: use commit -m \"message\"");
                return;
            }
            let message = &args[3];
            if let Err(e) = commands::commit::run(message) {
                eprintln!("Error committing: {}", e);
            }
        }
        "log" => {
            if let Err(e) = commands::log::run() {
                eprintln!("Error Logging: {}", e);
            }
        }
        "ls-tree" => {
            if args.len() < 3 {
                eprintln!("Error: specify a hash");
                return;
            }
            let hash = &args[2];
            if let Err(e) = commands::ls_tree::run(hash) {
                eprintln!("Error Printing ls tree: {}", e);
            }
        }
        "checkout" => {
            if args.len() < 3 {
                eprintln!("Error: specify a hash");
                return;
            }
            let hash = &args[2];
            if let Err(e) = commands::checkout::run(hash) {
                eprintln!("Error checking out the commit: {}", e);
            }
        }
        "branch" => {
            if args.len() == 2 {
                // Just `rit branch` → list branches
                if let Err(e) = commands::branch::run(None, false) {
                    eprintln!("Error: {}", e);
                }
            } else if args[2] == "-c" {
                // `rit branch -c <name>` → create branch
                if args.len() < 4 {
                    eprintln!("Error: branch name required after -c");
                    return;
                }
                let branch_name = &args[3];
                if let Err(e) = commands::branch::run(Some(branch_name), true) {
                    eprintln!("Error: {}", e);
                }
            } else {
                eprintln!("Unknown usage of branch command");
            }
        }

        _ => {
            print_usage();
        }
    }
}
