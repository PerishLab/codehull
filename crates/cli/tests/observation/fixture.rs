use serde_json::Value;
use std::path::PathBuf;
use std::process::{Command, Output};

pub(super) struct Fixture {
    pub(super) root: tempfile::TempDir,
    home: tempfile::TempDir,
}

impl Fixture {
    pub(super) fn new() -> Self {
        let fixture = Self {
            root: tempfile::tempdir().unwrap(),
            home: tempfile::tempdir().unwrap(),
        };
        std::fs::create_dir(fixture.seat()).unwrap();
        fixture
    }

    pub(super) fn seat(&self) -> PathBuf {
        self.home.path().join(if cfg!(windows) {
            "codehull"
        } else {
            ".codehull"
        })
    }

    pub(super) fn configure(&self, text: &str) {
        std::fs::write(self.seat().join("config.toml"), text).unwrap();
    }

    pub(super) fn run(&self, args: &[&str]) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_codehull"));
        command
            .env_clear()
            .env("HOME", self.home.path())
            .env("USERPROFILE", self.home.path())
            .env("LOCALAPPDATA", self.home.path())
            .env("PRIVATE_FIXTURE", "private-environment");
        for key in ["PATH", "SYSTEMROOT", "WINDIR"] {
            if let Some(value) = std::env::var_os(key) {
                command.env(key, value);
            }
        }
        command
            .current_dir(self.root.path())
            .args(args)
            .output()
            .unwrap()
    }

    pub(super) fn records(&self) -> Vec<Value> {
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
