use super::super::open;
use super::super::wire::{Head, num};
use serde_json::json;

#[test]
fn guild() {
    let (stage, wire) = open();
    let ada = wire.join(&stage.issuer, "ada", &[]);
    let cy = wire.join(&stage.issuer, "cy", &[]);
    let org = wire.made("/org", json!({ "name": "guild" }), &ada.head);
    wire.deed(
        "actor:label",
        json!({ "name": "triage", "color": "ff0", "org": org }),
        &ada.head,
    );
    let dup = wire.one(
        "actor:label",
        json!({ "name": "triage", "color": "0ff", "org": org }),
        &ada.head,
    );
    assert_eq!(dup.status, 409, "org label name not scoped-unique");
    wire.deed(
        "actor:runner",
        json!({ "name": "shared", "token": "ort1", "labels": "linux", "status": "idle", "org": org }),
        &ada.head,
    );
    wire.deed(
        "actor:secret",
        json!({ "name": "DEPLOY", "data": "cipher", "org": org }),
        &ada.head,
    );
    let ask = format!("from actor:secret where org = \"{org}\"");
    assert_eq!(
        wire.query(&ask, &ada.head).roots().len(),
        1,
        "owner cannot see org secret"
    );
    assert!(
        wire.query(&ask, &cy.head).roots().is_empty(),
        "non-member sees org secret"
    );
    let steal = wire.one(
        "actor:secret",
        json!({ "name": "GRAB", "data": "x", "org": org }),
        &cy.head,
    );
    assert_eq!(steal.status, 403, "non-member wrote org secret");
}

#[test]
fn upstream() {
    let (stage, wire) = open();
    let ada = wire.join(&stage.issuer, "ada", &[]);
    let crew = wire.join(&stage.issuer, "mariner", &["deck"]);
    let rows = wire
        .query("from Team where name = \"deck\"", &stage.crown())
        .roots();
    assert_eq!(rows.len(), 1, "claimed team did not settle");
    let deck = num(&rows[0]["id"]);
    let vault = wire.private(&ada.head, "hold", ada.id);
    wire.admit(&ada.head, deck, vault);
    assert!(
        wire.visible(vault, &crew.head),
        "a claimed team did not admit its member"
    );
    let ashore: Head = vec![(
        "authorization".into(),
        format!("Bearer {}", stage.issuer.mint("mariner", &[])),
    )];
    assert!(
        !wire.visible(vault, &ashore),
        "dropping the claim left the member admitted"
    );
    let other = wire.join(&stage.issuer, "bosun", &["deck"]);
    assert!(
        wire.visible(vault, &other.head),
        "an emptied team did not admit its next member"
    );
}
