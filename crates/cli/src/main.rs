mod act;
mod process;

use clap::{Parser, Subcommand};
use std::process::exit;

#[derive(Parser)]
#[command(
    name = "codehull",
    version = plumb::version!("CODEHULL"),
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
}

fn main() {
    if let Err(error) = plumb::identity!("CODEHULL") {
        eprintln!("codehull: {error}");
        exit(1);
    }
    let cli = Cli::parse();
    if let Err(error) = plumb::identity::ready() {
        eprintln!("codehull: {error}");
        exit(1);
    }
    let result = match cli.command {
        Command::Act => act::run(),
    };
    if let Err(error) = result {
        eprintln!("{error}");
        exit(1);
    }
}
