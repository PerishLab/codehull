use crate::git::Git;
use crate::repo::require;
use crate::{Error, Object, Repository};
use std::path::Path;

impl Repository {
    pub fn reference(&self, name: &str) -> Result<Option<Object>, Error> {
        valid(&self.root, name)?;
        Ok(self
            .references()?
            .into_iter()
            .find(|(held, _)| held == name)
            .map(|(_, object)| object))
    }

    pub fn references(&self) -> Result<Vec<(String, Object)>, Error> {
        let output = Git::at(&self.root).run(["show-ref"])?;
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
        require(&Git::at(&self.root), object)?;
        self.point(&[(name.to_string(), object.clone())], &[])
    }

    pub fn retire(&self, name: &str) -> Result<(), Error> {
        valid(&self.root, name)?;
        match self.reference(name)? {
            None => Ok(()),
            Some(_) => self.point(&[], &[name.to_string()]),
        }
    }

    pub fn point(&self, moves: &[(String, Object)], gone: &[String]) -> Result<(), Error> {
        let mut held = Vec::new();
        for name in gone {
            admit(name)?;
            held.extend_from_slice(format!("delete {name}\0\0").as_bytes());
        }
        for (name, object) in moves {
            admit(name)?;
            held.extend_from_slice(format!("update {name}\0{}\0\0", object.hex()).as_bytes());
        }
        if held.is_empty() {
            return Ok(());
        }
        Git::at(&self.root)
            .feed(
                ["update-ref", "--create-reflog", "-z", "--stdin"],
                &held,
                "cannot write references",
            )
            .map(drop)
    }
}

pub const REFS: &str = "refs/";
pub const RESERVED: [&str; 0] = [];

pub fn admitted(name: &str) -> bool {
    if !name.starts_with(REFS) || name.len() == REFS.len() {
        return false;
    }
    !RESERVED.iter().any(|held| name.starts_with(held))
}

pub(crate) fn valid(root: &Path, name: &str) -> Result<(), Error> {
    admit(name)?;
    Git::at(root).success(["check-ref-format", name], "invalid reference")
}

pub(crate) fn admit(name: &str) -> Result<(), Error> {
    if admitted(name) {
        return Ok(());
    }
    Err(Error::Invalid(
        "reference must be below refs and outside the reserved namespaces".into(),
    ))
}
