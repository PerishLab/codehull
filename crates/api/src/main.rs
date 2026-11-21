use api::{Seat, observation};
use clap::{Parser, Subcommand};
use std::process::exit;

#[derive(Parser)]
#[command(name = "codehull-api", version = plumb::version!("CODEHULL"), after_help = "Only finite bootstrap CLI observation can be enabled in the platform codehull home config.toml [locus] enabled = true. It defaults off. For HTTPS, set endpoint and an absolute writer key_file together in [locus]. The durable queue is state/locus.https/codehull under that home. On Unix, sender may name the trusted absolute locus executable. HTTP uses state/locus and defaults to http://127.0.0.1:43308. With no local transport fields, LOCUS_API and LOCUS_API_KEY_FILE can supply the endpoint and writer key path. Serve initializes no observer. Process exit and abort leave incomplete Source records; no return is invented.")]
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
        Some(Command::Bootstrap { root, artifacts }) => {
            let observer = observation::start("api.bootstrap");
            Seat(&root).bootstrap(&artifacts).await;
            if let Some(observer) = observer {
                observer.finish(0);
            }
        }
        Some(Command::Serve { root }) => Seat(&root).serve().await,
        None => Seat(&cli.root).serve().await,
    }
}
