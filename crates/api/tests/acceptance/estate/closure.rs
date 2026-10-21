use super::super::open;
use serde_json::json;

#[test]
#[ignore = "PerishLab/codehull#45: a bare DELETE of a repository with dependents answers 204 and orphans its comments"]
fn closure() {
    let (stage, wire) = open();
    let bob = wire.join(&stage.issuer, "bob", &[]);
    let head = &bob.head;
    let repo = wire.private(head, "doomed", bob.id);
    let issue = wire.deed(
        "Issue",
        json!({ "title": "orphan-to-be", "body": "", "closed": false, "repo": repo, "author": bob.id }),
        head,
    );
    wire.deed(
        "Comment",
        json!({ "body": "a word", "issue": issue, "author": bob.id }),
        head,
    );
    wire.deed(
        "Reaction",
        json!({ "emoji": "up", "issue": issue, "actor": bob.id }),
        head,
    );
    let pull = wire.deed(
        "Pull",
        json!({
            "base": "refs/heads/main",
            "head": "refs/heads/feat",
            "weld": "",
            "repo": repo,
            "issue": issue,
            "author": bob.id,
        }),
        head,
    );
    let review = wire.deed(
        "Review",
        json!({ "state": "approve", "body": "lgtm", "pull": pull, "reviewer": bob.id }),
        head,
    );
    wire.deed(
        "Note",
        json!({ "path": "src/main.rs", "line": 1, "body": "nit", "review": review }),
        head,
    );
    let shield = wire.deed(
        "Shield",
        json!({ "branch": "refs/heads/main", "force": false, "approvals": 1, "repo": repo }),
        head,
    );
    wire.deed(
        "Demand",
        json!({ "context": "guard", "shield": shield }),
        head,
    );
    let project = wire.deed(
        "Project",
        json!({ "title": "board", "closed": false, "repo": repo }),
        head,
    );
    wire.deed(
        "Column",
        json!({ "title": "todo", "sort": 1, "project": project }),
        head,
    );
    for (unit, fields) in [
        (
            "Milestone",
            json!({ "title": "v1", "due": 0, "closed": false, "repo": repo }),
        ),
        (
            "repo:label",
            json!({ "name": "bug", "color": "ff0000", "repo": repo }),
        ),
        (
            "Release",
            json!({ "tag": "v1", "title": "one", "body": "", "draft": false, "repo": repo, "author": bob.id }),
        ),
        (
            "Package",
            json!({ "name": "thing", "kind": "cargo", "version": "0.1.0", "repo": repo }),
        ),
        (
            "repo:runner",
            json!({ "name": "linux", "token": "rt9", "labels": "docker", "status": "idle", "repo": repo }),
        ),
        (
            "Run",
            json!({ "event": "push", "status": "success", "commit": "abc123", "repo": repo }),
        ),
        (
            "repo:secret",
            json!({ "name": "TOKEN", "data": "ciphertext", "repo": repo }),
        ),
        (
            "Variable",
            json!({ "name": "REGION", "value": "eu", "repo": repo }),
        ),
        (
            "repo:key",
            json!({ "title": "deploy", "print": "ssh-ed25519 AAAA", "repo": repo }),
        ),
        (
            "Mirror",
            json!({ "remote": "https://example.test/doomed.git", "interval": 60, "repo": repo }),
        ),
    ] {
        wire.deed(unit, fields, head);
    }
    let path = format!("/repo/{repo}");
    assert_eq!(
        wire.cut(&path, head).status,
        409,
        "bare delete expected 409"
    );
    assert_eq!(
        wire.knock(&format!("{path}/close"), head).status,
        204,
        "closure"
    );
    assert_eq!(wire.cut(&path, head).status, 404, "repo lingered");
    let kept = wire.query(&format!("from Comment where issue = \"{issue}\""), head);
    assert!(kept.roots().is_empty(), "a comment outlived its repository");
}
