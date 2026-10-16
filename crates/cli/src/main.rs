mod act;
mod observation;
mod process;

use clap::{Parser, Subcommand};
use std::process::exit;

#[derive(Parser)]
#[command(
    name = "codehull",
    version = plumb::version!("CODEHULL"),
    about = "the codehull operator client",
    arg_required_else_help = true,
    after_help = "Observation is off by default. Set [locus] enabled = true in config.toml under ~/.codehull on Unix or LOCALAPPDATA/codehull on Windows. Buffer is state/locus; endpoint overrides LOCUS_API, then http://127.0.0.1:43308. Child arguments, environment, output and CLI exit status are preserved."
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
    let observer = observation::start("act");
    let result = execute(cli.command);
    if let Some(observer) = observer {
        observer.finish(if result.is_ok() { 0 } else { 1 });
    }
    if let Err(error) = result {
        eprintln!("{error}");
        exit(1);
    }
}

#[locus::trace(with = plumb::observation::view())]
fn execute(command: Command) -> Result<(), String> {
    match command {
        Command::Act => act::run(),
    }
}
