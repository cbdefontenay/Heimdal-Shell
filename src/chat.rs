use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

#[derive(Clone, Copy)]
pub enum ChatRole {
    Host,
    Guest,
}

pub struct ChatConfig {
    pub role: ChatRole,
    pub chat_name: String,
    pub port: u16,
    pub remote_ip: Option<String>,
}

pub fn start_chat(config: ChatConfig) -> io::Result<()> {
    match config.role {
        ChatRole::Host => {
            println!(
                "Starting chat '{}' as host on port {}...",
                config.chat_name, config.port
            );
            host_chat(config.port)
        }
        ChatRole::Guest => {
            if let Some(ip) = config.remote_ip {
                println!(
                    "Connecting to chat '{}' at {}:{}...",
                    config.chat_name, ip, config.port
                );
                guest_chat(&ip, config.port)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Remote IP is required for guest mode",
                ))
            }
        }
    }
}

fn host_chat(port: u16) -> io::Result<()> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port))?;
    println!("Waiting for connection...");

    let (mut stream, addr) = listener.accept()?;
    println!("Connected with {}", addr);

    let mut stream_clone = stream.try_clone()?;
    let read_thread = thread::spawn(move || {
        let mut buffer = [0; 1024];
        loop {
            match stream_clone.read(&mut buffer) {
                Ok(0) => {
                    println!("Connection closed by remote.");
                    break;
                }
                Ok(n) => {
                    let msg = String::from_utf8_lossy(&buffer[..n]);
                    print!("\nRemote: {}", msg);
                    io::stdout().flush().unwrap();
                }
                Err(e) => {
                    eprintln!("Read error: {}", e);
                    break;
                }
            }
        }
    });

    println!("Type your messages (press Enter to send):");
    loop {
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if input.trim() == "/exit" {
            break;
        }
        if let Err(e) = stream.write_all(input.as_bytes()) {
            eprintln!("Write error: {}", e);
            break;
        }
    }

    read_thread.join().unwrap();
    Ok(())
}

fn guest_chat(ip: &str, port: u16) -> io::Result<()> {
    let mut stream = loop {
        match TcpStream::connect(format!("{}:{}", ip, port)) {
            Ok(stream) => break stream,
            Err(e) => {
                eprintln!("Failed to connect: {}. Retrying in 1 second...", e);
                thread::sleep(Duration::from_secs(1));
            }
        }
    };

    println!("Connected to host!");

    let mut stream_clone = stream.try_clone()?;
    let read_thread = thread::spawn(move || {
        let mut buffer = [0; 1024];
        loop {
            match stream_clone.read(&mut buffer) {
                Ok(0) => {
                    println!("Connection closed by remote.");
                    break;
                }
                Ok(n) => {
                    let msg = String::from_utf8_lossy(&buffer[..n]);
                    print!("\nRemote: {}", msg);
                    io::stdout().flush().unwrap();
                }
                Err(e) => {
                    eprintln!("Read error: {}", e);
                    break;
                }
            }
        }
    });

    println!("Type your messages (press Enter to send):");
    loop {
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if input.trim() == "/exit" {
            break;
        }
        if let Err(e) = stream.write_all(input.as_bytes()) {
            eprintln!("Write error: {}", e);
            break;
        }
    }

    read_thread.join().unwrap();
    Ok(())
}