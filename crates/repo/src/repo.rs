use crate::git;
use crate::store::{sync, write};
use crate::{Error, Object, io};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const MARKER: &str = "codehull.repository";
static NEXT: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug)]
pub struct Repository {
    id: u64,
    root: PathBuf,
}

impl Repository {
    pub(crate) fn create(parent: &Path, id: u64) -> Result<Self, Error> {
        let target = parent.join(format!("{id}.git"));
        if target.exists() {
            return Self::open(&target, id);
        }
        let draft = parent.join(format!(
            ".create-{id}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&draft).map_err(|error| io("cannot reserve repository draft", error))?;
        let result = initialize(&draft, id).and_then(|()| {
            fs::rename(&draft, &target).map_err(|error| io("cannot publish repository", error))
        });
        if let Err(error) = result {
            let _ = fs::remove_dir_all(&draft);
            if target.exists() {
                return Self::open(&target, id);
            }
            return Err(error);
        }
        sync(parent)?;
        Self::open(&target, id)
    }

    pub(crate) fn open(root: &Path, id: u64) -> Result<Self, Error> {
        let expected = marker(id);
        let held = fs::read(root.join(MARKER))
            .map_err(|error| Error::Foreign(format!("repository is not owned: {error}")))?;
        if held != expected {
            return Err(Error::Foreign("repository marker disagrees".into()));
        }
        let bare = git::text(
            root,
            ["rev-parse", "--is-bare-repository"],
            "cannot inspect repository",
        )?;
        if bare != "true" {
            return Err(Error::Foreign("repository is not bare".into()));
        }
        Ok(Self {
            id,
            root: root.to_path_buf(),
        })
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn ingest(&self, bundle: &[u8], object: &Object) -> Result<(), Error> {
        let source = self.root.join(format!(
            ".ingest-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        write(source.clone(), bundle, "cannot create object ingress")?;
        let result = self.fetch(&source, object);
        let retired = fs::remove_file(&source)
            .map_err(|error| io("cannot retire object ingress", error))
            .and_then(|()| sync(&self.root));
        result.and(retired)
    }

    fn fetch(&self, source: &Path, object: &Object) -> Result<(), Error> {
        let args = [
            OsString::from("fetch"),
            OsString::from("--no-tags"),
            source.as_os_str().to_os_string(),
            OsString::from(object.hex()),
        ];
        git::success(&self.root, args, "cannot ingest object")?;
        self.require(object)
    }

    pub fn reference(&self, name: &str) -> Result<Option<Object>, Error> {
        valid(&self.root, name)?;
        let output = git::run(&self.root, ["show-ref", "--verify", "--hash", name])?;
        match output.status.code() {
            Some(0) => Object::parse(String::from_utf8_lossy(&output.stdout).trim()).map(Some),
            Some(1) => Ok(None),
            _ => Err(Error::Git(format!(
                "cannot read reference: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ))),
        }
    }

    pub fn advance(
        &self,
        name: &str,
        before: Option<&Object>,
        after: &Object,
    ) -> Result<Object, Error> {
        valid(&self.root, name)?;
        self.require(after)?;
        let old = match before {
            Some(object) => object.hex().to_string(),
            None => self.zero()?,
        };
        let output = git::run(
            &self.root,
            ["update-ref", "--create-reflog", name, after.hex(), &old],
        )?;
        if !output.status.success() {
            return Err(Error::Conflict(format!(
                "reference compare-and-swap refused: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        self.reference(name)?
            .ok_or_else(|| Error::Git("accepted reference is absent".into()))
    }

    fn require(&self, object: &Object) -> Result<(), Error> {
        git::success(
            &self.root,
            ["cat-file", "-e", &format!("{}^{{commit}}", object.hex())],
            "commit object is absent",
        )
    }

    fn zero(&self) -> Result<String, Error> {
        let format = git::text(
            &self.root,
            ["rev-parse", "--show-object-format"],
            "cannot read object format",
        )?;
        match format.as_str() {
            "sha1" => Ok("0".repeat(40)),
            "sha256" => Ok("0".repeat(64)),
            _ => Err(Error::Git(format!("unknown object format: {format}"))),
        }
    }
}

fn initialize(root: &Path, id: u64) -> Result<(), Error> {
    git::success(
        root,
        ["init", "--bare", "--initial-branch=main", "."],
        "cannot initialize repository",
    )?;
    write(
        root.join(MARKER),
        &marker(id),
        "cannot create repository marker",
    )?;
    sync(root)
}

fn marker(id: u64) -> Vec<u8> {
    format!("codehull.repository/v1\nid={id}\n").into_bytes()
}

fn valid(root: &Path, name: &str) -> Result<(), Error> {
    if !name.starts_with("refs/heads/") {
        return Err(Error::Invalid("reference must be below refs/heads".into()));
    }
    git::success(root, ["check-ref-format", name], "invalid reference")
}
