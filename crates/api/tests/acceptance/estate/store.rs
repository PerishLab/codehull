use super::shaky::{Kind, Knob};
use crate::acceptance::bench::Bench;
use crate::acceptance::git::{Client, Source};
use crate::acceptance::wire::{Reply, Seat, Wire};
use serde_json::{Value, json};
use std::sync::Arc;

const MAIN: &str = "refs/heads/main";
const FLAT: &str = "refs/heads/flat";
const ISSUES: &str = "FROM \"repo_issue\"";
const REFS: &str = "FROM \"repo_ref\"";
const PULLS: &str = "FROM \"repo_pull\"";
const SHIELDS: &str = "FROM \"repo_shield\"";
const VERDICTS: &str = "FROM \"repo_verdict\"";
const REPOS: &str = "FROM \"actor_repo\"";
const BEGIN: &str = "BEGIN";

const CASES: [(Kind, u16, &str); 3] = [
    (Kind::Down, 503, "store unavailable"),
    (Kind::Taken, 409, "key taken"),
    (Kind::Fault, 500, "store fault"),
];

struct Stage {
    _stage: Bench,
    wire: Wire,
    seat: Seat,
    knob: Arc<Knob>,
    repo: i64,
    main: String,
    topic: String,
}

impl Stage {
    fn new() -> Self {
        let knob = Arc::new(Knob::default());
        let stage = Bench::shaky(knob.clone());
        let client = Client::new(&stage.nest);
        let seed = Source::new(&client, &stage.nest, true);
        let wire = stage.wire();
        let seat = wire.join(&stage.issuer, "keeper", &[]);
        let repo = wire.private(&seat.head, "vault", seat.id);
        let topic = seed.topic.clone().expect("topic");
        let held = Self {
            _stage: stage,
            wire,
            seat,
            knob,
            repo,
            main: seed.main.clone(),
            topic,
        };
        assert_eq!(held.put("").status, 200, "provision");
        for object in [&held.main, &held.topic] {
            let body = json!({ "object": object, "bundle": seed.bundle });
            assert_eq!(held.post("/object", body).status, 200, "ingest {object}");
        }
        held
    }

    fn path(&self, tail: &str) -> String {
        format!("/repo/{}/git{tail}", self.repo)
    }

    fn put(&self, tail: &str) -> Reply {
        self.wire.put(&self.path(tail), &self.seat.head)
    }

    fn get(&self, tail: &str) -> Reply {
        self.wire.get(&self.path(tail), &self.seat.head)
    }

    fn post(&self, tail: &str, body: Value) -> Reply {
        self.wire.post(&self.path(tail), body, &self.seat.head)
    }

    fn plant(&self, name: &str, before: Option<&str>, after: &str) -> Reply {
        let mut body = json!({ "name": name, "after": after });
        if let Some(before) = before {
            body["before"] = json!(before);
        }
        self.post("/ref", body)
    }

    fn issue(&self) -> i64 {
        let issue = json!({
            "title": "store",
            "body": "",
            "closed": false,
            "repo": self.repo,
            "author": self.seat.id,
        });
        self.wire.made("/issue", issue, &self.seat.head)
    }

    fn pull(&self, head: &str) -> i64 {
        let issue = self.issue();
        let body = json!({ "issue": issue, "base": MAIN, "head": head });
        self.wire.made("/propose", body, &self.seat.head)
    }

    fn permit(&self) {
        for mode in ["forward", "join", "squash"] {
            let fields = json!({ "mode": mode, "repo": self.repo });
            let made = self.wire.one("Weld", fields, &self.seat.head);
            assert_eq!(made.status, 200, "permitting {mode}");
        }
    }

    fn fail(&self, needle: &str, kind: Kind, reply: impl Fn() -> Reply, note: &str) {
        self.issue();
        self.knob.arm(needle, kind);
        let res = reply();
        self.knob.clear();
        let (_, status, text) = CASES
            .iter()
            .find(|(held, ..)| std::mem::discriminant(held) == std::mem::discriminant(&kind))
            .expect("case");
        assert_eq!(res.status, *status, "{note} answered {}", res.status);
        assert_eq!(res.body, json!({ "error": text }), "{note} body");
        let raw = String::from_utf8_lossy(&res.bytes);
        assert!(
            !raw.contains("driver detail") && !raw.contains("db.internal"),
            "{note} leaked the driver detail: {raw}"
        );
    }

    fn each(&self, needle: &str, note: &str, reply: impl Fn() -> Reply) {
        for (kind, ..) in CASES {
            self.fail(needle, kind, &reply, note);
        }
    }
}

#[test]
fn door() {
    let stage = Stage::new();
    let (wire, head) = (&stage.wire, &stage.seat.head);
    let org = || wire.post("/org", json!({ "name": "acme" }), head);
    let shut = || wire.knock(&format!("/repo/{}/close", stage.repo), head);
    stage.each(BEGIN, "found", org);
    stage.each(ISSUES, "close sweep", shut);
    stage.each(BEGIN, "close batch", shut);
    stage.plant(MAIN, None, &stage.main);
    let issue = stage.issue();
    let propose = || {
        let body = json!({ "issue": issue, "base": MAIN, "head": MAIN });
        wire.post("/propose", body, head)
    };
    stage.each(ISSUES, "propose lookup", propose);
    stage.each("INSERT INTO \"repo_pull\"", "propose insert", propose);
    assert_eq!(org().status, 201, "found after the failures");
    assert_eq!(shut().status, 204, "close after the failures");
}

#[test]
fn ground() {
    let stage = Stage::new();
    assert_eq!(stage.plant(MAIN, None, &stage.main).status, 200, "seed");
    let spare = stage.wire.private(&stage.seat.head, "spare", stage.seat.id);
    let door = format!("/repo/{spare}/git");
    stage.each(REPOS, "admit", || stage.wire.get(&door, &stage.seat.head));
    assert_eq!(
        stage.wire.put(&door, &stage.seat.head).status,
        200,
        "admit after the failures"
    );
    stage.each(REFS, "ref read", || stage.get("/ref?name=refs/heads/main"));
    stage.each(BEGIN, "ref move", || {
        stage.plant(MAIN, Some(&stage.main), &stage.topic)
    });
    stage.each(VERDICTS, "verdict read", || {
        stage.get(&format!("/verdict?commit={}", stage.main))
    });
    stage.each(BEGIN, "verdict write", || {
        let body = json!({ "commit": stage.main, "context": "guard", "state": "success" });
        stage.post("/verdict", body)
    });
    stage.permit();
    stage.plant(FLAT, None, &stage.topic);
    let pull = stage.pull(FLAT);
    let weld = || {
        let door = format!("/pull/{pull}/weld");
        let body = json!({ "mode": "join" });
        stage.wire.post(&door, body, &stage.seat.head)
    };
    stage.each(PULLS, "weld lookup", weld);
    stage.each(SHIELDS, "weld shield", weld);
    assert_eq!(
        stage.get("/ref?name=refs/heads/main").body["object"],
        json!(stage.main),
        "a failed store moved a reference"
    );
}

#[test]
fn conflict() {
    let stage = Stage::new();
    assert_eq!(stage.plant(MAIN, None, &stage.main).status, 200, "seed");
    assert_eq!(
        stage.plant(FLAT, None, &stage.main).status,
        200,
        "seed flat"
    );
    let moved = stage.plant(MAIN, Some(&stage.main), &stage.topic);
    assert_eq!(moved.status, 200, "advance main");
    stage.permit();
    let pull = stage.pull(FLAT);
    let door = format!("/pull/{pull}/weld");
    let body = json!({ "mode": "forward" });
    let res = stage.wire.post(&door, body, &stage.seat.head);
    assert_eq!(res.status, 409, "repository conflict");
    assert_eq!(
        res.body,
        json!({ "error": "base is not an ancestor of head" }),
        "repository conflict body"
    );
    let stale = stage.plant(MAIN, Some(&stage.main), &stage.topic);
    assert_eq!(stale.status, 409, "stale expectation");
    assert_eq!(
        stale.body,
        json!({ "error": "reference moved under the expectation" }),
        "stale expectation body"
    );
}
