use crate::pipe::{Brief, CAPTURE, TRANSFER, runner};
use crate::{Error, io};
use radicula_process::Cancel;
use radicula_process::asynchronous::{Cause, Error as Process, Limits, State, Streams};
use std::ffi::OsStr;
use std::io::Cursor;
use std::path::Path;
use std::process::{Command, Output};
use tokio::io::{AsyncRead, AsyncWrite, Sink};

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

    pub async fn pour<I, S>(
        &self,
        args: I,
        input: impl AsyncRead + Send + Unpin + 'static,
        cancel: Cancel,
        action: &str,
    ) -> Result<(), (Error, bool)>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let streams = Streams::<_, Sink, Vec<u8>> {
            input: Some(input),
            stdout: None,
            stderr: Some(Vec::new()),
        };
        let runner = runner(CAPTURE).map_err(|error| (error, true))?;
        let mut session = runner.start(self.command(args), streams, cancel);
        let output = session.wait().await.map_err(|error| {
            let settled = session.observer().state() == State::Settled;
            (fault(action, error), settled)
        })?;
        status(
            output.status.success(),
            action,
            &output.streams.stderr.unwrap_or_default(),
        )
        .map_err(|error| (error, true))
    }

    pub async fn draw<I, S>(
        &self,
        args: I,
        input: Vec<u8>,
        sink: impl AsyncWrite + Send + Unpin + 'static,
        request: Draw<'_>,
    ) -> Result<(), Error>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let streams = Streams::new(Cursor::new(input), sink, Brief::default());
        let output = runner(request.limits)?
            .start(self.command(args), streams, request.cancel)
            .wait()
            .await
            .map_err(|error| fault(request.action, error))?;
        let stderr = output.streams.stderr.unwrap_or_default();
        status(output.status.success(), request.action, stderr.bytes())
    }

    pub fn feed<I, S>(&self, args: I, input: Vec<u8>, action: &str) -> Result<Vec<u8>, Error>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let command = self.command(args);
        let runner = runner(CAPTURE)?;
        let capture = move || runner.blocking(command, input, Cancel::new());
        let output = if tokio::runtime::Handle::try_current().is_ok() {
            std::thread::Builder::new()
                .spawn(capture)
                .map_err(|error| io("cannot start git worker", error))?
                .join()
                .map_err(|_| Error::Io("git worker failed".into()))?
        } else {
            capture()
        }
        .map_err(|error| fault(action, error))?;
        status(output.status.success(), action, &output.stderr)?;
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

pub struct Draw<'a> {
    pub action: &'a str,
    pub limits: Limits,
    pub cancel: Cancel,
}

impl<'a> Draw<'a> {
    pub fn upload(action: &'a str) -> Self {
        Self {
            action,
            limits: TRANSFER,
            cancel: Cancel::new(),
        }
    }
}

fn status(success: bool, action: &str, stderr: &[u8]) -> Result<(), Error> {
    if success {
        Ok(())
    } else {
        Err(refused(action, stderr))
    }
}

fn fault<T>(action: &str, error: Process<T>) -> Error {
    let Process::Operation(held) = error else {
        return Error::Io(format!("{action}: {error}"));
    };
    let note = match held.cause {
        Cause::Io(error) => error.to_string(),
        Cause::Time => "git exceeded its time limit".into(),
        Cause::Output => "git exceeded its output limit".into(),
        Cause::Cancelled => "git was cancelled".into(),
    };
    let note = match held.cleanup {
        Some(error) => format!("{note}, git cleanup is uncertain: {error}"),
        None => note,
    };
    Error::Io(format!("{action}: {note}"))
}

#[cfg(all(test, target_os = "linux"))]
mod tests;
