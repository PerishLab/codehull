mod seal;

use std::fs::OpenOptions;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub(crate) enum Artifact {
    Kept(PathBuf),
    Sealed(String),
}

impl Artifact {
    pub(crate) fn load(&self) -> Result<Option<Vec<u8>>, String> {
        let path = match self {
            Artifact::Sealed(name) => return seal::load(name),
            Artifact::Kept(path) => path,
        };
        match std::fs::read(path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(None),
            Err(err) => Err(format!("cannot read artifact {}: {err}", path.display())),
        }
    }

    pub(crate) fn keep(&self, bytes: &[u8]) -> Result<bool, String> {
        let path = match self {
            Artifact::Sealed(name) => return seal::keep(name, bytes),
            Artifact::Kept(path) => path,
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|err| format!("cannot prepare {}: {err}", parent.display()))?;
        }
        let mut file = match options().open(path) {
            Ok(file) => file,
            Err(err) if err.kind() == ErrorKind::AlreadyExists => return Ok(false),
            Err(err) => return Err(format!("cannot create {}: {err}", path.display())),
        };
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|err| format!("cannot keep {}: {err}", path.display()))?;
        Ok(true)
    }
}

fn options() -> OpenOptions {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
}

pub(crate) fn bootstrap(root: &Path, specs: &[String]) -> Result<Artifact, String> {
    let mut sudo = None;
    for spec in specs {
        let (name, seat) = spec
            .split_once('=')
            .ok_or_else(|| "artifact must be NAME=DEST".to_string())?;
        if name != "sudo" {
            return Err(format!("unknown artifact: {name}"));
        }
        if sudo.is_some() {
            return Err("duplicate artifact: sudo".to_string());
        }
        sudo = Some(seat.to_string());
    }
    match sudo {
        Some(seat) => place(&seat),
        None => Ok(spare(root)),
    }
}

fn place(seat: &str) -> Result<Artifact, String> {
    if let Some(path) = held(seat, "file:") {
        return Ok(Artifact::Kept(PathBuf::from(path)));
    }
    if let Some(name) = held(seat, "kubernetes:") {
        return Ok(Artifact::Sealed(name.to_string()));
    }
    Err(format!("unsupported destination for sudo: {seat}"))
}

fn held<'a>(seat: &'a str, mark: &str) -> Option<&'a str> {
    seat.strip_prefix(mark).filter(|rest| !rest.is_empty())
}

pub(crate) fn text(bytes: &[u8]) -> Result<String, String> {
    let held = std::str::from_utf8(bytes).map_err(|_| "sudo artifact is not text".to_string())?;
    let token = held.trim();
    if token.is_empty() {
        return Err("sudo artifact is empty".to_string());
    }
    Ok(token.to_string())
}

fn spare(root: &Path) -> Artifact {
    Artifact::Kept(root.join(".local").join("sudo"))
}
