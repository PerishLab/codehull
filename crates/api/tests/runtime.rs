use api::runtime::{Kind, Runtime};
use plumb::config::migrated;
use std::collections::HashMap;

fn resolve(entries: &[(&str, &str)]) -> Runtime {
    let table: HashMap<String, String> = entries
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect();
    let get = |key: &str| table.get(key).cloned();
    let read = migrated(&Runtime::prefix(), "API", &get);
    Runtime::resolve(None, &Runtime::prefix(), &read).expect("runtime")
}

#[test]
fn prefix() {
    assert_eq!(Runtime::prefix(), "CODEHULL");
}

#[test]
fn primary() {
    let runtime = resolve(&[
        ("CODEHULL_STORE_KIND", "pg"),
        ("CODEHULL_STORE_URL", "host=primary"),
        ("CODEHULL_OIDC_ISSUER", "https://ensign.example/api"),
        ("CODEHULL_OIDC_AUDIENCE", "codehull"),
    ]);
    assert_eq!(runtime.store.kind, Kind::Pg);
    assert_eq!(runtime.store.url, "host=primary");
    assert_eq!(runtime.oidc.issuer, "https://ensign.example/api");
    assert_eq!(runtime.oidc.audience, "codehull");
}

#[test]
fn fallback() {
    let runtime = resolve(&[
        ("API_STORE_KIND", "pg"),
        ("API_STORE_URL", "host=legacy"),
        ("API_BLOB_ENDPOINT", "http://silo:9000"),
        ("API_BLOB_SECRET", "legacy-secret"),
    ]);
    assert_eq!(runtime.store.kind, Kind::Pg);
    assert_eq!(runtime.store.url, "host=legacy");
    assert_eq!(runtime.blob.endpoint, "http://silo:9000");
    assert_eq!(runtime.blob.secret, "legacy-secret");
}

#[test]
fn precedence() {
    let runtime = resolve(&[
        ("CODEHULL_STORE_URL", "host=new"),
        ("API_STORE_URL", "host=old"),
        ("CODEHULL_BLOB_KEY", "   "),
        ("API_BLOB_KEY", "fallback-key"),
    ]);
    assert_eq!(runtime.store.url, "host=new");
    assert_eq!(runtime.blob.key, "fallback-key");
}
