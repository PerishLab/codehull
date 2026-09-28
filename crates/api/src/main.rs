mod artifact;
mod door;
mod ground;
mod model;
mod rig;
mod runtime;
mod seam;
mod startup;
mod warden;

use clap::{Parser, Subcommand};
use startup::Seat;
use std::process::exit;

#[derive(Parser)]
#[command(name = "codehull-api", version = plumb::version!("CODEHULL"))]
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
    if let Err(error) = plumb::identity!("CODEHULL") {
        eprintln!("codehull-api: {error}");
        exit(1);
    }
    let cli = Cli::parse();
    if let Err(error) = plumb::identity::ready() {
        eprintln!("codehull-api: {error}");
        exit(1);
    }
    let _stamp = cli.stamp;
    match cli.command {
        Some(Command::Bootstrap { root, artifacts }) => Seat(&root).bootstrap(&artifacts).await,
        Some(Command::Serve { root }) => Seat(&root).serve().await,
        None => Seat(&cli.root).serve().await,
    }
}

pub(crate) fn halt(seat: &str, note: &str) -> ! {
    eprintln!("codehull: {seat}: {note}");
    exit(1)
}
