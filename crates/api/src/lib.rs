mod artifact;
mod door;
mod ground;
mod model;
mod rig;
pub mod runtime;
mod seam;
mod startup;

pub use startup::{Estate, Raised, Refusal, Seat, observation, router, shell};

pub(crate) fn halt(seat: &str, note: &str) -> ! {
    eprintln!("codehull: {seat}: {note}");
    std::process::exit(1)
}
