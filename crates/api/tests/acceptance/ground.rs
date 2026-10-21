use super::exe;
use super::git::{Client, Source};
use super::stage::Stage;
use super::wire::{Seat, Wire};
use serde_json::{Value, json};

const MAIN: &str = "refs/heads/main";
const TOPIC: &str = "refs/heads/topic";

struct Truth {
    wire: Wire,
    seat: Seat,
    repo: i64,
}

impl Truth {
    fn path(&self, tail: &str) -> String {
        format!("/repo/{}/git{tail}", self.repo)
    }

    fn tip(&self, name: &str) -> Value {
        let res = self
            .wire
            .get(&self.path(&format!("/ref?name={name}")), &self.seat.head);
        assert_eq!(res.status, 200, "read {name}");
        res.body["object"].clone()
    }

    fn shift(&self, name: &str, before: Option<&str>, after: &str) -> u16 {
        let mut body = json!({ "name": name, "after": after });
        if let Some(before) = before {
            body["before"] = json!(before);
        }
        self.wire
            .post(&self.path("/ref"), body, &self.seat.head)
            .status
    }
}

#[test]
fn ground() {
    let mut stage = Stage::seat(exe(), false);
    let client = Client::new(&stage.nest);
    let seed = Source::new(&client, &stage.nest, true);
    let (main, topic) = (seed.main.as_str(), seed.topic.as_deref().expect("topic"));
    let wire = Wire::new(&stage.base);
    let seat = wire.join(&stage.issuer, "ground", &[]);
    let stranger = wire.join(&stage.issuer, "drifter", &[]);
    let repo = wire.private(&seat.head, "truth", seat.id);
    let truth = Truth { wire, seat, repo };
    let (wire, head) = (&truth.wire, &truth.seat.head);

    let born = wire.put(&truth.path(""), head);
    assert_eq!(born.status, 200, "provision");
    let again = wire.put(&truth.path(""), head);
    assert_eq!(again.status, 200, "provision is not idempotent");

    for object in [main, topic] {
        let body = json!({ "object": object, "bundle": seed.bundle });
        let res = wire.post(&truth.path("/object"), body, head);
        assert_eq!(res.status, 200, "ingest {object}");
    }

    assert_eq!(truth.shift(MAIN, None, main), 200, "advance main");
    assert_eq!(truth.shift(TOPIC, None, topic), 200, "advance topic");
    assert_eq!(truth.tip(MAIN), main, "main did not seed");

    assert_eq!(
        truth.shift(MAIN, Some(topic), topic),
        409,
        "stale expectation resolved"
    );
    assert_eq!(
        truth.shift(MAIN, None, topic),
        409,
        "expected absence resolved over a live ref"
    );
    assert_eq!(truth.tip(MAIN), main, "a refused move still wrote");

    for tail in ["", "/ref?name=refs/heads/main"] {
        let res = wire.get(&truth.path(tail), &stranger.head);
        assert_eq!(res.status, 403, "stranger reached {tail}");
    }

    assert_eq!(truth.shift(MAIN, Some(main), topic), 200, "advance main");

    let retire = truth.path("/ref?name=refs/heads/topic");
    assert_eq!(wire.cut(&retire, head).status, 204, "retire");
    assert!(
        truth.tip(TOPIC).is_null(),
        "a retired reference still reads"
    );
    assert_eq!(wire.cut(&retire, head).status, 404, "retiring twice");
    let offer = wire.get(&truth.path("/info/refs?service=git-upload-pack"), head);
    assert!(
        !String::from_utf8_lossy(&offer.bytes).contains(TOPIC),
        "a retired reference is still advertised"
    );
    assert_eq!(truth.shift(TOPIC, None, topic), 200, "advance topic");
    assert_eq!(
        truth.tip(TOPIC),
        topic,
        "a released name did not accept a new reference"
    );

    stage.restart();
    let truth = Truth {
        wire: Wire::new(&stage.base),
        ..truth
    };
    let head = &truth.seat.head;
    assert_eq!(
        truth.wire.get(&truth.path(""), head).status,
        200,
        "seat lost across restart"
    );
    assert_eq!(truth.tip(MAIN), topic, "main did not survive restart");
    assert_eq!(truth.tip(TOPIC), topic, "topic did not survive restart");
}
