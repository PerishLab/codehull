use codehull_repo::{Error, Object, Store};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
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
    repo.project(main, &source.main).expect("seed main");
    repo.project(topic, &source.topic).expect("seed topic");
    assert_eq!(
        repo.reference(main).expect("main"),
        Some(source.main.clone())
    );
    let held = repo.references().expect("references");
    assert_eq!(held.len(), 2);
    assert!(held.contains(&(main.to_string(), source.main.clone())));
    assert!(held.contains(&(topic.to_string(), source.topic.clone())));

    repo.project(main, &source.topic).expect("move main");
    assert_eq!(
        repo.reference(topic).expect("topic"),
        Some(source.topic.clone())
    );

    repo.retire(topic).expect("retire topic");
    assert_eq!(repo.reference(topic).expect("retired"), None);
    repo.retire(topic)
        .expect("retiring a projection is idempotent");

    let reopened = Store::open(&root)
        .expect("reopen store")
        .repository(7)
        .expect("reopen repository");
    assert_eq!(
        reopened.reference(main).expect("restarted main"),
        Some(source.topic)
    );
}

#[test]
fn quarantine() {
    let temp = Temp::new();
    let work = temp.path().join("work");
    std::fs::create_dir(&work).expect("work root");
    git(&work, ["init", "-q", "-b", "main"]);
    git(&work, ["config", "user.name", "Codehull Test"]);
    git(&work, ["config", "user.email", "codehull@example.invalid"]);
    std::fs::write(work.join("probe"), "penned\n").expect("probe file");
    git(&work, ["add", "probe"]);
    git(&work, ["commit", "-q", "-m", "seed pen"]);
    let held = object(&work);
    let pack = packed(&work, &held);

    let store = Store::create(&temp.path().join("store")).expect("store");
    let repo = store.provision(7).expect("repository");
    let pen = repo.pen().expect("pen");
    repo.index(&pen, &pack).expect("index into the pen");
    assert!(repo.holds(&held).is_err());
    repo.sees(&pen, &held).expect("the pen carries the object");
    let hold = pen.hold().to_path_buf();
    pen.wipe().expect("wipe the pen");
    assert!(!hold.exists());
    assert!(repo.holds(&held).is_err());

    let pen = repo.pen().expect("second pen");
    repo.index(&pen, &pack).expect("index again");
    let hold = pen.hold().to_path_buf();
    pen.keep().expect("keep the pen");
    assert!(!hold.exists());
    repo.holds(&held).expect("the store carries the object");
    repo.project("refs/heads/main", &held)
        .expect("project the admitted object");
}

fn packed(root: &Path, object: &Object) -> Vec<u8> {
    let mut child = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["pack-objects", "--stdout", "--revs"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("pack");
    child
        .stdin
        .take()
        .expect("pack input")
        .write_all(format!("{}\n", object.hex()).as_bytes())
        .expect("pack revisions");
    let output = child.wait_with_output().expect("pack output");
    assert!(output.status.success());
    output.stdout
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
