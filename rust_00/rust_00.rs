use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "hello")]
struct Args {
    /// Name to greet
    #[arg(default_value = "World")]
    name: String,

    /// Convert to uppercase
    #[arg(long)]
    upper: bool,

    /// Repeat Greating N times
    #[arg(long, default_value_t = 1)]
    repeat: u8,
}

fn main() {
    let args = Args::parse();

    // Prépare le message
    let mut message = format!("Hello, {}!", args.name);
    if args.upper {
        message = message.to_uppercase();
    }

    // Affiche le message plusieurs fois
    for _ in 0..args.repeat {
        println!("{}", message);
    }
}
