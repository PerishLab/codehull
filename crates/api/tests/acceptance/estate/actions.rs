use super::super::open;
use serde_json::json;

#[test]
fn actions() {
    let (stage, wire) = open();
    let bob = wire.join(&stage.issuer, "bob", &[]);
    let cy = wire.join(&stage.issuer, "cy", &[]);
    let head = &bob.head;
    let repo = wire.private(head, "ci", bob.id);
    wire.deed(
        "repo:runner",
        json!({ "name": "linux", "token": "rt1", "labels": "docker,linux", "status": "idle", "repo": repo }),
        head,
    );
    wire.want(
        "/run",
        json!({ "event": "push", "status": "success", "commit": "abc123", "repo": repo }),
        head,
    );
    wire.deed(
        "repo:secret",
        json!({ "name": "TOKEN", "data": "ciphertext", "repo": repo }),
        head,
    );
    let dup = wire.one(
        "repo:secret",
        json!({ "name": "TOKEN", "data": "other", "repo": repo }),
        head,
    );
    assert_eq!(dup.status, 409, "secret name not scoped-unique");
    wire.want(
        "/variable",
        json!({ "name": "REGION", "value": "eu", "repo": repo }),
        head,
    );
    wire.deed(
        "repo:key",
        json!({ "title": "deploy", "print": "ssh-ed25519 AAAA", "repo": repo }),
        head,
    );
    let ask = format!("from repo:secret where repo = \"{repo}\"");
    assert_eq!(
        wire.query(&ask, head).roots().len(),
        1,
        "owner cannot see own secret"
    );
    assert!(
        wire.query(&ask, &cy.head).roots().is_empty(),
        "stranger sees repo secret"
    );
}
