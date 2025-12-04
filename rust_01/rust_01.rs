use clap::Parser;
use std::collections::HashMap;
use std::io::{self, Read};

/// Count word frequency in text
#[derive(Parser, Debug)]
#[command(name = "wordfreq", author, about, long_about = None, disable_version_flag = true)]
struct Args {
    /// Text to analyze (or use stdin)
    text: Option<String>,

    /// Show top N words [default: 10]
    #[arg(short = 'n', long, default_value_t = 10)]
    top: usize,

    /// Ignore words shorter than N [default: 1]
    #[arg(short = 'm', long, default_value_t = 1)]
    min_length: usize,

    /// Case insensitive counting
    #[arg(short = 'i', long, default_value_t = true)]
    ignore_case: bool,
}

fn main() {
    let args = Args::parse();

    let mut input = String::new();
    if let Some(ref text) = args.text {
        input = text.clone();
    } else {
        io::stdin()
            .read_to_string(&mut input)
            .expect("Failed to read from stdin");
    }

    let words = input
        .split_whitespace()
        .map(|w| {
            let mut word = w.trim_matches(|c: char| !c.is_alphanumeric() && c != '\'' && c != '"').to_string();
            if args.ignore_case {
                word = word.to_lowercase();
            }
            word
        })
        .filter(|w| w.len() >= args.min_length && !w.is_empty());

    let mut freq: HashMap<String, usize> = HashMap::new();
    for word in words {
        *freq.entry(word).or_insert(0) += 1;
    }

    let mut sorted: Vec<_> = freq.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));

    for (word, count) in sorted.into_iter().take(args.top) {
        println!("{}: {}", word, count);
    }
}
