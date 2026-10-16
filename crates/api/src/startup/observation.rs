use plumb::observation::{Declaration, Run};

pub(crate) fn start(command: &str) -> Option<Run> {
    let home = plumb::config::data("codehull")?;
    Run::start(
        Declaration {
            producer: "codehull",
            file: &home.join("config.toml"),
            buffer: home.join("state/locus"),
        },
        command,
    )
}
