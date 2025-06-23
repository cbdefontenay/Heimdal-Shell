use crate::chat;
use crate::chat::{ChatConfig, ChatRole};
use env::{set_current_dir, var};
use io::{stdin, stdout};
use std::env;
use std::io::{self, Write};
use std::path::Path;
use std::env::{consts, current_dir};
use tokio::process::Command;
use crate::commands::{get_os, print_fortune, print_help, print_tree, search_files};
use crate::commands_enum::commands_enum::ShellCommand;

fn parse_command(input: &str) -> ShellCommand {
    let mut parts = input.trim().split_whitespace();
    let command = parts.next().unwrap_or("");
    let args: Vec<String> = parts.map(|s| s.to_string()).collect();

    match command {
        "exit" => ShellCommand::Exit,
        "clear" => ShellCommand::Clear,
        "os" => ShellCommand::Os,
        "tree" => ShellCommand::Tree,
        "tip" | "fortune" => ShellCommand::Tip,
        "cd" => args.get(0)
            .map(|dir| ShellCommand::Cd(dir.clone()))
            .unwrap_or(ShellCommand::Cd(String::new())),
        "search" => args.get(0)
            .map(|word| ShellCommand::Search(word.clone()))
            .unwrap_or(ShellCommand::Search(String::new())),
        "whoami" => ShellCommand::Whoami,
        "heimdal" if args.get(0) == Some(&"--help".to_string()) => ShellCommand::HeimdalHelp,
        "chat" => {
            if args.len() >= 3 && args[0] == "host" {
                ShellCommand::Chat(ChatConfig {
                    role: ChatRole::Host,
                    chat_name: args[1].clone(),
                    port: args[2].parse().unwrap_or(8080),
                    remote_ip: None,
                })
            } else if args.len() >= 4 && args[0] == "guest" {
                ShellCommand::Chat(ChatConfig {
                    role: ChatRole::Guest,
                    chat_name: args[1].clone(),
                    remote_ip: Some(args[2].clone()),
                    port: args[3].parse().unwrap_or(8080),
                })
            } else {
                ShellCommand::Unknown(command.to_string(), args)
            }
        }
        _ => ShellCommand::Unknown(command.to_string(), args),
    }
}


pub async fn run() {
    loop {
        let path = current_dir().unwrap();
        let user = if cfg!(windows) {
            var("USERNAME")
        } else {
            var("USER")
        }
            .unwrap_or_else(|_| "unknown".to_string());
        let mut input = String::new();
        print!(
            "\x1b[1;32m{user}@heimdal\x1b[0m:\x1b[1;34m{}\x1b[0m$ ",
            path.display()
        );
        let _ = stdout().flush();
        stdin().read_line(&mut input).expect("Failed to read line");

        let parsed_command = parse_command(&input);

        match parsed_command {
            ShellCommand::Exit => break,
            ShellCommand::Clear => {
                print!("\x1B[2J\x1B[1;1H");
            }
            ShellCommand::Os => {
                let os = get_os();
                if os == "windows" {
                    println!("You're running on Windows.");
                } else {
                    println!("You're running on Linux.");
                }
            }
            ShellCommand::Tree => print_tree(Path::new("."), 0),
            ShellCommand::Tip => print_fortune(),
            ShellCommand::Search(word) => {
                if word.is_empty() {
                    eprintln!("search: missing keyword");
                } else {
                    search_files(&word).await;
                }
            }
            ShellCommand::Cd(dir) => {
                if dir.is_empty() {
                    eprintln!("cd: missing operand");
                } else if let Err(e) = set_current_dir(Path::new(&dir)) {
                    eprintln!("cd: {e}");
                }
            }
            ShellCommand::Whoami => {
                let user = if cfg!(windows) {
                    var("USERNAME")
                } else {
                    var("USER")
                }
                    .unwrap_or_else(|_| "unknown".to_string());

                println!("{user}");
            }
            ShellCommand::HeimdalHelp => print_help(),
            ShellCommand::Chat(config) => {
                if let Err(e) = chat::start_chat(config) {
                    eprintln!("Chat error: {}", e);
                }
            }
            ShellCommand::Unknown(cmd, args) => {
                match Command::new(&cmd).args(&args).spawn() {
                    Ok(mut child) => {
                        if let Err(e) = child.wait().await {
                            eprintln!("heimdal: command failed: {e}");
                        }
                    }
                    Err(_) => eprintln!("heimdal: command not found: {cmd}"),
                }
            }
        }
    }
}