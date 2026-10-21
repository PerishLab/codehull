use super::super::open;
use serde_json::json;

#[test]
fn planning() {
    let (stage, wire) = open();
    let bob = wire.join(&stage.issuer, "bob", &[]);
    let cy = wire.join(&stage.issuer, "cy", &[]);
    let head = &bob.head;
    let repo = wire.private(head, "product", bob.id);
    let project = wire.made(
        "/project",
        json!({ "title": "roadmap", "closed": false, "repo": repo }),
        head,
    );
    let column = wire.made(
        "/column",
        json!({ "title": "todo", "sort": 1, "project": project }),
        head,
    );
    let issue = wire.made(
        "/issue",
        json!({ "title": "card", "body": "", "closed": false, "repo": repo, "author": bob.id }),
        head,
    );
    wire.want(
        &format!("/column/{column}/cards"),
        json!({ "right": issue, "spot": 3 }),
        head,
    );
    wire.want(
        "/release",
        json!({ "tag": "v1.0", "title": "one", "body": "notes", "draft": false, "repo": repo, "author": bob.id }),
        head,
    );
    let dup = wire.post(
        "/release",
        json!({ "tag": "v1.0", "title": "dup", "body": "", "draft": true, "repo": repo, "author": bob.id }),
        head,
    );
    assert_eq!(dup.status, 409, "release tag not scoped-unique");
    wire.want(
        "/shield",
        json!({ "branch": "main", "force": false, "approvals": 2, "repo": repo }),
        head,
    );
    wire.want(
        "/package",
        json!({ "name": "libcore", "kind": "cargo", "version": "0.1.0", "repo": repo }),
        head,
    );
    let owned = wire.query(&format!("from Release where repo = \"{repo}\""), head);
    assert_eq!(owned.roots().len(), 1, "owner cannot see own release");
    let blind = wire.query(&format!("from Package where repo = \"{repo}\""), &cy.head);
    assert!(blind.roots().is_empty(), "stranger sees private package");
}
