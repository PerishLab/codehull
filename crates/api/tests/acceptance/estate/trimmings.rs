use super::super::open;
use serde_json::json;

#[test]
fn trimmings() {
    let (stage, wire) = open();
    let ada = wire.join(&stage.issuer, "ada", &[]);
    let bob = wire.join(&stage.issuer, "bob", &[]);
    let head = &bob.head;
    let up = wire.public(&ada.head, "upstream", ada.id);
    let fork = wire.made(
        "/repo",
        json!({
            "name": "fork",
            "visibility": "public",
            "owner": bob.id,
            "trunk": "main",
            "archived": false,
            "fork": up,
        }),
        head,
    );
    let forks = wire.query(&format!("from Repo where fork = \"{up}\""), head);
    assert_eq!(forks.roots().len(), 1, "fork not linked");
    let arch = wire.patch(&format!("/repo/{fork}"), json!({ "archived": true }), head);
    assert!(arch.ok(), "archive {}", arch.status);

    let mile = wire.made(
        "/milestone",
        json!({ "title": "v2", "due": 0, "closed": false, "repo": fork }),
        head,
    );
    let issue = wire.made(
        "/issue",
        json!({
            "title": "planned",
            "body": "",
            "closed": false,
            "repo": fork,
            "author": bob.id,
            "milestone": mile,
        }),
        head,
    );
    wire.want(
        &format!("/issue/{issue}/assignees"),
        json!({ "right": bob.id }),
        head,
    );
    let other = wire.made(
        "/issue",
        json!({ "title": "blocker", "body": "", "closed": false, "repo": fork, "author": bob.id }),
        head,
    );
    wire.want(
        &format!("/issue/{issue}/blocks"),
        json!({ "right": other }),
        head,
    );
    let pack = wire.query(
        &format!("from Issue where id = \"{issue}\" link assignees link blocks"),
        head,
    );
    assert_eq!(
        pack.bond("repo:issue.assignees").len(),
        1,
        "assignee missing"
    );
    assert_eq!(pack.bond("repo:issue.blocks").len(), 1, "block missing");

    wire.want(
        &format!("/actor/{}/stars", bob.id),
        json!({ "right": up }),
        head,
    );
    let stars = wire.query(&format!("from Actor where stars has \"{up}\" count"), head);
    assert_eq!(stars.count(), 1, "star not counted");
    wire.want(
        "/email",
        json!({ "mail": "bob@lab.dev", "primary": true, "actor": bob.id }),
        head,
    );
}
