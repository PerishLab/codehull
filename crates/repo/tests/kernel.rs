use codehull_repo::{Error, Object, Store};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(1);

struct Temp(PathBuf);

impl Temp {
    fn new() -> Self {
        let number = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "codehull-repo-test-{}-{number}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("temp root");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).expect("retire temp root");
    }
}

struct Source {
    main: Object,
    topic: Object,
    bundle: Vec<u8>,
}

impl Source {
    fn new() -> Self {
        let root = Temp::new();
        git(root.path(), ["init", "-q", "-b", "main"]);
        git(root.path(), ["config", "user.name", "Codehull Test"]);
        git(
            root.path(),
            ["config", "user.email", "codehull@example.invalid"],
        );
        std::fs::write(root.path().join("probe"), "main\n").expect("main file");
        git(root.path(), ["add", "probe"]);
        git(root.path(), ["commit", "-q", "-m", "seed main"]);
        let main = object(root.path());
        git(root.path(), ["checkout", "-q", "-b", "topic"]);
        std::fs::write(root.path().join("probe"), "topic\n").expect("topic file");
        git(root.path(), ["commit", "-q", "-am", "advance topic"]);
        let topic = object(root.path());
        let path = root.path().join("seed.bundle");
        let output = Command::new("git")
            .arg("-C")
            .arg(root.path())
            .args(["bundle", "create"])
            .arg(&path)
            .arg("--all")
            .output()
            .expect("bundle");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let bundle = std::fs::read(path).expect("bundle bytes");
        Self {
            main,
            topic,
            bundle,
        }
    }
}

#[test]
fn partial() {
    let temp = Temp::new();
    let root = temp.path().join("store");
    let store = Store::create(&root).expect("store");
    std::fs::create_dir(root.join("repos/7.git")).expect("partial repository");
    let error = store
        .provision(7)
        .expect_err("partial repository must refuse");
    assert!(matches!(error, Error::Foreign(_)));
}

#[test]
fn foreign() {
    let temp = Temp::new();
    let root = temp.path().join("store");
    std::fs::create_dir(&root).expect("foreign root");
    let error = Store::create(&root).expect_err("foreign root must refuse");
    assert!(matches!(error, Error::Foreign(_)));
}

#[test]
fn retention() {
    let temp = Temp::new();
    let source = Source::new();
    let root = temp.path().join("store");
    let store = Store::create(&root).expect("store");
    let repo = store.provision(7).expect("repository");
    assert_eq!(store.provision(7).expect("idempotent").id(), 7);
    repo.ingest(&source.bundle, &source.main)
        .expect("ingest main");
    repo.ingest(&source.bundle, &source.topic)
        .expect("ingest topic");

    let main = "refs/heads/main";
    let topic = "refs/heads/topic";
    assert_eq!(
        repo.advance(main, None, &source.main).expect("seed"),
        source.main
    );
    repo.advance(topic, None, &source.topic).expect("topic");
    let stale = repo.advance(main, Some(&source.topic), &source.topic);
    assert!(matches!(stale, Err(Error::Conflict(_))));
    assert_eq!(
        repo.reference(main).expect("main"),
        Some(source.main.clone())
    );

    repo.advance(main, Some(&source.main), &source.topic)
        .expect("advance main");
    assert_eq!(
        repo.reference(topic).expect("topic"),
        Some(source.topic.clone())
    );

    let reopened = Store::open(&root)
        .expect("reopen store")
        .repository(7)
        .expect("reopen repository");
    assert_eq!(
        reopened.reference(main).expect("restarted main"),
        Some(source.topic)
    );
}

fn git<const N: usize>(root: &Path, args: [&str; N]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .expect("git");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn object(root: &Path) -> Object {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("git");
    assert!(output.status.success());
    Object::parse(String::from_utf8_lossy(&output.stdout).trim()).expect("object")
}
