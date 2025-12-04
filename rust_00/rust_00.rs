use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "hello")]
struct Args {
    /// Name to greet
    #[arg(default_value = "World")]
    name: String,

    /// Convert to uppercase
    #[arg(short, long)]
    upper: bool,

    /// Repeat Greating N times
    #[arg(short, long, default_value_t = 1)]
    repeat: u8,
}

fn main() {
    let args = Args::parse();

    let mut message = format!("Hello, {}!", args.name);
    if args.upper {
        message = message.to_uppercase();
    }

    for _ in 0..args.repeat {
        println!("{}", message);
    }
}
