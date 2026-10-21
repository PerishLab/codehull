mod door;
mod push;
mod refs;
mod shell;
mod verdict;
mod weld;

use super::bench::Bench;
use super::git::{Client, Source};
use super::wire::{Seat, Wire};
use serde_json::json;
use std::path::{Path, PathBuf};

pub const MAIN: &str = "refs/heads/main";

pub struct Haul {
    pub stage: Bench,
    pub wire: Wire,
    pub client: Client,
    pub seat: Seat,
    pub stranger: Seat,
    pub repo: i64,
    pub url: String,
    pub bare: PathBuf,
    pub main: String,
}

impl Haul {
    fn new() -> Self {
        let stage = Bench::seat(true);
        let client = Client::new(&stage.nest);
        let seed = Source::new(&client, &stage.nest, false);
        let wire = Wire::new(&stage.base);
        let seat = wire.join(&stage.issuer, "hauler", &[]);
        let stranger = wire.join(&stage.issuer, "tourist", &[]);
        let repo = wire.private(&seat.head, "haul", seat.id);
        let path = format!("/repo/{repo}/git");
        assert_eq!(wire.put(&path, &seat.head).status, 200, "provision");
        let body = json!({ "object": seed.main, "bundle": seed.bundle });
        let res = wire.post(&format!("{path}/object"), body, &seat.head);
        assert_eq!(res.status, 200, "ingest");
        let body = json!({ "name": MAIN, "after": seed.main });
        let res = wire.post(&format!("{path}/ref"), body, &seat.head);
        assert_eq!(res.status, 200, "seed main");
        Self {
            url: wire.url(&path),
            bare: stage.nest.join(format!("seats/repos/{repo}.git")),
            main: seed.main,
            stage,
            wire,
            client,
            seat,
            stranger,
            repo,
        }
    }

    pub fn dir(&self, name: &str) -> PathBuf {
        self.stage.nest.join(name)
    }

    pub fn work(&self, name: &str) -> PathBuf {
        let work = self.dir(name);
        assert!(
            self.client.clone(&self.url, &work, Some(&self.seat)),
            "clone for {name} failed"
        );
        self.client.sign(&work);
        work
    }

    pub fn at(&self, work: &Path, args: &[&str]) -> String {
        self.client.text(work, args)
    }

    pub fn commit(&self, work: &Path, file: &str, text: &str) -> String {
        std::fs::write(work.join(file), text).expect("work file");
        self.at(work, &["add", file]);
        self.at(work, &["commit", "-q", "-m", file]);
        self.at(work, &["rev-parse", "HEAD"])
    }

    pub fn push(&self, work: &Path, args: &[&str]) -> bool {
        self.client.push(work, &self.seat, args)
    }

    pub fn path(&self, tail: &str) -> String {
        format!("/repo/{}/git{tail}", self.repo)
    }

    pub fn tip(&self, name: &str) -> Option<String> {
        let tail = format!("/ref?name={}", name.replace('/', "%2F"));
        let res = self.wire.get(&self.path(&tail), &self.seat.head);
        assert_eq!(res.status, 200, "read {name}");
        res.body["object"].as_str().map(str::to_string)
    }

    pub fn penned(&self) -> usize {
        std::fs::read_dir(&self.bare)
            .expect("bare seat")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().starts_with(".pen-"))
            .count()
    }
}

#[test]
fn haul() {
    let haul = Haul::new();
    haul.fetch();
    haul.advance();
    haul.bulk();
    haul.stale();
    haul.pen();
    haul.retire();
    haul.tags();
    haul.namespace();
    haul.quoted();
    haul.merges();
    haul.proposal();
    haul.shield();
    haul.unpermitted();
    haul.verdicts();
    haul.compressed();
    haul.thief();
    haul.shell();
    haul.denied();
    haul.anonymous();
    haul.organization();
    haul.archive();
    haul.close();
}
