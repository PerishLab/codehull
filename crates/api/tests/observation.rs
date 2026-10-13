use serde_json::Value;
use std::path::PathBuf;
use std::process::{Command, Output};

struct Fixture {
    root: tempfile::TempDir,
    home: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let fixture = Self {
            root: tempfile::tempdir().unwrap(),
            home: tempfile::tempdir().unwrap(),
        };
        std::fs::write(
            fixture.root.path().join("codehull.toml"),
            "[store]\nkind = 'file'\npath = 'estate.db'\n",
        )
        .unwrap();
        std::fs::create_dir(fixture.seat()).unwrap();
        fixture
    }
    fn seat(&self) -> PathBuf {
        self.home.path().join(if cfg!(windows) {
            "codehull"
        } else {
            ".codehull"
        })
    }
    fn config(&self, text: &str) {
        std::fs::write(self.seat().join("config.toml"), text).unwrap();
    }
    fn run(&self, bad: bool) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_codehull-api"));
        command
            .env_clear()
            .env("HOME", self.home.path())
            .env("USERPROFILE", self.home.path())
            .env("LOCALAPPDATA", self.home.path())
            .arg("bootstrap")
            .arg(self.root.path())
            .arg("--artifact");
        if bad {
            command.arg("private-artifact=file:private-destination");
        } else {
            command.arg(format!(
                "sudo=file:{}",
                self.root.path().join("sudo").display()
            ));
        }
        for key in ["PATH", "SYSTEMROOT", "WINDIR"] {
            if let Some(value) = std::env::var_os(key) {
                command.env(key, value);
            }
        }
        command.output().unwrap()
    }
    fn records(&self) -> Vec<Value> {
        let mut atoms = Vec::new();
        for entry in std::fs::read_dir(self.seat().join("state/locus")).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy();
            if name == "active.jsonl" || name.starts_with("sealed-") {
                let text = std::fs::read_to_string(path).unwrap();
                atoms.extend(
                    text.lines()
                        .map(|line| serde_json::from_str::<Value>(line).unwrap()),
                );
            }
        }
        atoms
    }
}

#[test]
fn replay() {
    let fixture = Fixture::new();
    fixture.config("[locus]\nenabled = true\nendpoint = 'http://127.0.0.1:0'\n");
    assert_eq!(fixture.run(false).status.code(), Some(0));
    let saved = std::fs::read(fixture.root.path().join("sudo")).unwrap();
    assert_eq!(fixture.run(false).status.code(), Some(0));
    assert_eq!(
        saved,
        std::fs::read(fixture.root.path().join("sudo")).unwrap()
    );
    assert_eq!(fixture.run(true).status.code(), Some(1));
    let atoms = fixture.records();
    assert!(atoms.iter().all(|atom| atom["producer"] == "codehull"
        && atom["context"]["codehull.command"] == "api.bootstrap"));
    assert_eq!(
        atoms
            .iter()
            .filter(|atom| atom["source"]["edge"] == "enter")
            .count(),
        3
    );
    assert_eq!(
        atoms
            .iter()
            .filter(|atom| atom["source"]["edge"] == "return")
            .count(),
        2
    );
    assert_eq!(
        atoms
            .iter()
            .filter(|atom| atom["payload"]["event"] == "cli.finish")
            .count(),
        2
    );
    let text = serde_json::to_string(&atoms).unwrap();
    assert!(!text.contains("private-artifact"));
    assert!(!text.contains("private-destination"));
    assert!(!text.contains(fixture.root.path().to_str().unwrap()));
    assert!(!text.contains(std::str::from_utf8(&saved).unwrap().trim()));
}

#[test]
fn isolated() {
    let fixture = Fixture::new();
    assert_eq!(fixture.run(false).status.code(), Some(0));
    let saved = std::fs::read(fixture.root.path().join("sudo")).unwrap();
    for config in [
        "[locus]\nenabled = false\n",
        "[locus]\nenabled = true\nendpoint = 'http://127.0.0.1:0'\n",
        "[locus]\nenabled = 'private-setting'\n",
        "[locus]\nenabled = true\nendpoint = ''\n",
    ] {
        fixture.config(config);
        let output = fixture.run(false);
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(
            saved,
            std::fs::read(fixture.root.path().join("sudo")).unwrap()
        );
        assert!(!String::from_utf8_lossy(&output.stderr).contains("private-setting"));
        assert_eq!(fixture.run(true).status.code(), Some(1));
    }
    let blocked = Fixture::new();
    blocked.config("[locus]\nenabled = true\n");
    std::fs::write(blocked.seat().join("state"), "occupied").unwrap();
    assert_eq!(blocked.run(false).status.code(), Some(0));
}
