// src/main.rs - Version Courte

use clap::Parser;
use std::{fs::{OpenOptions}, io::{self, Read, Seek, SeekFrom, Write}, path::PathBuf};

#[derive(Parser, Debug)]
#[command(name = "hextool")]
struct Args {
    #[arg(short = 'f', long = "file")]
    target_file: PathBuf,
    #[arg(short = 'r', long = "read", group = "mode")]
    read_mode: bool,
    #[arg(short = 'w', long = "write", group = "mode")]
    write_hex_string: Option<String>,
    #[arg(short = 'o', long = "offset", default_value = "0", value_parser = parse_offset)]
    offset: u64,
    #[arg(short = 's', long = "size", default_value = "16")]
    size: usize,
}

fn parse_offset(s: &str) -> Result<u64, String> {
    if let Some(stripped) = s.strip_prefix("0x") {
        u64::from_str_radix(stripped, 16).map_err(|e| format!("Offset hex invalide: {}", e))
    } else {
        s.parse::<u64>().map_err(|e| format!("Offset décimal invalide: {}", e))
    }
}

fn format_ascii(b: u8) -> char {
    if (0x20..=0x7E).contains(&b) { b as char } else { '.' }
}

fn print_hex_dump(buffer: &[u8], base_offset: u64) {
    let mut offset = base_offset;
    for chunk in buffer.chunks(16) {
        let hex_part: String = chunk.iter().enumerate()
            .map(|(i, &b)| format!("{:02x}{}", b, if i == 7 { "  " } else { " " }))
            .collect();
        let ascii_part: String = chunk.iter().map(|&b| format_ascii(b)).collect();
        
        let padded_hex = format!("{:<49}", hex_part.trim_end());
        println!("{:08x}: {}|{}|", offset, padded_hex, ascii_part);
        offset += chunk.len() as u64;
    }
}

fn handle_read(args: &Args) -> io::Result<()> {
    let mut file = OpenOptions::new().read(true).open(&args.target_file)?;
    file.seek(SeekFrom::Start(args.offset))?;
    let mut buffer = vec![0u8; args.size];
    let bytes_read = file.read(&mut buffer)?;
    buffer.truncate(bytes_read);
    
    if bytes_read > 0 {
        print_hex_dump(&buffer, args.offset);
    } else {
        println!("Aucun octet lu à l'offset 0x{:x}.", args.offset);
    }
    Ok(())
}

fn handle_write(args: &Args, hex_string: &str) -> Result<(), String> {
    let bytes_to_write = hex::decode(hex_string)
        .map_err(|_| String::from("Erreur: Chaîne hexadécimale invalide."))?;
    
    let write_len = bytes_to_write.len();

    let mut file = OpenOptions::new()
        .read(true).write(true).create(true).open(&args.target_file)
        .map_err(|e| format!("Erreur I/O: {}", e))?;

    file.seek(SeekFrom::Start(args.offset))
        .map_err(|e| format!("Erreur I/O seek: {}", e))?;
    file.write_all(&bytes_to_write)
        .map_err(|e| format!("Erreur I/O write: {}", e))?;
    file.flush()
        .map_err(|e| format!("Erreur I/O flush: {}", e))?;

    println!("Writing {} bytes at offset 0x{:08X}", write_len, args.offset);
    println!("Hex: {}", hex_string.to_lowercase());
    println!("✓ Successfully written");

    Ok(())
}

fn main() -> Result<(), String> {
    let args = Args::parse();

    if args.read_mode {
        handle_read(&args).map_err(|e| format!("Erreur de lecture: {}", e))
    } else if let Some(ref hex_string) = args.write_hex_string {
        handle_write(&args, hex_string)
    } else {
        Err(String::from("Erreur: Vous devez spécifier le mode --read (-r) ou --write (-w)."))
    }
}