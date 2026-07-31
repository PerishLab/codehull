mod artifact;
mod door;
mod model;
mod rig;
mod runtime;
mod seam;
mod startup;
mod warden;

use clap::{Parser, Subcommand};
use startup::Seat;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
    #[arg(default_value = ".")]
    root: String,
    #[arg(long = "sidecar-stamp", hide = true, global = true)]
    stamp: Option<String>,
}

#[derive(Subcommand)]
enum Command {
    Bootstrap {
        #[arg(default_value = ".")]
        root: String,
        #[arg(long = "artifact", value_name = "NAME=DEST")]
        artifacts: Vec<String>,
    },
    Serve {
        #[arg(default_value = ".")]
        root: String,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let _stamp = cli.stamp;
    match cli.command {
        Some(Command::Bootstrap { root, artifacts }) => Seat(&root).bootstrap(&artifacts).await,
        Some(Command::Serve { root }) => Seat(&root).serve().await,
        None => Seat(&cli.root).serve().await,
    }
}

pub(crate) fn halt(seat: &str, note: &str) -> ! {
    eprintln!("codehull: {seat}: {note}");
    std::process::exit(1)
}
