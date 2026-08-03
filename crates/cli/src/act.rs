use std::ffi::OsString;

use crate::process;

pub fn run() -> Result<(), String> {
    let root = process::root()?;
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
    process::Process::new("deno").run(args, Some(&root))
}
