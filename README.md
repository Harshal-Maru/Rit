# Rit

A minimal version control system built in Rust to understand how Git works under the hood.

## Why I Built This

I wanted to understand what actually happens when you run `git commit` or `git merge`. Reading Git's source code felt overwhelming, so I decided to build my own simplified version from scratch. This project taught me about content-addressable storage, tree structures, and how version control systems manage file history.

## What It Does

Rit implements the core features you'd use daily with Git:

```bash
rit init                    # Initialize a repository
rit add .                   # Stage files
rit commit -m "message"     # Create a commit
rit branch feature          # Create a branch
rit checkout feature        # Switch branches
rit merge main              # Merge branches
rit log                     # View history
rit diff                    # Show changes
rit status                  # Show working directory status
rit ls-tree <hash>          # View tree object contents
```

Additional features:
- `.ritignore` files (works like `.gitignore`)
- Nested directory handling
- Merge conflict detection
- File removal with `rit rm`
- User configuration with `rit config user.name` and `rit config user.email`

## Installation

Requires Rust 1.70 or later.

```bash
git clone https://github.com/Harshal-Maru/Rit.git
cd Rit
cargo build --release

# Optional: add to PATH
cp target/release/rit /usr/local/bin/
```

## Quick Example

```bash
mkdir test-project && cd test-project
rit init
echo "Hello, Rit!" > hello.txt
rit add .
rit commit -m "First commit"
rit log
```

## How It Works

### Object Storage
Files are stored as compressed blobs identified by SHA-1 hashes. Each commit creates a tree object that references these blobs, similar to Git's object database structure. Uses Zlib compression for efficient storage.

### Staging Area
The index tracks which files go into the next commit. Stored in `.rit/index` as a simple file listing.

### Branching Model
Branches are pointers to commits in `.rit/refs/heads/`. HEAD tracks the current branch. Switching branches updates the working directory to match that commit's tree.

### Merge Implementation
Uses three-way merge:
1. Finds common ancestor commit
2. Compares changes from both branches
3. Detects conflicts when same lines differ

Conflicts are detected but not auto-resolved. You'll see which files have conflicts and need to fix them manually.

## Project Structure

```
src/
├── main.rs           # Entry point and CLI setup
├── cli.rs            # Command definitions (clap)
└── commands/
    ├── init.rs       # Creates .rit directory structure
    ├── add.rs        # Updates staging area
    ├── commit.rs     # Creates commit objects
    ├── branch.rs     # Branch operations
    ├── checkout.rs   # Switches branches/commits
    ├── merge.rs      # Three-way merge logic
    ├── diff.rs       # File comparison
    └── utils.rs      # Shared utilities
```

## Tech Stack

- **Rust** - Memory safety and performance
- **clap** - Command-line parsing
- **sha1** - Content hashing for objects
- **flate2** - Compressing stored objects (Zlib)
- **dissimilar** - Text diffing algorithm
- **colored** - Terminal output formatting

## Key Learnings

- Content-addressable storage using SHA-1 hashes
- Difference between blob, tree, and commit objects
- How the staging area bridges working directory and commits
- Three-way merge algorithm implementation
- Rust's ownership model for managing file operations safely

## Limitations

Built for learning, not production:
- No remote operations (push/pull/fetch)
- Manual conflict resolution only
- No object garbage collection
- No packfiles or delta compression
- Single-threaded operations

## Future Work

Things I might add later:
- Basic rebase support
- Tag support
- Stash functionality

---

Built to learn version control internals. Use Git for actual projects.