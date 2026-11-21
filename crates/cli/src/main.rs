mod observation;

use clap::error::ErrorKind;
use clap::{CommandFactory, Parser};
use std::process::exit;

#[derive(Parser)]
#[command(
    name = "codehull",
    version = plumb::version!("CODEHULL"),
    about = "the codehull operator client",
    after_help = "Observation is off by default. Set [locus] enabled = true in config.toml under ~/.codehull on Unix or LOCALAPPDATA/codehull on Windows. For HTTPS, set endpoint and an absolute writer key_file together in [locus]. The durable queue is state/locus.https/codehull under that home. On Unix, sender may name the trusted absolute locus executable. HTTP uses state/locus and defaults to http://127.0.0.1:43308. With no local transport fields, LOCUS_API and LOCUS_API_KEY_FILE can supply the endpoint and writer key path. Help and version are never observed; a usage error finishes with its exit status and the usage fault."
)]
struct Cli {}

fn main() {
    if let Err(error) = plumb::identity!("CODEHULL") {
        eprintln!("codehull: {error}");
        exit(1);
    }
    let parsed = Cli::try_parse();
    if let Err(error) = &parsed
        && matches!(
            error.kind(),
            ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
        )
    {
        error.exit();
    }
    if let Err(error) = plumb::identity::ready() {
        eprintln!("codehull: {error}");
        exit(1);
    }
    let observer = observation::start("codehull");
    let code = settle(parsed);
    if let Some(observer) = observer {
        observer.finish(code);
    }
    exit(code);
}

#[locus::trace(with = plumb::observation::view())]
fn settle(parsed: Result<Cli, clap::Error>) -> i32 {
    match parsed {
        Ok(_) => match Cli::command().print_help() {
            Ok(()) => 0,
            Err(_) => 1,
        },
        Err(error) => {
            plumb::observation::fault("usage");
            let _ = error.print();
            error.exit_code()
        }
    }
}
