use super::super::open;
use super::super::wire::num;
use serde_json::json;

#[test]
fn reaction() {
    let (stage, wire) = open();
    let ada = wire.join(&stage.issuer, "ada", &[]);
    let bob = wire.join(&stage.issuer, "bob", &[]);
    let repo = wire.public(&ada.head, "open", ada.id);
    let issue =
        json!({ "title": "hi", "body": "", "closed": false, "repo": repo, "author": bob.id });
    let on = wire.made("/issue", issue, &bob.head);
    let up = json!({ "emoji": "up", "issue": on, "actor": bob.id });
    wire.want("/reaction", up.clone(), &bob.head);
    wire.want(
        "/reaction",
        json!({ "emoji": "tada", "issue": on, "actor": bob.id }),
        &bob.head,
    );
    let dup = wire.post("/reaction", up.clone(), &bob.head);
    assert_eq!(dup.status, 409, "dup reaction");
    let held = wire
        .query(
            &format!("from Reaction where issue = \"{on}\" and emoji = \"up\""),
            &bob.head,
        )
        .roots();
    let gone = wire.cut(&format!("/reaction/{}", num(&held[0]["id"])), &bob.head);
    assert_eq!(gone.status, 204, "undo");
    let again = wire.post("/reaction", up, &bob.head);
    assert_eq!(again.status, 201, "re-react");
}

#[test]
fn watch() {
    let (stage, wire) = open();
    let ada = wire.join(&stage.issuer, "ada", &[]);
    let bob = wire.join(&stage.issuer, "bob", &[]);
    let cy = wire.join(&stage.issuer, "cy", &[]);
    let repo = wire.public(&ada.head, "watched", ada.id);
    wire.want(
        &format!("/actor/{}/watches", bob.id),
        json!({ "right": repo }),
        &bob.head,
    );
    let issue = json!({ "title": "upstream change", "body": "", "closed": false, "repo": repo, "author": ada.id });
    wire.want("/issue", issue, &ada.head);
    let mine = wire.query(
        &format!("from Actor where id = \"{}\" link watches", bob.id),
        &bob.head,
    );
    assert_eq!(
        mine.0["bags"]["actor.watches"].as_array().map(Vec::len),
        Some(1),
        "watch not recorded"
    );
    let feed = format!("from Issue where repo = \"{repo}\" and id > \"0\"");
    assert_eq!(
        wire.query(&feed, &bob.head).roots().len(),
        1,
        "watcher feed expected 1"
    );
    wire.query(&feed, &cy.head);
}
