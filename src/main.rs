use clap::Parser;

/// A simple file inspection tool
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// File to open
    file: String,
}

fn main() {
    let cli = Cli::parse();
    println!("Opening file: {}", cli.file);
}
