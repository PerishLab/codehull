use crate::{Error, Repository, io};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const MARKER: &str = ".codehull-store";
const MARK: &[u8] = b"codehull.store/v1\n";
static NEXT: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug)]
pub struct Store {
    root: PathBuf,
}

impl Store {
    pub fn create(root: &Path) -> Result<Self, Error> {
        if root.exists() {
            return Self::open(root);
        }
        let parent = parent(root);
        let draft = parent.join(format!(
            ".codehull-store-create-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&draft).map_err(|error| io("cannot reserve repository store", error))?;
        let result = write(draft.join(MARKER), MARK, "cannot create store marker")
            .and_then(|()| {
                fs::create_dir(draft.join("repos"))
                    .map_err(|error| io("cannot create repository seat", error))
            })
            .and_then(|()| sync(&draft))
            .and_then(|()| {
                fs::rename(&draft, root)
                    .map_err(|error| io("cannot publish repository store", error))
            });
        if let Err(error) = result {
            let _ = fs::remove_dir_all(&draft);
            if root.exists() {
                return Self::open(root);
            }
            return Err(error);
        }
        sync(parent)?;
        Self::open(root)
    }

    pub fn open(root: &Path) -> Result<Self, Error> {
        let marker = fs::read(root.join(MARKER))
            .map_err(|error| Error::Foreign(format!("repository store is not owned: {error}")))?;
        if marker != MARK {
            return Err(Error::Foreign("repository store marker disagrees".into()));
        }
        if !root.join("repos").is_dir() {
            return Err(Error::Foreign(
                "repository store has no repository seat".into(),
            ));
        }
        Ok(Self {
            root: root.to_path_buf(),
        })
    }

    pub fn provision(&self, id: u64) -> Result<Repository, Error> {
        Repository::create(&self.root.join("repos"), id)
    }

    pub fn repository(&self, id: u64) -> Result<Repository, Error> {
        Repository::open(&self.root.join("repos").join(format!("{id}.git")), id)
    }
}

fn parent(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

pub(crate) fn write(path: PathBuf, bytes: &[u8], action: &str) -> Result<(), Error> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|error| io(action, error))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| io("cannot persist ownership marker", error))
}

pub(crate) fn sync(path: &Path) -> Result<(), Error> {
    OpenOptions::new()
        .read(true)
        .open(path)
        .and_then(|file| file.sync_all())
        .map_err(|error| io("cannot persist repository directory", error))
}
