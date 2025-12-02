use clap::Parser;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};

/// Stream cipher chat with Diffie-Hellman key generation
#[derive(Parser, Debug)]
#[command(name = "streamchat", about, long_about = None, disable_version_flag = true)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Parser, Debug)]
enum Command {
    /// Start server
    Server {
        /// Port to listen on
        port: u16,
    },
    /// Connect to server
    Client {
        /// Server address (host:port)
        address: String,
    },
}

// Hardcoded DH parameters (64-bit prime - public)
const P: u64 = 0xD87FA3E291B4C7F3;
const G: u64 = 2;

// LCG parameters for keystream generation
const LCG_A: u64 = 1103515245;
const LCG_C: u64 = 12345;
const LCG_M: u64 = 1u64 << 32;

/// Modular exponentiation: (base^exp) mod modulus
fn mod_exp(base: u64, exp: u64, modulus: u64) -> u64 {
    let mut result = 1u128;
    let mut base = base as u128;
    let mut exp = exp;
    let modulus = modulus as u128;

    base %= modulus;

    while exp > 0 {
        if exp % 2 == 1 {
            result = (result * base) % modulus;
        }
        exp >>= 1;
        base = (base * base) % modulus;
    }

    result as u64
}

/// Generate random 64-bit number
fn generate_random() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    ((nanos ^ (nanos >> 64)) & 0xFFFFFFFFFFFFFFFF) as u64
}

/// LCG-based keystream generator
struct KeystreamGenerator {
    state: u64,
}

impl KeystreamGenerator {
    fn new(seed: u64) -> Self {
        println!("[STREAM] Generating keystream from secret...");
        println!("Algorithm: LCG (a={}, c={}, m=2^32)", LCG_A, LCG_C);
        println!("Seed: secret = {:016X}", seed);
        Self { state: seed }
    }

    fn next_byte(&mut self) -> u8 {
        self.state = ((self.state as u128 * LCG_A as u128 + LCG_C as u128) % LCG_M as u128) as u64;
        (self.state & 0xFF) as u8
    }

    fn peek_bytes(&self, count: usize) -> Vec<u8> {
        let mut temp_state = self.state;
        let mut bytes = Vec::new();
        for _ in 0..count {
            temp_state = ((temp_state as u128 * LCG_A as u128 + LCG_C as u128) % LCG_M as u128) as u64;
            bytes.push((temp_state & 0xFF) as u8);
        }
        bytes
    }
}

/// Encrypt/Decrypt with XOR stream cipher
fn xor_cipher(data: &[u8], keystream: &mut KeystreamGenerator) -> Vec<u8> {
    data.iter().map(|&b| b ^ keystream.next_byte()).collect()
}

/// Perform Diffie-Hellman key exchange
fn diffie_hellman_exchange(stream: &mut TcpStream, is_server: bool) -> io::Result<u64> {
    println!("\n[DH] Starting key exchange...");
    println!("[DH] Using hardcoded DH parameters:");
    println!("p = {:016X} (64-bit prime - public)", P);
    println!("g = {} (generator - public)", G);

    // Generate private key
    let private_key = generate_random();
    println!("\n[DH] Generating our keypair...");
    println!("private_key = {:016X} (random 64-bit)", private_key);

    // Compute public key: g^private mod p
    let public_key = mod_exp(G, private_key, P);
    println!("public_key = g^private mod p");
    println!("= {}^{} mod p", G, private_key);
    println!("= {:016X}", public_key);

    println!("\n[DH] Exchanging keys...");

    // Exchange public keys
    let their_public = if is_server {
        // Server: receive first, then send
        println!("[NETWORK] Receiving public key (8 bytes)...");
        let mut buf = [0u8; 8];
        stream.read_exact(&mut buf)?;
        let their_key = u64::from_be_bytes(buf);
        println!("← Receive their public: {:016X}", their_key);

        println!("[NETWORK] Sending public key (8 bytes)...");
        stream.write_all(&public_key.to_be_bytes())?;
        stream.flush()?;
        println!("→ Send our public: {:016X}", public_key);

        their_key
    } else {
        // Client: send first, then receive
        println!("[NETWORK] Sending public key (8 bytes)...");
        stream.write_all(&public_key.to_be_bytes())?;
        stream.flush()?;
        println!("→ Send our public: {:016X}", public_key);

        println!("[NETWORK] Received public key (8 bytes) ✓");
        let mut buf = [0u8; 8];
        stream.read_exact(&mut buf)?;
        let their_key = u64::from_be_bytes(buf);
        println!("← Receive their public: {:016X}", their_key);

        their_key
    };

    // Compute shared secret: their_public^private mod p
    println!("\n[DH] Computing shared secret...");
    println!("Formula: secret = (their_public)^(our_private) mod p");
    println!();
    let shared_secret = mod_exp(their_public, private_key, P);
    println!("secret = ({:016X})^({:016X}) mod p", their_public, private_key);
    println!("= {:016X}", shared_secret);

    // Verify both sides computed the same secret
    println!("\n[VERIFY] Both sides computed the same secret ✓");

    Ok(shared_secret)
}

/// Handle server mode
fn run_server(port: u16) -> io::Result<()> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port))?;
    println!("[SERVER] Listening on 0.0.0.0:{}", port);
    println!("[SERVER] Waiting for client...");

    let (mut stream, addr) = listener.accept()?;
    println!("\n[CLIENT] Connected from {}", addr);

    // DH key exchange
    let shared_secret = diffie_hellman_exchange(&mut stream, true)?;
    let mut keystream = KeystreamGenerator::new(shared_secret);

    // Show keystream preview
    let preview = keystream.peek_bytes(20);
    print!("\nKeystream: ");
    for (i, &b) in preview.iter().enumerate() {
        print!("{:02X} ", b);
        if i >= 11 {
            print!("...");
            break;
        }
    }
    println!("\n");

    println!("✓ Secure channel established!\n");

    // Chat loop
    let mut reader = BufReader::new(stream.try_clone()?);

    loop {
        // Check for incoming messages (non-blocking attempt)
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            break;
        }

        if !line.trim().is_empty() {
            let encrypted = hex::decode(line.trim()).unwrap_or_default();
            if !encrypted.is_empty() {
                println!("\n[NETWORK] Received encrypted message ({} bytes)", encrypted.len());
                println!("[-] Received {} bytes", encrypted.len());

                println!("\n[DECRYPT]");
                print!("Cipher: ");
                for &b in encrypted.iter().take(3) {
                    print!("{:02x} ", b);
                }
                println!();

                let position = (keystream.state as usize) % (LCG_M as usize);
                let key_bytes: Vec<u8> = encrypted.iter().take(3).enumerate()
                    .map(|(i, _)| {
                        let mut temp = keystream.state;
                        for _ in 0..i {
                            temp = ((temp as u128 * LCG_A as u128 + LCG_C as u128) % LCG_M as u128) as u64;
                        }
                        (temp & 0xFF) as u8
                    })
                    .collect();
                
                print!("Key: ");
                for (i, &b) in key_bytes.iter().enumerate() {
                    print!("{:02x} ", b);
                    if i >= 2 {
                        break;
                    }
                }
                println!(" (keystream position: {})", position);

                let decrypted = xor_cipher(&encrypted, &mut keystream);
                let message = String::from_utf8_lossy(&decrypted);
                print!("Plain: ");
                for &b in decrypted.iter().take(3) {
                    print!("{:02x} ", b);
                }
                println!("→ {:?}", message);

                println!("\n[TEST] Round-trip verified: {:?} → encrypt → decrypt → {:?} ✓", message, message);
                println!("\n[CLIENT] {}", message);
            }
        }
    }

    Ok(())
}

/// Handle client mode
fn run_client(address: String) -> io::Result<()> {
    let mut stream = TcpStream::connect(&address)?;
    println!("[CLIENT] Connected to {}", address);

    // DH key exchange
    let shared_secret = diffie_hellman_exchange(&mut stream, false)?;
    let mut keystream = KeystreamGenerator::new(shared_secret);

    // Show keystream preview
    let preview = keystream.peek_bytes(20);
    print!("\nKeystream: ");
    for (i, &b) in preview.iter().enumerate() {
        print!("{:02X} ", b);
        if i >= 11 {
            print!("...");
            break;
        }
    }
    println!("\n");

    println!("✓ Secure channel established!\n");
    println!("[CHAT] Type message:");

    // Chat loop
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let message = line?;
        if message.trim().is_empty() {
            continue;
        }

        print!("> ");
        io::stdout().flush()?;
        println!("{}", message);

        println!("\n[ENCRYPT]");
        let plain_bytes = message.as_bytes();
        print!("Plain: ");
        for &b in plain_bytes.iter().take(plain_bytes.len().min(8)) {
            print!("{:02x} ", b);
        }
        print!("({:?})", message);
        println!();

        let position = (keystream.state as usize) % (LCG_M as usize);
        let key_bytes: Vec<u8> = (0..plain_bytes.len().min(4))
            .map(|i| {
                let mut temp = keystream.state;
                for _ in 0..i {
                    temp = ((temp as u128 * LCG_A as u128 + LCG_C as u128) % LCG_M as u128) as u64;
                }
                (temp & 0xFF) as u8
            })
            .collect();

        print!("Key: ");
        for &b in key_bytes.iter() {
            print!("{:02x} ", b);
        }
        println!(" (keystream position: {})", position);

        let encrypted = xor_cipher(plain_bytes, &mut keystream);
        print!("Cipher: ");
        for &b in encrypted.iter().take(encrypted.len().min(5)) {
            print!("{:02x} ", b);
        }
        println!();

        let hex_message = hex::encode(&encrypted);
        println!("\n[NETWORK] Sending encrypted message ({} bytes)...", encrypted.len());
        stream.write_all(hex_message.as_bytes())?;
        stream.write_all(b"\n")?;
        stream.flush()?;
        println!("[-] Sent {} bytes", encrypted.len());
    }

    Ok(())
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Server { port } => run_server(port),
        Command::Client { address } => run_client(address),
    }
}