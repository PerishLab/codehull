mod act;
mod process;
mod ship;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    version,
    about = "the codehull operator client",
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Act,
    Ship,
}

fn main() {
    let result = match Cli::parse().command {
        Command::Act => act::run(),
        Command::Ship => ship::run(),
    };
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
