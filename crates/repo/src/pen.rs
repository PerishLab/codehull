use crate::store::sync;
use crate::{Error, io};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const MARK: &str = ".pen-";
const OBJECTS: &str = "objects";
const INDEX: &str = "idx";
static NEXT: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug)]
pub struct Pen {
    root: PathBuf,
    hold: PathBuf,
}

impl Pen {
    pub(crate) fn make(root: &Path) -> Result<Self, Error> {
        let hold = root.join(format!(
            "{MARK}{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&hold).map_err(|error| io("cannot reserve an object pen", error))?;
        Ok(Self {
            root: root.to_path_buf(),
            hold,
        })
    }

    pub(crate) fn open(root: &Path, hold: &Path) -> Result<Self, Error> {
        let named = hold.file_name().and_then(OsStr::to_str).unwrap_or_default();
        if hold.parent() != Some(root) || !named.starts_with(MARK) {
            return Err(Error::Foreign("object pen belongs elsewhere".into()));
        }
        if !hold.is_dir() {
            return Err(Error::Foreign("object pen is absent".into()));
        }
        Ok(Self {
            root: root.to_path_buf(),
            hold: hold.to_path_buf(),
        })
    }

    pub fn hold(&self) -> &Path {
        &self.hold
    }

    pub fn keep(self) -> Result<(), Error> {
        self.pour(false)?;
        self.pour(true)?;
        sync(&self.root.join(OBJECTS))?;
        self.wipe()
    }

    pub fn wipe(self) -> Result<(), Error> {
        fs::remove_dir_all(&self.hold).map_err(|error| io("cannot retire an object pen", error))
    }

    fn pour(&self, last: bool) -> Result<(), Error> {
        for spot in listed(&self.hold)? {
            self.shift(&spot, last)?;
        }
        Ok(())
    }

    fn shift(&self, spot: &Path, last: bool) -> Result<(), Error> {
        let Some(name) = spot.file_name().filter(|_| spot.is_dir()) else {
            return Ok(());
        };
        let into = self.root.join(OBJECTS).join(name);
        fs::create_dir_all(&into).map_err(|error| io("cannot open an object seat", error))?;
        for held in listed(spot)? {
            plant(&held, &into, last)?;
        }
        sync(&into)
    }
}

fn plant(held: &Path, into: &Path, last: bool) -> Result<(), Error> {
    let indexed = held.extension().and_then(OsStr::to_str) == Some(INDEX);
    let Some(name) = held.file_name().filter(|_| indexed == last) else {
        return Ok(());
    };
    fs::rename(held, into.join(name)).map_err(|error| io("cannot admit an object", error))
}

fn listed(spot: &Path) -> Result<Vec<PathBuf>, Error> {
    let mut held = Vec::new();
    for entry in fs::read_dir(spot).map_err(|error| io("cannot read an object pen", error))? {
        let seen = entry.map_err(|error| io("cannot read an object pen", error))?;
        held.push(seen.path());
    }
    Ok(held)
}
