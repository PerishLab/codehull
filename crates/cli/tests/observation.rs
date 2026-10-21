#[path = "observation/fixture.rs"]
mod fixture;
use fixture::Fixture;
use serde_json::Value;
const ENABLED: &str = "[locus]\nenabled = true\nendpoint = 'http://127.0.0.1:0'\n";
const STRAY: &[&str] = &["private-argument"];

fn events(atoms: &[Value], event: &str) -> Vec<Value> {
    atoms
        .iter()
        .filter(|atom| atom["payload"]["event"] == event)
        .cloned()
        .collect()
}

#[test]
fn disabled() {
    let fixture = Fixture::new();
    let baseline = fixture.run(&[]);
    fixture.configure("[locus]\nenabled = false\n");
    let disabled = fixture.run(&[]);
    assert_eq!(baseline.status.code(), Some(0));
    assert_eq!(baseline.stdout, disabled.stdout);
    assert_eq!(baseline.stderr, disabled.stderr);
    assert_eq!(fixture.run(STRAY).status.code(), Some(2));
    assert!(!fixture.seat().join("state/locus").exists());
}

#[test]
fn buffered() {
    let fixture = Fixture::new();
    fixture.configure(ENABLED);
    let normal = fixture.run(&[]);
    let refused = fixture.run(STRAY);
    assert_eq!(normal.status.code(), Some(0));
    assert_eq!(refused.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&normal.stdout).contains("Usage: codehull"));
    assert!(String::from_utf8_lossy(&refused.stderr).contains("private-argument"));
    for quiet in [["--help"], ["--version"]] {
        assert_eq!(fixture.run(&quiet).status.code(), Some(0));
    }
    let atoms = fixture.records();
    assert!(
        atoms.iter().all(|atom| atom["producer"] == "codehull"
            && atom["context"]["codehull.command"] == "codehull")
    );
    assert_eq!(events(&atoms, "cli.start").len(), 2);
    let finish = events(&atoms, "cli.finish");
    let codes: Vec<i64> = finish
        .iter()
        .map(|atom| atom["payload"]["code"].as_i64().unwrap())
        .collect();
    assert_eq!(codes, [0, 2]);
    assert!(finish[0]["context"].get("codehull.fault").is_none());
    assert_eq!(finish[1]["context"]["codehull.fault"], "usage");
    for edge in ["enter", "return"] {
        assert_eq!(
            atoms
                .iter()
                .filter(|atom| atom["source"]["edge"] == edge)
                .count(),
            2
        );
    }
    let text = serde_json::to_string(&atoms).unwrap();
    for private in ["private-environment", "private-argument", "127.0.0.1"] {
        assert!(!text.contains(private));
    }
}

#[test]
fn isolated() {
    let fixture = Fixture::new();
    let normal = fixture.run(&[]);
    let refused = fixture.run(STRAY);
    for config in [
        ENABLED,
        "[locus]\nenabled = 'private-setting'\n",
        "[locus]\nenabled = true\nendpoint = ''\n",
    ] {
        fixture.configure(config);
        let first = fixture.run(&[]);
        let second = fixture.run(STRAY);
        assert_eq!(first.status.code(), normal.status.code());
        assert_eq!(second.status.code(), refused.status.code());
        assert_eq!(first.stdout, normal.stdout);
        assert!(
            String::from_utf8_lossy(&second.stderr)
                .ends_with(&*String::from_utf8_lossy(&refused.stderr))
        );
        assert!(!String::from_utf8_lossy(&first.stderr).contains("private-setting"));
    }
    let blocked = Fixture::new();
    blocked.configure(ENABLED);
    std::fs::write(blocked.seat().join("state"), "occupied").unwrap();
    let result = blocked.run(&[]);
    assert_eq!(result.status.code(), Some(0));
    assert_eq!(result.stdout, normal.stdout);
}
