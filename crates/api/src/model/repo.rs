use crate::model::*;
use keel::atom::{int, string};
use keel::resource;

#[resource]
pub(crate) struct Repo {
    #[field(string, unique = owner)]
    name: string,
    #[field(string)]
    visibility: string,
    #[field(string)]
    trunk: string,
    #[field(bool)]
    archived: bool,
    #[relation(Actor, many2one, root)]
    owner: Actor,
    #[relation(Repo, many2one, opt)]
    fork: Repo,
    #[relation(Topic, many2many)]
    topics: Topic,
}

#[resource]
pub(crate) struct Weld {
    #[field(string, unique = repo)]
    mode: string,
    #[relation(Repo, many2one, root)]
    repo: Repo,
}

#[resource]
pub(crate) struct Ref {
    #[field(string, unique = repo)]
    name: string,
    #[field(string)]
    object: string,
    #[relation(Repo, many2one, root)]
    repo: Repo,
}

#[resource]
pub(crate) struct Label {
    #[field(string, unique = org)]
    name: string,
    #[field(string)]
    color: string,
    #[relation(Actor, many2one, root)]
    org: Actor,
}

#[resource]
pub(crate) struct Runner {
    #[field(string)]
    name: string,
    #[field(string, unique)]
    token: string,
    #[field(string)]
    labels: string,
    #[field(string)]
    status: string,
    #[relation(Actor, many2one, root)]
    org: Actor,
}

#[resource]
pub(crate) struct Secret {
    #[field(string, unique = org)]
    name: string,
    #[field(string)]
    data: string,
    #[relation(Actor, many2one, root)]
    org: Actor,
}

#[resource]
pub(crate) struct Project {
    #[field(string)]
    title: string,
    #[field(bool)]
    closed: bool,
    #[relation(Repo, many2one, root)]
    repo: Repo,
}

#[resource]
pub(crate) struct Column {
    #[field(string)]
    title: string,
    #[field(int)]
    sort: int,
    #[relation(Project, many2one, root)]
    project: Project,
    #[relation(Issue, many2many, spot = int)]
    cards: Issue,
}
