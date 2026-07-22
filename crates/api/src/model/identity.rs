use crate::model::*;
use keel::atom::url as link;
use keel::atom::{int, string};
use keel::resource;

#[resource]
pub(crate) struct Actor {
    #[field(string, unique)]
    login: string,
    #[field(string)]
    kind: string,
    #[field(bool)]
    barred: bool,
    #[relation(Repo, many2many)]
    stars: Repo,
    #[relation(Repo, many2many)]
    watches: Repo,
    #[relation(Actor, many2many)]
    follows: Actor,
}

#[resource]
pub(crate) struct Key {
    #[field(string)]
    title: string,
    #[field(string, unique)]
    print: string,
    #[field(string)]
    kind: string,
    #[relation(Actor, many2one, root)]
    owner: Actor,
}

#[resource]
pub(crate) struct Mirror {
    #[field(url)]
    remote: link,
    #[field(int)]
    interval: int,
    #[relation(Repo, one2one, root)]
    repo: Repo,
}

#[resource]
pub(crate) struct Email {
    #[field(string, unique)]
    mail: string,
    #[field(bool)]
    primary: bool,
    #[relation(Actor, many2one, root)]
    actor: Actor,
}

#[resource]
pub(crate) struct Team {
    #[field(string, unique = org)]
    name: string,
    #[field(string)]
    mode: string,
    #[relation(Actor, many2one, root)]
    org: Actor,
    #[relation(Actor, many2many, crew)]
    members: Actor,
    #[relation(Repo, many2many)]
    repos: Repo,
}

#[resource]
pub(crate) struct Topic {
    #[field(string, unique)]
    name: string,
}
