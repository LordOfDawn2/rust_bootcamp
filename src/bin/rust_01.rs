use clap::Parser;
use std::collections::HashMap;
use std::io::{self, Read};

/// Count word frequency in text
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Text to analyze (optional, reads from stdin if not provided)
    text: Option<String>,

    /// Show top N words [default: 10]
    #[arg(long, default_value_t = 10)]
    top: usize,

    /// Ignore words shorter than N [default: 1]
    #[arg(long, default_value_t = 1)]
    min_length: usize,

    /// Case insensitive counting
    #[arg(long)]
    ignore_case: bool,
}

fn main() {
    let args = Args::parse();

    // Lire le texte depuis les arguments ou stdin
    let mut input = String::new();
    if let Some(ref text) = args.text {
        input = text.clone();
    } else {
        io::stdin()
            .read_to_string(&mut input)
            .expect("Failed to read from stdin");
    }

    // Extraire les mots
    let words = input
        .split_whitespace()
        .map(|w| {
            let mut word = w
                .trim_matches(|c: char| !c.is_alphanumeric())
                .to_string();
            if args.ignore_case {
                word = word.to_lowercase();
            }
            word
        })
        .filter(|w| w.len() >= args.min_length && !w.is_empty());

    // Compter les mots
    let mut freq: HashMap<String, usize> = HashMap::new();
    for word in words {
        *freq.entry(word).or_insert(0) += 1;
    }

    // Trier par fréquence (descendante)
    let mut sorted: Vec<_> = freq.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));

    // Formatage du résultat
    if args.text.is_none() {
        println!("Top {} words:", args.top);
    } else {
        println!("Word frequency:");
    }

    for (word, count) in sorted.into_iter().take(args.top) {
        println!("{}: {}", word, format_number(count));
    }
}

fn format_number(n: usize) -> String {
    let s = n.to_string();
    let chars: Vec<char> = s.chars().rev().collect(); // <-- plus besoin de "mut"
    let mut result = String::new();

    for (i, c) in chars.iter().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(*c);
    }

    result.chars().rev().collect()
}
