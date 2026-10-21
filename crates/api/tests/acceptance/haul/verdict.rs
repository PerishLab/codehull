use super::super::wire::Head;
use super::{Haul, MAIN};
use flate2::Compression;
use flate2::write::GzEncoder;
use serde_json::{Value, json};
use std::io::Write;

impl Haul {
    pub fn judge(&self, commit: &str, context: &str, state: &str) -> u16 {
        let body = json!({ "commit": commit, "context": context, "state": state });
        self.wire
            .post(&self.path("/verdict"), body, &self.seat.head)
            .status
    }

    fn combined(&self, commit: &str) -> Value {
        let res = self.wire.get(
            &self.path(&format!("/verdict?commit={commit}")),
            &self.seat.head,
        );
        assert_eq!(res.status, 200, "verdict read");
        res.body
    }

    pub fn shield(&self) {
        let work = self.work("gated");
        self.at(&work, &["checkout", "-q", "-b", "gated"]);
        let offered = self.commit(&work, "gated", "gated\n");
        assert!(
            self.push(&work, &["origin", "gated"]),
            "push refused the gated branch"
        );
        let head = &self.seat.head;
        let shield = self.wire.made(
            "/shield",
            json!({ "branch": "main", "force": false, "approvals": 1, "repo": self.repo }),
            head,
        );
        self.wire.made(
            "/demand",
            json!({ "context": "guard", "shield": shield }),
            head,
        );
        let pull = self.propose("gated");
        let held = self.tip(MAIN);
        let door = format!("/pull/{pull}/weld");
        let weld = || self.wire.post(&door, json!({ "mode": "join" }), head);
        assert_eq!(weld().status, 412, "an unjudged head merged");
        assert_eq!(
            self.judge(&offered, "guard", "failure"),
            200,
            "casting failure"
        );
        assert_eq!(weld().status, 412, "a failing verdict merged");
        assert_eq!(
            self.judge(&offered, "guard", "success"),
            200,
            "casting success"
        );
        assert_eq!(weld().status, 412, "a proposal with no approval merged");
        assert_eq!(
            self.tip(MAIN),
            held,
            "a refused proposal still moved the base"
        );
        self.wire.made(
            "/review",
            json!({ "state": "approve", "body": "lgtm", "pull": pull, "reviewer": self.seat.id }),
            head,
        );
        let done = weld();
        assert_eq!(done.status, 200, "a cleared proposal");
        assert_eq!(
            self.tip(MAIN).as_deref(),
            done.body["object"].as_str(),
            "a cleared proposal did not move the base"
        );
    }

    pub fn verdicts(&self) {
        let head = self.tip(MAIN).expect("main is absent");
        let empty = self.combined(&head);
        assert!(
            empty["state"].is_null() && empty["verdicts"].as_array().is_some_and(Vec::is_empty),
            "a commit with no verdicts is not empty"
        );
        assert_eq!(
            self.judge(&head, "guard", "pending"),
            200,
            "posting a verdict failed"
        );
        assert_eq!(
            self.combined(&head)["state"],
            "pending",
            "one pending verdict did not roll up to pending"
        );
        assert_eq!(
            self.judge(&head, "guard", "success"),
            200,
            "replacing a verdict failed"
        );
        let once = self.combined(&head);
        assert_eq!(once["state"], "success", "replacing rolled up wrong");
        assert_eq!(
            once["verdicts"].as_array().map(Vec::len),
            Some(1),
            "replacing left extras"
        );
        assert_eq!(
            self.judge(&head, "ship", "failure"),
            200,
            "posting a second context failed"
        );
        let both = self.combined(&head);
        assert_eq!(both["state"], "failure", "two contexts rolled up wrong");
        assert_eq!(
            both["verdicts"].as_array().map(Vec::len),
            Some(2),
            "two contexts"
        );
        assert_eq!(
            self.judge(&head, "guard", "flaky"),
            400,
            "an unknown state was accepted"
        );
        let blind = self.wire.get(
            &self.path(&format!("/verdict?commit={head}")),
            &self.stranger.head,
        );
        assert_eq!(blind.status, 403, "a stranger read verdicts");
    }

    pub fn compressed(&self) {
        let head = self.tip(MAIN).expect("main is absent");
        let line = format!("want {head} side-band-64k thin-pack ofs-delta agent=act\n");
        let body = format!("{:04x}{line}00000009done\n", line.len() + 4);
        let mut squeeze = GzEncoder::new(Vec::new(), Compression::default());
        squeeze.write_all(body.as_bytes()).expect("gzip");
        let packed = squeeze.finish().expect("gzip");
        let mut ask: Head = vec![
            (
                "content-type".into(),
                "application/x-git-upload-pack-request".into(),
            ),
            ("content-encoding".into(), "gzip".into()),
        ];
        ask.extend(self.seat.head.iter().cloned());
        let target = format!("{}/git-upload-pack", self.url);
        let asked = self.wire.carry(&target, &ask, &packed);
        assert_eq!(asked.status, 200, "a compressed request");
        assert!(
            asked.bytes.len() >= 100,
            "a compressed request came back without a pack"
        );
        let lying = self.wire.carry(&target, &ask, body.as_bytes());
        assert_eq!(lying.status, 400, "a body that is not gzip");
    }

    pub fn thief(&self) {
        let work = self.dir("thief");
        assert!(
            self.client.clone(&self.url, &work, Some(&self.seat)),
            "clone for stranger push failed"
        );
        assert!(
            !self.client.push(&work, &self.stranger, &["origin", "main"]),
            "a stranger pushed the seat"
        );
    }
}

#[test]
fn shield() {
    let haul = Haul::new();
    haul.permit();
    haul.shield();
}

#[test]
fn verdicts() {
    Haul::new().verdicts();
}

#[test]
fn compressed() {
    Haul::new().compressed();
}

#[test]
fn thief() {
    Haul::new().thief();
}
