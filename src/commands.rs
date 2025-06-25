use rand::prelude::IndexedRandom;
use std::env::consts;
use std::path::Path;
use tokio::fs;
use walkdir::WalkDir;

pub async fn search_files(keyword: &str) {
    for entry in WalkDir::new(".").into_iter().filter_map(Result::ok) {
        if entry.path().is_file() {
            if let Ok(content) = fs::read_to_string(entry.path()).await {
                if content.contains(keyword) {
                    println!(
                        "{}: {}",
                        entry.path().display(),
                        content
                            .lines()
                            .find(|line| line.contains(keyword))
                            .unwrap_or("")
                    );
                }
            }
        }
    }
}

pub fn print_tree(path: &Path, indent: usize) {
    if let Ok(entries) = path.read_dir() {
        for entry in entries.filter_map(Result::ok) {
            let file_name = entry.file_name().into_string().unwrap_or_default();
            println!("{}{}", " ".repeat(indent), file_name);
            if entry.path().is_dir() {
                print_tree(&entry.path(), indent + 2);
            }
        }
    }
}

const TIPS: &[&str] = &[
    "Did you know? Heimdal is written in Rust!",
    "Tip: Use `cd ..` to go back one folder.",
    "You can clear the screen with `clear`.",
];

pub fn print_fortune() {
    let mut rng = rand::rng();
    if let Some(tip) = TIPS.choose(&mut rng) {
        println!("\x1b[1;36m💡 {tip}\x1b[0m\n");
    }
}

pub fn print_help() {
    println!("\n\x1b[1;36mHeimdal Shell\x1b[0m\n");
    println!("Available internal commands:");
    println!("  \x1b[1;33mcd <dir>\x1b[0m         Change directory");
    println!("  \x1b[1;33mclear\x1b[0m           Clear the screen");
    println!("  \x1b[1;33mexit\x1b[0m            Exit the shell");
    println!("  \x1b[1;33mwhoami\x1b[0m          Print current user");
    println!("  \x1b[1;33mheimdal --help\x1b[0m   Show this help message\n");
    println!("  \x1b[1;33mtree\x1b[0m            Print folder tree");
    println!("  \x1b[1;33mtip\x1b[0m             Show a random Heimdal tip");
    println!("  \x1b[1;33msearch <word>\x1b[0m   Search files for a keyword");
    println!("  \x1b[1;33mchat host <name> <port> <password>\x1b[0m   Start a chat session as host");
    println!("  \x1b[1;33mchat guest <name> <ip> <port> <password>\x1b[0m  Join a chat session as guest");

    println!(
        "External commands like \x1b[1;32mecho\x1b[0m or \x1b[1;32mls\x1b[0m are passed to the OS."
    );
    println!("You can run any system command available in your environment.\n");

    let os = consts::OS;
    if os == "windows" {
        println!(
            "⚠️  Some Unix commands like `cat`, `grep`, or `touch` may not work unless you install Git Bash or enable WSL."
        );
    }
}

pub fn get_os() -> &'static str {
    consts::OS
}
