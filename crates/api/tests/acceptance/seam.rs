use super::issuer::Issuer;
use super::open;
use super::wire::Head;
use serde_json::json;

fn bearer(token: &str) -> Head {
    vec![("authorization".into(), format!("Bearer {token}"))]
}

#[test]
fn namespace() {
    let (stage, wire) = open();
    let bare = wire.fetch(&format!("{}/health", stage.origin), &Vec::new());
    assert_eq!(bare.status, 404, "root health");
}

#[test]
fn seam() {
    let (stage, wire) = open();
    let seat = wire.join(&stage.issuer, "warden", &[]);
    let vault = wire.private(&seat.head, "secret", seat.id);
    let path = format!("/repo/{vault}");
    assert_eq!(
        wire.get(&path, &seat.head).status,
        200,
        "operator cannot see own repo"
    );
    let held = seat.token();
    let (rest, seal) = held.rsplit_once('.').expect("token seal");
    let tail = if seal.ends_with("AA") { "BB" } else { "AA" };
    let forged = format!("{rest}.{}{tail}", &seal[..seal.len() - 2]);
    let other = Issuer::new().mint("warden", &[]);
    for (note, token) in [("a forged signature", forged), ("a stranger", other)] {
        let res = wire.get(&path, &bearer(&token));
        assert_eq!(res.status, 404, "{note} resolved an operator");
    }
    let login = vec![("x-login".to_string(), "warden".to_string())];
    assert_eq!(
        wire.get(&path, &login).status,
        404,
        "a bare login header resolved an operator"
    );
}

#[test]
fn blob() {
    let (stage, wire) = open();
    if !stage.blob {
        eprintln!("API_BLOB_ENDPOINT absent: the blob plane stays unproven this run");
        return;
    }
    let bob = wire.join(&stage.issuer, "bob", &[]);
    let cy = wire.join(&stage.issuer, "cy", &[]);
    let made = wire.want(
        "/asset",
        json!({ "name": "logo.png", "mime": "image/png", "size": 5, "hash": "h1", "owner": bob.id }),
        &bob.head,
    );
    let put = made["put"].as_str().expect("presigned put");
    let up = wire.upload(put, &[1, 2, 3, 4, 5]);
    assert!(up.ok(), "presigned put {}", up.status);
    let path = format!("/asset/{}", made["id"].as_i64().expect("asset id"));
    let seen = wire.get(&path, &bob.head);
    assert_eq!(seen.status, 302, "gated download");
    let place = seen.header("location").unwrap_or_default().to_string();
    let back = wire.fetch(&place, &Vec::new());
    assert_eq!(back.bytes, [1, 2, 3, 4, 5], "bytes round-trip mismatch");
    assert_eq!(
        wire.get(&path, &cy.head).status,
        404,
        "stranger expected 404"
    );
}
