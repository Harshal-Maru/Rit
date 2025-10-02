mod commands;

use std::env;

fn print_usage() {
    println!("Usage:");
    println!("  rit init");
    println!("  rit add <file>");
    println!("  rit commit -m \"message\"");
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
        _ => {
            print_usage();
        }
    }
}
