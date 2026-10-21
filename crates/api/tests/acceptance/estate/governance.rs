use super::super::open;
use super::super::wire::num;
use serde_json::json;

#[test]
fn governance() {
    let (stage, wire) = open();
    let ada = wire.join(&stage.issuer, "ada", &[]);
    let bob = wire.join(&stage.issuer, "bob", &[]);
    let cy = wire.join(&stage.issuer, "cy", &[]);

    let org = wire.made("/org", json!({ "name": "lab" }), &ada.head);
    let pack = wire.query(
        &format!("from Team where org = \"{org}\" link members"),
        &ada.head,
    );
    let team = pack.roots();
    assert_eq!(team.len(), 1, "owners team not created atomically");
    let crew = num(&team[0]["id"]);
    assert_eq!(
        pack.bond("actor:team.members").len(),
        1,
        "creator not enrolled atomically"
    );
    let clash = wire.post("/org", json!({ "name": "lab" }), &bob.head);
    assert!(
        clash.status == 403 || clash.status == 409,
        "dup org {}",
        clash.status
    );
    let after = wire.query("from Actor where sub = \"lab\" count", &ada.head);
    assert_eq!(after.count(), 1, "rolled-back org left a row");

    let vault = wire.private(&ada.head, "vault", org);
    wire.admit(&ada.head, crew, vault);
    assert_ne!(wire.peek(vault, &bob.head), 200, "bob sees before joining");
    wire.want(
        &format!("/team/{crew}/members"),
        json!({ "right": bob.id }),
        &ada.head,
    );
    assert!(wire.visible(vault, &bob.head), "member cannot see repo");
    assert!(!wire.visible(vault, &cy.head), "stranger sees repo");

    let pack = wire.query(
        &format!("from Team where id = \"{crew}\" link members"),
        &ada.head,
    );
    let tie = pack
        .bond("actor:team.members")
        .into_iter()
        .find(|tie| num(&tie["right"]) == bob.id)
        .expect("bob membership tie missing");
    let gone = wire.cut(
        &format!("/team/{crew}/members/{}", num(&tie["id"])),
        &ada.head,
    );
    assert_eq!(gone.status, 204, "remove");
    assert!(
        !wire.visible(vault, &bob.head),
        "access outlived membership"
    );
}
