#[path = "acceptance/issuer.rs"]
#[allow(dead_code)]
mod issuer;
#[path = "smoke/stage.rs"]
mod stage;
#[path = "acceptance/wire.rs"]
mod wire;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde_json::Value;
use stage::Stage;
use std::path::Path;
use ureq::Agent;
use wire::{Head, Wire};

const ACCESS: &str = "urn:ietf:params:oauth:token-type:access_token";

fn grant(issuer: &str, key: &str) -> Value {
    let agent: Agent = Agent::config_builder()
        .http_status_as_error(false)
        .build()
        .into();
    let mut res = agent
        .post(format!("{issuer}/token"))
        .send_form([
            (
                "grant_type",
                "urn:ietf:params:oauth:grant-type:token-exchange",
            ),
            ("subject_token", key),
            ("subject_token_type", ACCESS),
            ("resource", "codehull"),
        ])
        .expect("token exchange");
    let status = res.status().as_u16();
    let text = res.body_mut().read_to_string().unwrap_or_default();
    assert_eq!(status, 200, "token exchange: {text}");
    serde_json::from_str(&text).expect("token exchange body")
}

fn claims(token: &str) -> Value {
    let body = token.split('.').nth(1).expect("jwt payload");
    let bytes = URL_SAFE_NO_PAD.decode(body).expect("jwt payload encoding");
    serde_json::from_slice(&bytes).expect("jwt payload json")
}

#[test]
fn meet() {
    let issuer = std::env::var("ENSIGN_ISSUER").unwrap_or_default();
    let path = std::env::var("ENSIGN_KEY").unwrap_or_default();
    if issuer.is_empty() || path.is_empty() {
        eprintln!(
            "ENSIGN_ISSUER or ENSIGN_KEY absent: the ensign handshake stays unproven this run"
        );
        return;
    }
    let issuer = issuer.trim_end_matches('/');
    let key = std::fs::read_to_string(&path).expect("ensign key file");
    let held = grant(issuer, key.trim());
    let token = held["access_token"].as_str().expect("access_token");
    assert_eq!(held["issued_token_type"], ACCESS, "issued_token_type");
    assert!(
        held.get("refresh_token").is_none(),
        "a refresh_token came back"
    );
    assert!(held.get("id_token").is_none(), "an id_token came back");
    let seen = claims(token);
    assert_eq!(seen["aud"], "codehull", "aud");
    assert_eq!(seen["kind"], "access", "kind");
    let sub = seen["sub"].as_str().expect("sub");

    let stage = Stage::external(Path::new(env!("CARGO_BIN_EXE_codehull-api")), issuer);
    let wire = Wire::new(&stage.base);
    let head: Head = vec![("authorization".into(), format!("Bearer {token}"))];
    let rows = wire
        .query(&format!("from Actor where sub = \"{sub}\""), &head)
        .roots();
    assert_eq!(rows.len(), 1, "anchor row for {sub}");
    let actor = rows[0]["id"].as_i64().expect("actor id");
    let repo = wire.private(&head, "meet", actor);
    let path = format!("/repo/{repo}");
    assert_eq!(
        wire.get(&path, &head).status,
        200,
        "the bearer cannot read its repo"
    );
    let bare = wire.get(&path, &Vec::new()).status;
    eprintln!("unsigned GET {path} answered {bare}");
    assert_ne!(bare, 200, "an unsigned caller read the repo");
    assert_eq!(
        bare, 404,
        "an unsigned caller is refused as the seam refuses"
    );
}
