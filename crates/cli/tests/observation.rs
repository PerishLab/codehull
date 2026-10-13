#[path = "observation/fixture.rs"]
mod fixture;
use fixture::Fixture;
const ENABLED: &str = "[locus]\nenabled = true\nendpoint = 'http://127.0.0.1:0'\n";

#[test]
fn disabled() {
    let fixture = Fixture::new();
    let baseline = fixture.run(0);
    fixture.configure("[locus]\nenabled = false\n");
    let disabled = fixture.run(0);
    assert_eq!(baseline.status.code(), Some(0));
    assert_eq!(baseline.stdout, disabled.stdout);
    assert_eq!(baseline.stderr, disabled.stderr);
    assert!(!fixture.seat().join("state/locus").exists());
}

#[test]
fn buffered() {
    let fixture = Fixture::new();
    fixture.configure(ENABLED);
    let normal = fixture.run(0);
    let refused = fixture.run(17);
    assert_eq!(normal.status.code(), Some(0));
    assert_eq!(refused.status.code(), Some(1));
    assert_eq!(normal.stdout, refused.stdout);
    assert_eq!(normal.stdout, b"private-child-output\n");
    let atoms = fixture.records();
    assert!(
        atoms
            .iter()
            .all(|atom| atom["producer"] == "codehull"
                && atom["context"]["codehull.command"] == "act")
    );
    for edge in ["enter", "return"] {
        assert_eq!(
            atoms
                .iter()
                .filter(|atom| atom["source"]["edge"] == edge)
                .count(),
            10
        );
    }
    let text = serde_json::to_string(&atoms).unwrap();
    for private in [
        "private-environment",
        "private-loader",
        "private-child-output",
        "private-child-error",
        "act.ts",
        "deno.json",
        "127.0.0.1",
    ] {
        assert!(!text.contains(private));
    }
}

#[test]
fn isolated() {
    let fixture = Fixture::new();
    let normal = fixture.run(0);
    let refused = fixture.run(17);
    for config in [
        ENABLED,
        "[locus]\nenabled = 'private-setting'\n",
        "[locus]\nenabled = true\nendpoint = ''\n",
    ] {
        fixture.configure(config);
        let first = fixture.run(0);
        let second = fixture.run(17);
        assert_eq!(first.status.code(), normal.status.code());
        assert_eq!(second.status.code(), refused.status.code());
        assert_eq!(first.stdout, normal.stdout);
        assert_eq!(second.stdout, refused.stdout);
        assert!(String::from_utf8_lossy(&first.stderr).contains("private-child-error"));
        assert!(!String::from_utf8_lossy(&first.stderr).contains("private-setting"));
    }
    let blocked = Fixture::new();
    blocked.configure(ENABLED);
    std::fs::write(blocked.seat().join("state"), "occupied").unwrap();
    let result = blocked.run(0);
    assert_eq!(result.status.code(), Some(0));
    assert_eq!(result.stdout, normal.stdout);
}
