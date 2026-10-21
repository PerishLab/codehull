use super::super::open;
use serde_json::{Value, json};

fn issue(title: &str, repo: i64, author: i64) -> Value {
    json!({ "title": title, "body": "", "closed": false, "repo": repo, "author": author })
}

fn refused(status: u16) -> bool {
    status == 403 || status == 404
}

#[test]
fn search() {
    let (stage, wire) = open();
    let ada = wire.join(&stage.issuer, "ada", &[]);
    let bob = wire.join(&stage.issuer, "bob", &[]);
    let repo = wire.public(&ada.head, "site", ada.id);
    wire.want(
        "/issue",
        issue("fix the login bug", repo, bob.id),
        &bob.head,
    );
    wire.want("/issue", issue("add dark mode", repo, bob.id), &bob.head);
    let hit = wire.query(
        "from Issue where title like \"LOGIN\" and closed = \"false\"",
        &bob.head,
    );
    assert_eq!(hit.roots().len(), 1, "search expected 1");
}

#[test]
fn review() {
    let (stage, wire) = open();
    let bob = wire.join(&stage.issuer, "bob", &[]);
    let cy = wire.join(&stage.issuer, "cy", &[]);
    let repo = wire.public(&bob.head, "app", bob.id);
    let on = wire.made("/issue", issue("PR: feature", repo, bob.id), &bob.head);
    let pull = wire.made(
        "/pull",
        json!({
            "base": "refs/heads/main",
            "head": "refs/heads/feat",
            "weld": "",
            "repo": repo,
            "issue": on,
            "author": bob.id,
        }),
        &bob.head,
    );
    let verdict = json!({ "state": "approve", "body": "lgtm", "pull": pull, "reviewer": cy.id });
    let review = wire.made("/review", verdict, &cy.head);
    wire.want(
        "/note",
        json!({ "path": "src/main.rs", "line": 10, "body": "nit", "review": review }),
        &cy.head,
    );
    let mine = wire.query(
        &format!("from Review where reviewer = \"{}\"", cy.id),
        &cy.head,
    );
    assert!(!mine.roots().is_empty(), "reviewer cannot see own review");
    let path = format!("/pull/{pull}");
    let owned = wire.patch(&path, json!({ "head": "refs/heads/feat2" }), &bob.head);
    assert!(
        owned.ok(),
        "author cannot edit own proposal {}",
        owned.status
    );
    let grab = wire.patch(&path, json!({ "head": "refs/heads/theirs" }), &cy.head);
    assert!(
        refused(grab.status),
        "stranger edited a proposal {}",
        grab.status
    );

    let raised = wire.made(
        "/issue",
        issue("PR: from a contributor", repo, cy.id),
        &cy.head,
    );
    let offered = wire.post(
        "/propose",
        json!({ "issue": raised, "base": "refs/heads/main", "head": "refs/heads/theirs" }),
        &cy.head,
    );
    assert_eq!(offered.status, 201, "a contributor could not propose");
    let path = format!("/pull/{}", offered.id());
    let edit = wire.patch(&path, json!({ "head": "refs/heads/theirs2" }), &cy.head);
    assert!(
        edit.ok(),
        "an author cannot edit an unmerged proposal {}",
        edit.status
    );
    let forged = wire.patch(&path, json!({ "weld": "0".repeat(40) }), &cy.head);
    assert!(
        refused(forged.status),
        "an author wrote a merge onto a proposal {}",
        forged.status
    );
    let stamped = wire.patch(&path, json!({ "weld": "1".repeat(40) }), &bob.head);
    assert!(
        stamped.ok(),
        "the repository owner cannot record a merge {}",
        stamped.status
    );
    let late = wire.patch(&path, json!({ "head": "refs/heads/late" }), &cy.head);
    assert!(
        refused(late.status),
        "an author edited a merged proposal {}",
        late.status
    );
}
