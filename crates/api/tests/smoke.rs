#[path = "acceptance/git.rs"]
mod git;
#[path = "acceptance/issuer.rs"]
mod issuer;
#[path = "smoke/restart.rs"]
mod restart;
#[path = "smoke/stage.rs"]
mod stage;
#[path = "acceptance/wire.rs"]
mod wire;

use stage::Stage;
use std::path::Path;
use wire::Wire;

fn exe() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_codehull-api"))
}

#[test]
fn health() {
    let stage = Stage::live(exe());
    let wire = Wire::new(&stage.base);
    assert_eq!(
        wire.get("/health", &Vec::new()).status,
        200,
        "served health"
    );
}
