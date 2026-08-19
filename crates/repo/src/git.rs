use crate::{Error, io};
use std::ffi::OsStr;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

const OBJECTS: &str = "objects";
const SEAT: &str = "GIT_OBJECT_DIRECTORY";
const SPARE: &str = "GIT_ALTERNATE_OBJECT_DIRECTORIES";

pub struct Git<'a> {
    root: &'a Path,
    pen: Option<&'a Path>,
}

impl<'a> Git<'a> {
    pub fn at(root: &'a Path) -> Self {
        Self { root, pen: None }
    }

    pub fn pen(root: &'a Path, pen: &'a Path) -> Self {
        Self {
            root,
            pen: Some(pen),
        }
    }

    pub fn run<I, S>(&self, args: I) -> Result<Output, Error>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.command(args)
            .output()
            .map_err(|error| io("cannot run git", error))
    }

    pub fn text<I, S>(&self, args: I, action: &str) -> Result<String, Error>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let output = self.run(args)?;
        if !output.status.success() {
            return Err(refused(action, &output.stderr));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    pub fn success<I, S>(&self, args: I, action: &str) -> Result<(), Error>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.text(args, action).map(drop)
    }

    pub fn pour<I, S>(&self, args: I, src: &mut dyn Read, action: &str) -> Result<(), Error>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut child = self
            .command(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| io("cannot run git", error))?;
        let mut sink = child
            .stdin
            .take()
            .ok_or_else(|| Error::Io("git input is absent".into()))?;
        let moved = std::io::copy(src, &mut sink);
        drop(sink);
        let output = child
            .wait_with_output()
            .map_err(|error| io("cannot wait for git", error))?;
        moved.map_err(|error| io("cannot write git input", error))?;
        if !output.status.success() {
            return Err(Error::Git(format!(
                "{action}: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        Ok(())
    }

    pub fn draw<I, S>(
        &self,
        args: I,
        input: &[u8],
        sink: &mut dyn Write,
        action: &str,
    ) -> Result<(), Error>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut child = self
            .command(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| io("cannot run git", error))?;
        let mut hold = child
            .stdin
            .take()
            .ok_or_else(|| Error::Io("git input is absent".into()))?;
        hold.write_all(input)
            .map_err(|error| io("cannot write git input", error))?;
        drop(hold);
        let mut held = child
            .stdout
            .take()
            .ok_or_else(|| Error::Io("git output is absent".into()))?;
        let moved = std::io::copy(&mut held, sink);
        let output = child
            .wait_with_output()
            .map_err(|error| io("cannot wait for git", error))?;
        moved.map_err(|error| io("cannot read git output", error))?;
        if !output.status.success() {
            return Err(refused(action, &output.stderr));
        }
        Ok(())
    }

    pub fn feed<I, S>(&self, args: I, input: &[u8], action: &str) -> Result<Vec<u8>, Error>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut child = self
            .command(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| io("cannot run git", error))?;
        let mut sink = child
            .stdin
            .take()
            .ok_or_else(|| Error::Io("git input is absent".into()))?;
        sink.write_all(input)
            .map_err(|error| io("cannot write git input", error))?;
        drop(sink);
        let output = child
            .wait_with_output()
            .map_err(|error| io("cannot wait for git", error))?;
        if !output.status.success() {
            return Err(refused(action, &output.stderr));
        }
        Ok(output.stdout)
    }

    fn command<I, S>(&self, args: I) -> Command
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut held = Command::new("git");
        held.arg("-C").arg(self.root).args(args);
        if let Some(pen) = self.pen {
            held.env(SEAT, pen);
            held.env(SPARE, self.root.join(OBJECTS));
        }
        held
    }
}

fn refused(action: &str, stderr: &[u8]) -> Error {
    Error::Git(format!(
        "{action}: {}",
        String::from_utf8_lossy(stderr).trim()
    ))
}
