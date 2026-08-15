use crate::model::*;
use keel::atom::{int, string};
use keel::resource;

#[resource]
pub(crate) struct Release {
    #[field(string, unique = repo)]
    tag: string,
    #[field(string)]
    title: string,
    #[field(string)]
    body: string,
    #[field(bool)]
    draft: bool,
    #[relation(Repo, many2one, root)]
    repo: Repo,
    #[relation(Actor, many2one)]
    author: Actor,
}

#[resource]
pub(crate) struct Shield {
    #[field(string, unique = repo)]
    branch: string,
    #[field(bool)]
    force: bool,
    #[field(int)]
    approvals: int,
    #[relation(Repo, many2one, root)]
    repo: Repo,
}

#[resource]
pub(crate) struct Demand {
    #[field(string, unique = shield)]
    context: string,
    #[relation(Shield, many2one, root)]
    shield: Shield,
}

#[resource]
pub(crate) struct Package {
    #[field(string, unique = repo)]
    name: string,
    #[field(string)]
    kind: string,
    #[field(string)]
    version: string,
    #[relation(Repo, many2one, root)]
    repo: Repo,
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
    #[relation(Repo, many2one, root)]
    repo: Repo,
}

#[resource]
pub(crate) struct Run {
    #[field(string)]
    event: string,
    #[field(string)]
    status: string,
    #[field(string)]
    commit: string,
    #[relation(Repo, many2one, root)]
    repo: Repo,
}

#[resource]
pub(crate) struct Secret {
    #[field(string, unique = repo)]
    name: string,
    #[field(string)]
    data: string,
    #[relation(Repo, many2one, root)]
    repo: Repo,
}

#[resource]
pub(crate) struct Variable {
    #[field(string, unique = repo)]
    name: string,
    #[field(string)]
    value: string,
    #[relation(Repo, many2one, root)]
    repo: Repo,
}

#[resource]
pub(crate) struct Key {
    #[field(string)]
    title: string,
    #[field(string, unique)]
    print: string,
    #[relation(Repo, many2one, root)]
    repo: Repo,
}

#[resource]
pub(crate) struct Milestone {
    #[field(string)]
    title: string,
    #[field(int)]
    due: int,
    #[field(bool)]
    closed: bool,
    #[relation(Repo, many2one, root)]
    repo: Repo,
}

#[resource]
pub(crate) struct Label {
    #[field(string, unique = repo)]
    name: string,
    #[field(string)]
    color: string,
    #[relation(Repo, many2one, root)]
    repo: Repo,
}
