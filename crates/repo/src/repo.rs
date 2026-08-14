use crate::git::Git;
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
        let bare = Git(root).text(
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

    pub fn root(&self) -> &Path {
        &self.root
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
        Git(&self.root).success(args, "cannot ingest object")?;
        self.require(object)
    }

    pub fn reference(&self, name: &str) -> Result<Option<Object>, Error> {
        valid(&self.root, name)?;
        Ok(self
            .references()?
            .into_iter()
            .find(|(held, _)| held == name)
            .map(|(_, object)| object))
    }

    pub fn references(&self) -> Result<Vec<(String, Object)>, Error> {
        let output = Git(&self.root).run(["show-ref"])?;
        if !matches!(output.status.code(), Some(0) | Some(1)) {
            return Err(Error::Git(format!(
                "cannot list references: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let mut held = Vec::new();
        for line in text.lines() {
            let Some((hex, name)) = line.split_once(' ') else {
                continue;
            };
            held.push((name.to_string(), Object::parse(hex)?));
        }
        Ok(held)
    }

    pub fn project(&self, name: &str, object: &Object) -> Result<(), Error> {
        valid(&self.root, name)?;
        self.holds(object)?;
        Git(&self.root).success(
            ["update-ref", "--create-reflog", name, object.hex()],
            "cannot project reference",
        )
    }

    pub fn retire(&self, name: &str) -> Result<(), Error> {
        valid(&self.root, name)?;
        let Some(object) = self.reference(name)? else {
            return Ok(());
        };
        Git(&self.root).success(
            ["update-ref", "--create-reflog", "-d", name, object.hex()],
            "cannot retire reference",
        )
    }

    pub fn index(&self, pack: &[u8]) -> Result<(), Error> {
        Git(&self.root)
            .feed(
                ["index-pack", "--stdin", "--fix-thin"],
                pack,
                "cannot index objects",
            )
            .map(drop)
    }

    pub fn ancestor(&self, old: &Object, new: &Object) -> Result<bool, Error> {
        let output = Git(&self.root).run(["merge-base", "--is-ancestor", old.hex(), new.hex()])?;
        match output.status.code() {
            Some(0) => Ok(true),
            Some(1) => Ok(false),
            _ => Err(Error::Git(format!(
                "cannot compare objects: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ))),
        }
    }

    pub fn weld(&self, base: &Object, head: &Object) -> Result<Object, Error> {
        let output = Git(&self.root).run(["merge-tree", "--write-tree", base.hex(), head.hex()])?;
        if !output.status.success() {
            return Err(Error::Conflict("the two sides do not merge cleanly".into()));
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let first = text.lines().next().unwrap_or_default().trim();
        Object::parse(first)
    }

    pub fn commit(&self, tree: &Object, parents: &[&Object], note: &str) -> Result<Object, Error> {
        let mut args = vec![
            "-c".to_owned(),
            "user.name=Codehull".to_owned(),
            "-c".to_owned(),
            "user.email=codehull@invalid".to_owned(),
            "commit-tree".to_owned(),
            tree.hex().to_owned(),
        ];
        for parent in parents {
            args.push("-p".to_owned());
            args.push(parent.hex().to_owned());
        }
        args.push("-m".to_owned());
        args.push(note.to_owned());
        Object::parse(&Git(&self.root).text(args, "cannot write the merge commit")?)
    }

    pub fn holds(&self, object: &Object) -> Result<(), Error> {
        self.require(object)
    }

    pub fn advertise(&self) -> Result<Vec<u8>, Error> {
        Git(&self.root).feed(
            ["upload-pack", "--stateless-rpc", "--advertise-refs", "."],
            &[],
            "cannot advertise references",
        )
    }

    pub fn upload(&self, want: &[u8]) -> Result<Vec<u8>, Error> {
        Git(&self.root).feed(
            ["upload-pack", "--stateless-rpc", "."],
            want,
            "cannot upload objects",
        )
    }

    fn require(&self, object: &Object) -> Result<(), Error> {
        Git(&self.root).success(
            ["cat-file", "-e", &format!("{}^{{commit}}", object.hex())],
            "commit object is absent",
        )
    }
}

fn initialize(root: &Path, id: u64) -> Result<(), Error> {
    Git(root).success(
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

pub const REFS: &str = "refs/";
pub const RESERVED: [&str; 0] = [];

pub fn admitted(name: &str) -> bool {
    if !name.starts_with(REFS) || name.len() == REFS.len() {
        return false;
    }
    !RESERVED.iter().any(|held| name.starts_with(held))
}

fn valid(root: &Path, name: &str) -> Result<(), Error> {
    if !admitted(name) {
        return Err(Error::Invalid(
            "reference must be below refs and outside the reserved namespaces".into(),
        ));
    }
    Git(root).success(["check-ref-format", name], "invalid reference")
}
