use clap::Parser;
use std::{
    fs::OpenOptions,
    io::{self, Read, Seek, SeekFrom, Write},
    path::PathBuf,
};

/// Read and write binary files in hexadecimal
#[derive(Parser, Debug)]
#[command(name = "hextool", about, long_about = None, disable_version_flag = true)]
struct Args {
    /// Target file
    #[arg(short = 'f', long = "file")]
    target_file: PathBuf,

    /// Read mode (display hex)
    #[arg(short = 'r', long = "read", group = "mode")]
    read_mode: bool,

    /// Write mode (hex string to write)
    #[arg(short = 'w', long = "write", group = "mode")]
    write_hex_string: Option<String>,

    /// Offset in bytes (decimal or 0x hex)
    #[arg(short = 'o', long = "offset", default_value = "0", value_parser = parse_offset)]
    offset: u64,

    /// Number of bytes to read
    #[arg(short = 's', long = "size", default_value = "16")]
    size: usize,
}

fn parse_offset(s: &str) -> Result<u64, String> {
    if let Some(stripped) = s.strip_prefix("0x") {
        u64::from_str_radix(stripped, 16).map_err(|e| format!("Offset hex invalide: {}", e))
    } else {
        s.parse::<u64>()
            .map_err(|e| format!("Offset décimal invalide: {}", e))
    }
}

fn format_ascii(b: u8) -> char {
    if (0x20..=0x7E).contains(&b) {
        b as char
    } else {
        '.'
    }
}

fn print_hex_dump(buffer: &[u8], base_offset: u64) {
    let mut offset = base_offset;
    for chunk in buffer.chunks(16) {
        let hex_part: String = chunk
            .iter()
            .enumerate()
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
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&args.target_file)
        .map_err(|e| format!("Erreur I/O: {}", e))?;

    file.seek(SeekFrom::Start(args.offset))
        .map_err(|e| format!("Erreur I/O seek: {}", e))?;
    file.write_all(&bytes_to_write)
        .map_err(|e| format!("Erreur I/O write: {}", e))?;
    file.flush()
        .map_err(|e| format!("Erreur I/O flush: {}", e))?;

    println!(
        "Writing {} bytes at offset 0x{:08x}",
        write_len, args.offset
    );

    // Afficher le hex formaté
    let hex_formatted: Vec<String> = bytes_to_write
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect();
    println!("Hex: {}", hex_formatted.join(" "));

    // Afficher l'ASCII
    let ascii: String = bytes_to_write.iter().map(|&b| format_ascii(b)).collect();
    println!("ASCII: {}", ascii);

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
        Err(String::from(
            "Erreur: Vous devez spécifier le mode --read (-r) ou --write (-w).",
        ))
    }
}
