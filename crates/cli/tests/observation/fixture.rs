use serde_json::Value;
use std::path::PathBuf;
use std::process::{Command, Output};

pub(super) struct Fixture {
    pub(super) root: tempfile::TempDir,
    pub(super) tools: tempfile::TempDir,
    home: tempfile::TempDir,
}

impl Fixture {
    pub(super) fn new() -> Self {
        let fixture = Self {
            root: tempfile::tempdir().unwrap(),
            tools: tempfile::tempdir().unwrap(),
            home: tempfile::tempdir().unwrap(),
        };
        assert!(
            Command::new("git")
                .args(["init", "-q"])
                .arg(fixture.root.path())
                .status()
                .unwrap()
                .success()
        );
        let source = fixture.tools.path().join("child.rs");
        std::fs::write(&source, r#"fn main() {
            for key in ["DYLD_FALLBACK_LIBRARY_PATH", "DYLD_INSERT_LIBRARIES", "DYLD_LIBRARY_PATH", "LD_PRELOAD", "LD_LIBRARY_PATH"] { assert!(std::env::var_os(key).is_none()); }
            assert_eq!(std::env::var("PRIVATE_FIXTURE").unwrap(), "private-environment");
            let args: Vec<String> = std::env::args().skip(1).collect();
            assert_eq!(&args[..7], ["run", "--allow-read", "--allow-write", "--allow-env", "--allow-net", "--allow-run", "--config"]);
            assert!(args[7].ends_with("crates/cli/scripts/deno.json")); assert!(args[8].ends_with("crates/cli/scripts/act.ts"));
            println!("private-child-output"); eprintln!("private-child-error");
            std::process::exit(std::env::var("PRIVATE_STATUS").unwrap().parse().unwrap());
        }"#).unwrap();
        let binary = fixture
            .tools
            .path()
            .join(if cfg!(windows) { "deno.exe" } else { "deno" });
        assert!(
            Command::new("rustc")
                .arg(&source)
                .args(["--edition", "2024", "-o"])
                .arg(binary)
                .status()
                .unwrap()
                .success()
        );
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

    pub(super) fn run(&self, status: i32) -> Output {
        let path = std::env::join_paths(
            std::iter::once(self.tools.path().to_path_buf())
                .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
        )
        .unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_codehull"));
        command
            .env_clear()
            .env("PATH", path)
            .env("HOME", self.home.path())
            .env("USERPROFILE", self.home.path())
            .env("LOCALAPPDATA", self.home.path())
            .env("PRIVATE_FIXTURE", "private-environment")
            .env("PRIVATE_STATUS", status.to_string())
            .env("DYLD_LIBRARY_PATH", "private-loader");
        for key in ["SYSTEMROOT", "WINDIR"] {
            if let Some(value) = std::env::var_os(key) {
                command.env(key, value);
            }
        }
        command
            .current_dir(self.root.path())
            .arg("act")
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
