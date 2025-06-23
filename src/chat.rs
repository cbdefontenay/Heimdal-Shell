use aes_gcm::aead::{Aead, AeadCore, Key, KeyInit};
use aes_gcm::Aes256Gcm;
use generic_array::GenericArray;
use pbkdf2::pbkdf2_hmac;
use rand::{rngs::OsRng, TryRngCore};
use serde::{Deserialize, Serialize};
use serde_json;
use sha2::{Digest, Sha256};
use std::io::{self, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream, Shutdown};
use std::thread;
use std::time::Duration;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::os::unix::io::{AsRawFd};

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const FAINT: &str = "\x1b[2m";
const GREEN: &str = "\x1b[92m";
const CYAN: &str = "\x1b[96m";
const RED: &str = "\x1b[91m";
const YELLOW: &str = "\x1b[93m";
const MAGENTA: &str = "\x1b[95m";

type GcmNonce = GenericArray<u8, <Aes256Gcm as AeadCore>::NonceSize>;

#[derive(Debug, Serialize, Deserialize)]
pub struct EncryptedMessage {
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

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
    pub password: Option<String>,
}

pub fn start_chat(config: ChatConfig) -> io::Result<()> {
    let password = config.password.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Password is required for secure chat",
        )
    })?;

    let salt = b"some_fixed_salt_for_heimdal_chat";
    let mut key_bytes = [0u8; 32];
    pbkdf2_hmac::<Sha256>(&password.as_bytes(), salt, 100_000, &mut key_bytes);

    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);

    println!("{}████████████████████████████████████████████{}", GREEN, RESET);
    println!("{}█{} {}HEIMDAL SECURE CHAT INTERFACE{}{}{}", BOLD, FAINT, MAGENTA, RESET, BOLD, RESET);
    println!("{}████████████████████████████████████████████{}", GREEN, RESET);
    println!("{}>> Key derivation complete. Initializing secure channel...{}", YELLOW, RESET);
    println!("{}>> Chatting securely.{}", GREEN, RESET);


    match config.role {
        ChatRole::Host => {
            println!(
                "{}>> Starting host session '{}' on port {}...{}",
                GREEN, config.chat_name, config.port, RESET
            );
            host_chat(config.port, cipher)
        }
        ChatRole::Guest => {
            if let Some(ip) = config.remote_ip {
                println!(
                    "{}>> Attempting to connect to '{}' at {}:{}...{}",
                    CYAN, config.chat_name, ip, config.port, RESET
                );
                guest_chat(&ip, config.port, cipher)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("{}ERROR: Remote IP is required for guest mode{}", RED, RESET),
                ))
            }
        }
    }
}

fn send_encrypted_message(
    stream: &mut TcpStream,
    cipher: &Aes256Gcm,
    message: &str,
) -> io::Result<()> {
    let mut nonce_bytes = [0u8; 12];
    let mut rng = OsRng::default();
    rng.try_fill_bytes(&mut nonce_bytes).unwrap();

    let nonce = GcmNonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, message.as_bytes())
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{}ERROR: Encryption failed: {}{}", RED, e, RESET)))?;

    let encrypted_msg = EncryptedMessage {
        nonce: nonce_bytes.to_vec(),
        ciphertext,
    };

    let encoded = serde_json::to_vec(&encrypted_msg)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{}ERROR: Serialization failed: {}{}", RED, e, RESET)))?;

    let len_bytes = (encoded.len() as u32).to_be_bytes();
    stream.write_all(&len_bytes)?;
    stream.write_all(&encoded)?;
    stream.flush()?;
    Ok(())
}

fn receive_and_decrypt_message(
    reader: &mut BufReader<TcpStream>,
    cipher: &Aes256Gcm,
    exit_signal: &AtomicBool,
) -> io::Result<Option<String>> {
    let mut len_bytes = [0u8; 4];

    reader.get_mut().set_nonblocking(true)?;
    let read_result = loop {
        if exit_signal.load(Ordering::SeqCst) {
            reader.get_mut().set_nonblocking(false)?;
            return Ok(None);
        }
        match reader.read_exact(&mut len_bytes) {
            Ok(_) => break Ok(()),
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(50));
                continue;
            },
            Err(e) => break Err(e),
        }
    };
    reader.get_mut().set_nonblocking(false)?;

    read_result?;

    let msg_len = u32::from_be_bytes(len_bytes) as usize;

    let mut encoded = vec![0u8; msg_len];
    reader.read_exact(&mut encoded)?;

    let encrypted_msg: EncryptedMessage = serde_json::from_slice(&encoded).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{}ERROR: Deserialization failed: {}{}", RED, e, RESET),
        )
    })?;

    let nonce = GcmNonce::from_slice(&encrypted_msg.nonce);
    let plaintext = cipher
        .decrypt(nonce, encrypted_msg.ciphertext.as_ref())
        .map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{}ERROR: Decryption failed: {}{}", RED, e, RESET),
            )
        })?;

    Ok(Some(String::from_utf8_lossy(&plaintext).to_string()))
}

fn host_chat(port: u16, cipher: Aes256Gcm) -> io::Result<()> {
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr)?;

    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;
        let fd = listener.as_raw_fd();
        unsafe {
            let reuse_addr: i32 = 1;
            if libc::setsockopt(fd, libc::SOL_SOCKET, libc::SO_REUSEADDR,
                                &reuse_addr as *const i32 as *const libc::c_void,
                                std::mem::size_of_val(&reuse_addr) as libc::socklen_t) != 0 {
                return Err(io::Error::last_os_error());
            }
        }
    }

    println!("{}>> Waiting for incoming connection...{}", YELLOW, RESET);

    let (mut stream, addr) = listener.accept()?;
    println!("{}>> Connection established with: {}{}", GREEN, addr, RESET);

    let cipher_read_thread = cipher.clone();
    let reader_stream_clone = stream.try_clone()?;

    let should_read_thread_exit = Arc::new(AtomicBool::new(false));
    let read_thread_exit_signal_clone = should_read_thread_exit.clone();


    let read_thread = thread::spawn(move || {
        let mut reader = BufReader::new(reader_stream_clone);
        loop {
            match receive_and_decrypt_message(&mut reader, &cipher_read_thread, &read_thread_exit_signal_clone) {
                Ok(Some(msg)) => {
                    print!("\n{}[INCOMING PAYLOAD]: {}{}\n", CYAN, msg, RESET);
                    io::stdout().flush().unwrap();
                }
                Ok(None) => {
                    if !read_thread_exit_signal_clone.load(Ordering::SeqCst) {
                        println!("{}>> Remote connection terminated.{}", YELLOW, RESET);
                    } else {
                        println!("{}>> Read thread terminating as signaled.{}", FAINT, RESET);
                    }
                    break;
                }
                Err(e) => {
                    eprintln!("{}CRITICAL ERROR: Read/Decryption failure: {}{}", RED, e, RESET);
                    break;
                }
            }
        }
    });

    let mut user_exited_chat = false;
    println!("{}>> Session Active. Type your secure messages (press Enter to send, /exit to terminate):{}", BOLD, RESET);
    loop {
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if input.trim() == "/exit" {
            println!("{}>> Initiating session termination...{}", YELLOW, RESET);
            should_read_thread_exit.store(true, Ordering::SeqCst);
            let _ = stream.shutdown(Shutdown::Read);
            user_exited_chat = true;
            break;
        }

        if let Err(e) = send_encrypted_message(&mut stream, &cipher, input.trim()) {
            eprintln!("{}CRITICAL ERROR: Write/Encryption failure: {}{}", RED, e, RESET);
            should_read_thread_exit.store(true, Ordering::SeqCst);
            let _ = stream.shutdown(Shutdown::Read);
            user_exited_chat = true;
            break;
        }
    }

    read_thread.join().unwrap();

    println!("{}>> Session terminated.{}", YELLOW, RESET);

    if user_exited_chat {
        Err(io::Error::new(io::ErrorKind::Interrupted, "Chat session explicitly exited by user, signaling shell termination"))
    } else {
        Ok(())
    }
}

fn guest_chat(ip: &str, port: u16, cipher: Aes256Gcm) -> io::Result<()> {
    let mut stream = loop {
        match TcpStream::connect(format!("{}:{}", ip, port)) {
            Ok(stream) => break stream,
            Err(e) => {
                eprintln!("{}ERROR: Connection failed: {}. Retrying in 1 second...{}", RED, e, RESET);
                thread::sleep(Duration::from_secs(1));
            }
        }
    };

    println!("{}>> Successfully established connection to host!{}", GREEN, RESET);

    let cipher_read_thread = cipher.clone();
    let reader_stream_clone = stream.try_clone()?;

    let should_read_thread_exit = Arc::new(AtomicBool::new(false));
    let read_thread_exit_signal_clone = should_read_thread_exit.clone();

    let read_thread = thread::spawn(move || {
        let mut reader = BufReader::new(reader_stream_clone);
        loop {
            match receive_and_decrypt_message(&mut reader, &cipher_read_thread, &read_thread_exit_signal_clone) {
                Ok(Some(msg)) => {
                    print!("\n{}[INCOMING PAYLOAD]: {}{}\n", CYAN, msg, RESET);
                    io::stdout().flush().unwrap();
                }
                Ok(None) => {
                    if !read_thread_exit_signal_clone.load(Ordering::SeqCst) {
                        println!("{}>> Remote connection terminated.{}", YELLOW, RESET);
                    } else {
                        println!("{}>> Read thread terminating as signaled.{}", FAINT, RESET);
                    }
                    break;
                }
                Err(e) => {
                    eprintln!("{}CRITICAL ERROR: Read/Decryption failure: {}{}", RED, e, RESET);
                    break;
                }
            }
        }
    });

    let mut user_exited_chat = false;
    println!("{}>> Session Active. Type your secure messages (press Enter to send, /exit to terminate):{}", BOLD, RESET);
    loop {
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if input.trim() == "/exit" {
            println!("{}>> Initiating session termination...{}", YELLOW, RESET);
            should_read_thread_exit.store(true, Ordering::SeqCst);
            let _ = stream.shutdown(Shutdown::Read);
            user_exited_chat = true;
            break;
        }

        if let Err(e) = send_encrypted_message(&mut stream, &cipher, input.trim()) {
            eprintln!("{}CRITICAL ERROR: Write/Encryption failure: {}{}", RED, e, RESET);
            should_read_thread_exit.store(true, Ordering::SeqCst);
            let _ = stream.shutdown(Shutdown::Read);
            user_exited_chat = true;
            break;
        }
    }

    read_thread.join().unwrap();

    println!("{}>> Session terminated.{}", YELLOW, RESET);

    if user_exited_chat {
        Err(io::Error::new(io::ErrorKind::Interrupted, "Chat session explicitly exited by user, signaling shell termination"))
    } else {
        Ok(())
    }
}