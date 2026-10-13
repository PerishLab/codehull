use std::ffi::OsString;

use crate::process;

#[locus::trace(with = observer.map(crate::observation::Run::view))]
pub fn run(observer: Option<&crate::observation::Run>) -> Result<(), String> {
    let root = process::root(observer)?;
    let base = root.join("crates/cli/scripts");
    let args: Vec<OsString> = [
        "run",
        "--allow-read",
        "--allow-write",
        "--allow-env",
        "--allow-net",
        "--allow-run",
        "--config",
    ]
    .into_iter()
    .map(OsString::from)
    .chain([
        base.join("deno.json").into_os_string(),
        base.join("act.ts").into_os_string(),
    ])
    .collect();
    process::Process::new("deno").run(args, Some(&root), observer)
}
