use plumb::config::{Cascade, Env};
use std::path::Path;

pub(crate) const NAME: &str = "codehull.toml";

#[derive(Debug, Default, PartialEq, Cascade)]
pub(crate) struct Runtime {
    pub(crate) fresh: bool,
    #[cascade(section)]
    pub(crate) listen: Listen,
    #[cascade(section)]
    pub(crate) store: Store,
    #[cascade(section)]
    pub(crate) cache: Cache,
    #[cascade(section)]
    pub(crate) blob: Blob,
    #[cascade(section)]
    pub(crate) oidc: Oidc,
}

#[derive(Debug, serde::Deserialize, PartialEq, Cascade)]
#[cascade(section)]
#[serde(default)]
pub(crate) struct Listen {
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) prefix: String,
}

impl Default for Listen {
    fn default() -> Self {
        Listen {
            host: "127.0.0.1".to_string(),
            port: 3400,
            prefix: String::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Kind {
    #[default]
    Memory,
    File,
    Pg,
}

impl Env for Kind {
    fn read(value: &str) -> Result<Self, String> {
        match value {
            "memory" => Ok(Kind::Memory),
            "file" => Ok(Kind::File),
            "pg" => Ok(Kind::Pg),
            _ => Err("neither memory, file, nor pg".to_string()),
        }
    }
}

#[derive(Debug, Default, serde::Deserialize, PartialEq, Cascade)]
#[cascade(section)]
#[serde(default)]
pub(crate) struct Store {
    pub(crate) kind: Kind,
    pub(crate) path: String,
    pub(crate) url: String,
}

#[derive(Clone, Copy, Debug, Default, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Hold {
    #[default]
    Memory,
    None,
}

impl Env for Hold {
    fn read(value: &str) -> Result<Self, String> {
        match value {
            "memory" => Ok(Hold::Memory),
            "none" => Ok(Hold::None),
            _ => Err("neither memory nor none".to_string()),
        }
    }
}

#[derive(Debug, Default, serde::Deserialize, PartialEq, Cascade)]
#[cascade(section)]
#[serde(default)]
pub(crate) struct Cache {
    pub(crate) kind: Hold,
}

#[derive(Debug, Default, serde::Deserialize, PartialEq, Cascade)]
#[cascade(section)]
#[serde(default)]
pub(crate) struct Blob {
    pub(crate) endpoint: String,
    pub(crate) key: String,
    pub(crate) secret: String,
}

#[derive(Debug, Default, serde::Deserialize, PartialEq, Cascade)]
#[cascade(section)]
#[serde(default)]
pub(crate) struct Oidc {
    pub(crate) issuer: String,
    pub(crate) audience: String,
}

pub(crate) fn load(start: &Path) -> Result<Runtime, plumb::config::Error> {
    match plumb::config::discover(start, NAME) {
        Ok(found) => Runtime::resolve(Some(&found)),
        Err(_) => Runtime::resolve(None),
    }
}
