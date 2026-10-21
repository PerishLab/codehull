use super::exe;
use super::git::{Client, Source};
use super::stage::Stage;
use super::wire::Wire;
use serde_json::json;

const MAIN: &str = "refs/heads/main";
const TOPIC: &str = "refs/heads/topic";

#[test]
fn restart() {
    let mut stage = Stage::seat(exe(), false);
    let client = Client::new(&stage.nest);
    let seed = Source::new(&client, &stage.nest, true);
    let (main, topic) = (seed.main.as_str(), seed.topic.as_deref().expect("topic"));
    let wire = Wire::new(&stage.base);
    let seat = wire.join(&stage.issuer, "ground", &[]);
    let repo = wire.private(&seat.head, "truth", seat.id);
    let path = format!("/repo/{repo}/git");
    assert_eq!(wire.put(&path, &seat.head).status, 200, "provision");
    for object in [main, topic] {
        let body = json!({ "object": object, "bundle": seed.bundle });
        let res = wire.post(&format!("{path}/object"), body, &seat.head);
        assert_eq!(res.status, 200, "ingest {object}");
    }
    for (name, before, after) in [
        (MAIN, None, main),
        (TOPIC, None, topic),
        (MAIN, Some(main), topic),
    ] {
        let mut body = json!({ "name": name, "after": after });
        if let Some(before) = before {
            body["before"] = json!(before);
        }
        let res = wire.post(&format!("{path}/ref"), body, &seat.head);
        assert_eq!(res.status, 200, "advance {name}");
    }

    stage.restart();
    let wire = Wire::new(&stage.base);
    let tip = |name: &str| {
        let res = wire.get(&format!("{path}/ref?name={name}"), &seat.head);
        assert_eq!(res.status, 200, "read {name}");
        res.body["object"].clone()
    };
    assert_eq!(
        wire.get(&path, &seat.head).status,
        200,
        "seat lost across restart"
    );
    assert_eq!(tip(MAIN), topic, "main did not survive restart");
    assert_eq!(tip(TOPIC), topic, "topic did not survive restart");
}
