use api::runtime::{Kind, Runtime};
use std::collections::HashMap;

fn resolve(entries: &[(&str, &str)]) -> Runtime {
    let table: HashMap<String, String> = entries
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect();
    let get = |key: &str| table.get(key).cloned();
    Runtime::resolve(None, &Runtime::prefix(), &get).expect("runtime")
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
        ("CODEHULL_BLOB_ENDPOINT", "http://silo:9000"),
        ("CODEHULL_BLOB_KEY", "primary-key"),
        ("CODEHULL_BLOB_SECRET", "primary-secret"),
    ]);
    assert_eq!(runtime.store.kind, Kind::Pg);
    assert_eq!(runtime.store.url, "host=primary");
    assert_eq!(runtime.oidc.issuer, "https://ensign.example/api");
    assert_eq!(runtime.oidc.audience, "codehull");
    assert_eq!(runtime.blob.endpoint, "http://silo:9000");
    assert_eq!(runtime.blob.key, "primary-key");
    assert_eq!(runtime.blob.secret, "primary-secret");
}

#[test]
fn legacy() {
    let runtime = resolve(&[
        ("API_STORE_KIND", "pg"),
        ("API_STORE_URL", "host=legacy"),
        ("API_BLOB_ENDPOINT", "http://silo:9000"),
        ("API_BLOB_KEY", "legacy-key"),
        ("API_BLOB_SECRET", "legacy-secret"),
    ]);
    assert_eq!(runtime.store.kind, Kind::Memory);
    assert!(runtime.store.url.is_empty());
    assert!(runtime.blob.endpoint.is_empty());
    assert!(runtime.blob.key.is_empty());
    assert!(runtime.blob.secret.is_empty());
}
