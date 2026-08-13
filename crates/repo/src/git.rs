use crate::{Error, io};
use std::ffi::OsStr;
use std::path::Path;
use std::process::{Command, Output};

pub fn run<I, S>(root: &Path, args: I) -> Result<Output, Error>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|error| io("cannot run git", error))
}

pub fn text<I, S>(root: &Path, args: I, action: &str) -> Result<String, Error>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = run(root, args)?;
    if !output.status.success() {
        return Err(Error::Git(format!(
            "{action}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn success<I, S>(root: &Path, args: I, action: &str) -> Result<(), Error>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    text(root, args, action).map(drop)
}
