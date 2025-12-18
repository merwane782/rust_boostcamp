use clap::{Parser, Subcommand};
use rand::Rng;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

const P: u64 = 0xD87FA3E291B4C7F3;
const G: u64 = 2;

const LCG_A: u64 = 1103515245;
const LCG_C: u64 = 12345;
const LCG_M: u64 = 1u64 << 32;

#[derive(Parser, Debug)]
#[command(name = "streamchat")]
#[command(about = "P2P encrypted chat using Diffie-Hellman", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Server {
        port: u16,
    },
    Client {
        address: String,
    },
}

fn mod_pow(mut base: u128, mut exp: u128, modulus: u128) -> u64 {
    let mut result: u128 = 1;
    base %= modulus;

    while exp > 0 {
        if exp % 2 == 1 {
            result = (result * base) % modulus;
        }
        base = (base * base) % modulus;
        exp /= 2;
    }

    result as u64
}

struct StreamCipher {
    state: u64,
}

impl StreamCipher {
    fn new(seed: u64) -> Self {
        StreamCipher { state: seed }
    }

    fn next_byte(&mut self) -> u8 {
        self.state =
            ((self.state as u128 * LCG_A as u128 + LCG_C as u128) % LCG_M as u128) as u64;
        (self.state & 0xFF) as u8
    }

    fn encrypt(&mut self, data: &[u8]) -> Vec<u8> {
        data.iter().map(|&b| b ^ self.next_byte()).collect()
    }

    fn decrypt(&mut self, data: &[u8]) -> Vec<u8> {
        self.encrypt(data)
    }
}

fn diffie_hellman_exchange(stream: &mut TcpStream) -> io::Result<u64> {
    let mut rng = rand::rng();
    let private_key: u64 = rng.random_range(1000..100000);

    println!("\n🔑 Diffie-Hellman Key Exchange");
    println!(" P (prime): 0x{:016X}", P);
    println!(" G (generator): {}", G);
    println!(" Private key: {} (0x{:X})", private_key, private_key);

    let public_key = mod_pow(G as u128, private_key as u128, P as u128);
    println!(" Public key: {} (0x{:X})", public_key, public_key);

    println!("\n📤 Sending public key...");
    stream.write_all(&public_key.to_be_bytes())?;
    stream.flush()?;

    println!("📥 Receiving peer's public key...");
    let mut peer_public_key_bytes = [0u8; 8];
    stream.read_exact(&mut peer_public_key_bytes)?;
    let peer_public_key = u64::from_be_bytes(peer_public_key_bytes);
    println!(
        " Peer's public key: {} (0x{:X})",
        peer_public_key, peer_public_key
    );

    let shared_secret = mod_pow(peer_public_key as u128, private_key as u128, P as u128);
    println!(
        "\n🔐 Shared secret computed: {} (0x{:X})",
        shared_secret, shared_secret
    );

    Ok(shared_secret)
}

fn chat_loop(mut stream: TcpStream) -> io::Result<()> {
    println!("\n🤝 Establishing secure connection...");
    let shared_secret = diffie_hellman_exchange(&mut stream)?;
    println!("\n✅ Secure channel established!");
    println!("💬 You can now send messages (Ctrl+C to quit)\n");

    let stream_clone = stream.try_clone()?;
    let cipher_recv = Arc::new(Mutex::new(StreamCipher::new(shared_secret)));
    let cipher_send = Arc::new(Mutex::new(StreamCipher::new(shared_secret)));

    let cipher_recv_clone = Arc::clone(&cipher_recv);
    thread::spawn(move || {
        let mut reader = BufReader::new(stream_clone);
        loop {
            let mut length_bytes = [0u8; 2];
            if reader.read_exact(&mut length_bytes).is_err() {
                println!("\n❌ Connection closed by peer.");
                std::process::exit(0);
            }

            let length = u16::from_be_bytes(length_bytes) as usize;
            let mut encrypted_data = vec![0u8; length];
            if reader.read_exact(&mut encrypted_data).is_err() {
                println!("\n❌ Error reading message.");
                continue;
            }

            let mut cipher = cipher_recv_clone.lock().unwrap();
            let decrypted = cipher.decrypt(&encrypted_data);

            if let Ok(message) = String::from_utf8(decrypted.clone()) {
                println!("\n📨 Received: {}", message);
                println!(" [Encrypted hex: {}]", hex::encode(&encrypted_data));
                print!(">> ");
                io::stdout().flush().unwrap();
            }
        }
    });

    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();

    print!(">> ");
    io::stdout().flush()?;

    while let Some(Ok(line)) = lines.next() {
        if line.trim().is_empty() {
            print!(">> ");
            io::stdout().flush()?;
            continue;
        }

        let message_bytes = line.as_bytes();
        let mut cipher = cipher_send.lock().unwrap();
        let encrypted = cipher.encrypt(message_bytes);

        println!("📤 Sending: {}", line);
        println!(" [Plaintext hex: {}]", hex::encode(message_bytes));
        println!(" [Encrypted hex: {}]", hex::encode(&encrypted));

        let length = encrypted.len() as u16;
        stream.write_all(&length.to_be_bytes())?;
        stream.write_all(&encrypted)?;
        stream.flush()?;

        print!(">> ");
        io::stdout().flush()?;
    }

    Ok(())
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Server { port } => {
            let listener = TcpListener::bind(format!("127.0.0.1:{}", port))?;
            println!("🎧 Server listening on port {}", port);
            println!("⏳ Waiting for client connection...");
            let (stream, addr) = listener.accept()?;
            println!("✓ Client connected from {}", addr);
            chat_loop(stream)?;
        }
        Commands::Client { address } => {
            println!("🔌 Connecting to {}...", address);
            let stream = TcpStream::connect(&address)?;
            println!("✓ Connected to server!");
            chat_loop(stream)?;
        }
    }

    Ok(())
}
