use super::{Haul, MAIN};
use serde_json::json;
use std::path::Path;

impl Haul {
    fn merge(&self, mode: &str, head: &str) -> String {
        let body = json!({ "base": MAIN, "head": format!("refs/heads/{head}"), "mode": mode });
        let res = self.wire.post(&self.path("/merge"), body, &self.seat.head);
        assert_eq!(res.status, 200, "merge {mode}");
        res.body["object"]
            .as_str()
            .expect("merge object")
            .to_string()
    }

    fn branch(&self, work: &Path, name: &str, from: &[&str]) -> String {
        let mut args = vec!["checkout", "-q", "-b", name];
        args.extend(from);
        self.at(work, &args);
        let made = self.commit(work, name, &format!("{name}\n"));
        assert!(
            self.push(work, &["origin", name]),
            "push refused the {name} branch"
        );
        made
    }

    fn parents(&self, work: &Path, object: &str) -> Vec<String> {
        self.at(work, &["rev-list", "--parents", "-n", "1", object])
            .split_whitespace()
            .skip(1)
            .map(str::to_string)
            .collect()
    }

    pub fn welded(&self, pull: i64) -> String {
        let pack = self
            .wire
            .query(&format!("from Pull where id = \"{pull}\""), &self.seat.head);
        let rows = pack.roots();
        assert_eq!(rows.len(), 1, "the proposal row is missing");
        rows[0]["weld"].as_str().unwrap_or_default().to_string()
    }

    pub fn propose(&self, branch: &str) -> i64 {
        let issue = json!({
            "title": format!("PR: {branch}"),
            "body": "",
            "closed": false,
            "repo": self.repo,
            "author": self.seat.id,
        });
        let issue = self.wire.made("/issue", issue, &self.seat.head);
        let body = json!({ "issue": issue, "base": MAIN, "head": format!("refs/heads/{branch}") });
        self.wire.made("/propose", body, &self.seat.head)
    }

    pub fn merges(&self) {
        for mode in ["forward", "join", "squash"] {
            let made = self.wire.one(
                "Weld",
                json!({ "mode": mode, "repo": self.repo }),
                &self.seat.head,
            );
            assert_eq!(made.status, 200, "permitting {mode}");
        }
        let work = self.work("weld");
        let trunk = self.at(&work, &["rev-parse", "HEAD"]);
        let ahead = self.branch(&work, "ahead", &[]);
        assert_eq!(
            self.merge("forward", "ahead"),
            ahead,
            "a forward merge did not move main to head"
        );

        self.branch(&work, "side", &[&trunk]);
        let joined = self.merge("join", "side");
        let back = self.dir("joined");
        assert!(
            self.client.clone(&self.url, &back, Some(&self.seat)),
            "clone after join failed"
        );
        assert_eq!(
            self.parents(&back, &joined).len(),
            2,
            "a join did not produce two parents"
        );
        assert_eq!(
            std::fs::read_to_string(back.join("side")).expect("side"),
            "side\n",
            "the joined tree lost one side"
        );

        self.branch(&work, "flat", &[&trunk]);
        let flat = self.merge("squash", "flat");
        assert!(
            self.client
                .fetch(&back, &self.seat, &["-q", "origin", "main"]),
            "fetch after squash failed"
        );
        assert_eq!(
            self.parents(&back, &flat).len(),
            1,
            "a squash did not produce one parent"
        );
    }

    pub fn proposal(&self) {
        let work = self.work("proposal");
        let offered = self.branch(&work, "proposed", &[]);
        let pull = self.propose("proposed");
        let held = self.tip(MAIN);
        let door = format!("/pull/{pull}/weld");
        let stale = self.wire.post(
            &door,
            json!({ "mode": "join", "expect": "0".repeat(40) }),
            &self.seat.head,
        );
        assert_eq!(stale.status, 409, "a stale expectation");
        assert_eq!(
            self.tip(MAIN),
            held,
            "a refused proposal still moved the base"
        );
        assert_eq!(self.welded(pull), "", "a refused proposal recorded a merge");

        let done = self.wire.post(
            &door,
            json!({ "mode": "join", "expect": offered }),
            &self.seat.head,
        );
        assert_eq!(done.status, 200, "merging a proposal");
        let object = done.body["object"].as_str().expect("object").to_string();
        assert_eq!(
            self.tip(MAIN).as_deref(),
            Some(object.as_str()),
            "the base did not move to the merge"
        );
        assert_eq!(
            self.welded(pull),
            object,
            "the proposal does not carry the commit it produced"
        );
        let back = self.dir("proposed-back");
        assert!(
            self.client.clone(&self.url, &back, Some(&self.seat)),
            "clone after the proposal merge failed"
        );
        assert!(
            self.parents(&back, &object).contains(&offered),
            "the merge does not carry the proposed head as a parent"
        );
    }

    pub fn unpermitted(&self) {
        let held = self.tip(MAIN);
        let body = json!({ "base": MAIN, "head": "refs/heads/side", "mode": "rebase" });
        let res = self.wire.post(&self.path("/merge"), body, &self.seat.head);
        assert_eq!(res.status, 400, "an unknown mode");
        assert_eq!(self.tip(MAIN), held, "a refused merge still moved main");
    }
}
