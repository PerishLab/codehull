#![allow(dead_code)]

use super::wire::Seat;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub const NAME: &str = "Codehull Act";
pub const MAIL: &str = "codehull@example.invalid";

pub struct Client {
    home: PathBuf,
}

impl Client {
    pub fn new(room: &Path) -> Self {
        let home = room.join("home");
        std::fs::create_dir_all(&home).expect("client home");
        Self { home }
    }

    pub fn command(&self) -> Command {
        let mut command = Command::new("git");
        command
            .env_clear()
            .env("HOME", &self.home)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_TERMINAL_PROMPT", "0")
            .stdin(Stdio::null());
        for key in ["PATH", "SYSTEMROOT", "WINDIR"] {
            if let Some(value) = std::env::var_os(key) {
                command.env(key, value);
            }
        }
        command
    }

    pub fn text(&self, work: &Path, args: &[&str]) -> String {
        let done = self
            .command()
            .arg("-C")
            .arg(work)
            .args(args)
            .stderr(Stdio::null())
            .output()
            .expect("git");
        assert!(done.status.success(), "git {args:?} in {}", work.display());
        String::from_utf8_lossy(&done.stdout).trim_end().to_string()
    }

    pub fn status(&self, args: &[&str]) -> bool {
        self.command()
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("git")
            .success()
    }

    pub fn clone(&self, url: &str, into: &Path, seat: Option<&Seat>) -> bool {
        let header = seat.map(extra);
        let mut args = Vec::new();
        if let Some(header) = &header {
            args.extend(["-c", header.as_str()]);
        }
        let into = into.to_string_lossy();
        args.extend(["clone", "--quiet", url, &into]);
        self.status(&args)
    }

    pub fn push(&self, work: &Path, seat: &Seat, args: &[&str]) -> bool {
        let header = extra(seat);
        let work = work.to_string_lossy();
        let mut all = vec!["-c", header.as_str(), "-C", &work, "push"];
        all.extend(args);
        self.status(&all)
    }

    pub fn fetch(&self, work: &Path, seat: &Seat, args: &[&str]) -> bool {
        let header = extra(seat);
        let work = work.to_string_lossy();
        let mut all = vec!["-C", &work, "-c", header.as_str(), "fetch"];
        all.extend(args);
        self.status(&all)
    }

    pub fn sign(&self, work: &Path) {
        self.text(work, &["config", "user.name", NAME]);
        self.text(work, &["config", "user.email", MAIL]);
    }

    pub fn stored(&self, bare: &Path, object: &str) -> bool {
        let bare = bare.to_string_lossy();
        self.status(&[
            "-C",
            &bare,
            "cat-file",
            "-e",
            &format!("{object}^{{commit}}"),
        ])
    }
}

pub fn extra(seat: &Seat) -> String {
    format!("http.extraHeader=Authorization: {}", seat.head[0].1)
}

pub struct Source {
    pub main: String,
    pub topic: Option<String>,
    pub bundle: String,
}

impl Source {
    pub fn new(client: &Client, dir: &Path, topic: bool) -> Self {
        let seed = dir.join("seed");
        std::fs::create_dir_all(&seed).expect("seed");
        client.text(&seed, &["init", "-q", "-b", "main"]);
        client.sign(&seed);
        std::fs::write(seed.join("probe"), "main\n").expect("probe");
        client.text(&seed, &["add", "probe"]);
        client.text(&seed, &["commit", "-q", "-m", "seed main"]);
        let main = client.text(&seed, &["rev-parse", "HEAD"]);
        let topic = topic.then(|| {
            client.text(&seed, &["checkout", "-q", "-b", "topic"]);
            std::fs::write(seed.join("probe"), "topic\n").expect("probe");
            client.text(&seed, &["commit", "-q", "-am", "advance topic"]);
            client.text(&seed, &["rev-parse", "HEAD"])
        });
        let pack = dir.join("seed.bundle");
        client.text(
            &seed,
            &["bundle", "create", &pack.to_string_lossy(), "--all"],
        );
        let bundle = STANDARD.encode(std::fs::read(&pack).expect("bundle"));
        Self {
            main,
            topic,
            bundle,
        }
    }
}
