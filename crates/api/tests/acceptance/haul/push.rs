use super::{Haul, MAIN};

impl Haul {
    pub fn fetch(&self) {
        let into = self.dir("clone");
        assert!(
            self.client.clone(&self.url, &into, Some(&self.seat)),
            "a real git client clones what the seat holds"
        );
        assert_eq!(
            self.at(&into, &["rev-parse", "HEAD"]),
            self.main,
            "clone head"
        );
        assert_eq!(
            std::fs::read_to_string(into.join("probe")).expect("probe"),
            "main\n",
            "cloned bytes differ from the ingested bytes"
        );
    }

    pub fn advance(&self) {
        let work = self.work("work");
        let head = self.commit(&work, "probe", "pushed\n");
        assert!(
            self.push(&work, &["origin", "main"]),
            "push refused a fast-forward"
        );
        assert_eq!(self.tip(MAIN).as_deref(), Some(head.as_str()), "main reads");
        let back = self.dir("back");
        assert!(
            self.client.clone(&self.url, &back, Some(&self.seat)),
            "clone after push failed"
        );
        assert_eq!(
            std::fs::read_to_string(back.join("probe")).expect("probe"),
            "pushed\n",
            "pushed bytes did not come back"
        );
    }

    pub fn bulk(&self) {
        let work = self.work("bulk");
        let mut bulk = vec![0u8; 3 << 20];
        rand::fill(&mut bulk[..]);
        std::fs::write(work.join("bulk"), &bulk).expect("bulk file");
        self.at(&work, &["add", "bulk"]);
        self.at(&work, &["commit", "-q", "-m", "carry three megabytes"]);
        let head = self.at(&work, &["rev-parse", "HEAD"]);
        assert!(
            self.push(&work, &["origin", "main"]),
            "a three megabyte push was refused"
        );
        assert_eq!(
            self.tip(MAIN).as_deref(),
            Some(head.as_str()),
            "the bulk push did not land"
        );
    }

    pub fn stale(&self) {
        let work = self.work("stale");
        let held = self.tip(MAIN);
        self.at(&work, &["reset", "-q", "--hard", "HEAD~1"]);
        let forked = self.commit(&work, "probe", "forked\n");
        let lease = format!("--force-with-lease=refs/heads/main:{}", "0".repeat(40));
        assert!(
            !self.push(&work, &[&lease, "origin", "main"]),
            "a stale expectation was accepted"
        );
        assert_eq!(
            self.tip(MAIN),
            held,
            "a refused push still moved the reference"
        );
        assert!(
            !self.client.stored(&self.bare, &forked),
            "a lease the client refused still reached the store"
        );
    }

    pub fn pen(&self) {
        let work = self.work("pen");
        let forked = self.commit(&work, "probe", "penned\n");
        assert!(
            !self.push(&work, &["origin", "HEAD:refs/heads/a\"b"]),
            "a quoted reference name was accepted over http"
        );
        assert!(
            !self.client.stored(&self.bare, &forked),
            "a refused push left its objects in the store"
        );
        assert_eq!(self.penned(), 0, "a refused push left an object pen behind");
        assert!(
            self.push(&work, &["origin", "HEAD:refs/heads/penned"]),
            "push refused an ordinary branch"
        );
        assert!(
            self.client.stored(&self.bare, &forked),
            "an accepted push did not admit its objects"
        );
        assert_eq!(
            self.penned(),
            0,
            "an accepted push left an object pen behind"
        );
    }

    pub fn retire(&self) {
        let work = self.work("gone");
        assert!(
            self.push(&work, &["origin", "HEAD:refs/heads/spare"]),
            "push refused a new branch"
        );
        assert!(
            self.tip("refs/heads/spare").is_some(),
            "the new branch did not settle"
        );
        assert!(
            self.push(&work, &["origin", "--delete", "spare"]),
            "push refused a delete"
        );
        assert!(
            self.tip("refs/heads/spare").is_none(),
            "a deleted reference still reads"
        );
    }
}
