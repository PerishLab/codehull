use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const BLOCKED: [&str; 5] = [
    "DYLD_FALLBACK_LIBRARY_PATH",
    "DYLD_INSERT_LIBRARIES",
    "DYLD_LIBRARY_PATH",
    "LD_PRELOAD",
    "LD_LIBRARY_PATH",
];

pub struct Process<'a> {
    name: &'a str,
}

impl<'a> Process<'a> {
    pub fn new(name: &'a str) -> Self {
        Self { name }
    }

    pub fn output<I, S>(&self, args: I, cwd: Option<&Path>) -> Result<String, String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let result = self
            .build(cwd)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .output()
            .map_err(|error| format!("{}: {error}", self.name))?;
        if !result.status.success() {
            return Err(format!("{}: exited with {}", self.name, result.status));
        }
        Ok(String::from_utf8_lossy(&result.stdout)
            .trim_end()
            .to_owned())
    }

    pub fn run<I, S>(&self, args: I, cwd: Option<&Path>) -> Result<(), String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let result = self
            .build(cwd)
            .args(args)
            .status()
            .map_err(|error| format!("{}: {error}", self.name))?;
        if result.success() {
            Ok(())
        } else {
            Err(format!("{}: exited with {result}", self.name))
        }
    }

    fn build(&self, cwd: Option<&Path>) -> Command {
        let mut command = Command::new(self.name);
        command.current_dir(cwd.unwrap_or_else(|| Path::new(".")));
        for key in BLOCKED {
            command.env_remove(key);
        }
        command
    }
}

pub fn root() -> Result<PathBuf, String> {
    let value = Process::new("git").output(["rev-parse", "--show-toplevel"], None)?;
    Ok(PathBuf::from(value))
}
