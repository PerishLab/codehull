mod git;
mod object;
mod repo;
mod store;

pub use object::Object;
pub use repo::{REFS, RESERVED, Repository, admitted};
pub use store::Store;

use std::fmt::{Display, Formatter};

#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    Conflict(String),
    Foreign(String),
    Git(String),
    Invalid(String),
    Io(String),
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Conflict(note)
            | Self::Foreign(note)
            | Self::Git(note)
            | Self::Invalid(note)
            | Self::Io(note) => formatter.write_str(note),
        }
    }
}

impl std::error::Error for Error {}

pub(crate) fn io(action: &str, error: std::io::Error) -> Error {
    Error::Io(format!("{action}: {error}"))
}
