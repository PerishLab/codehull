use crate::model::*;
use keel::atom::{int, string};
use keel::resource;

#[resource]
pub(crate) struct Issue {
    #[field(serial, scope = repo)]
    index: int,
    #[field(string)]
    title: string,
    #[field(string)]
    body: string,
    #[field(bool)]
    closed: bool,
    #[relation(Repo, many2one, root)]
    repo: Repo,
    #[relation(Actor, many2one)]
    author: Actor,
    #[relation(Milestone, many2one, opt)]
    milestone: Milestone,
    #[relation(Actor, many2many)]
    assignees: Actor,
    #[relation(Label, many2many)]
    labels: Label,
    #[relation(Issue, many2many)]
    blocks: Issue,
}

#[resource]
pub(crate) struct Comment {
    #[field(string)]
    body: string,
    #[relation(Issue, many2one, root)]
    issue: Issue,
    #[relation(Actor, many2one)]
    author: Actor,
}

#[resource]
pub(crate) struct Reaction {
    #[field(string, unique = (actor, issue))]
    emoji: string,
    #[relation(Issue, many2one, root)]
    issue: Issue,
    #[relation(Actor, many2one)]
    actor: Actor,
}

#[resource]
pub(crate) struct Pull {
    #[field(string)]
    base: string,
    #[field(string)]
    head: string,
    #[field(bool)]
    merged: bool,
    #[relation(Issue, one2one, root)]
    issue: Issue,
    #[relation(Repo, many2one, opt)]
    source: Repo,
}

#[resource]
pub(crate) struct Review {
    #[field(string)]
    state: string,
    #[field(string)]
    body: string,
    #[relation(Pull, many2one, root)]
    pull: Pull,
    #[relation(Actor, many2one)]
    reviewer: Actor,
}

#[resource]
pub(crate) struct Note {
    #[field(string)]
    path: string,
    #[field(int)]
    line: int,
    #[field(string)]
    body: string,
    #[relation(Review, many2one, root)]
    review: Review,
}
