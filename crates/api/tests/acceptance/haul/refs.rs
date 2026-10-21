use super::Haul;
use serde_json::json;

impl Haul {
    pub fn tags(&self) {
        let work = self.work("tags");
        self.at(&work, &["tag", "plain"]);
        self.at(&work, &["tag", "-a", "signed", "-m", "an annotated tag"]);
        assert!(
            self.push(&work, &["origin", "plain", "signed"]),
            "push refused tags"
        );
        let plain = self.at(&work, &["rev-parse", "plain"]);
        let signed = self.at(&work, &["rev-parse", "signed"]);
        assert_eq!(
            self.tip("refs/tags/plain"),
            Some(plain),
            "a plain tag did not settle"
        );
        assert_eq!(
            self.tip("refs/tags/signed"),
            Some(signed),
            "an annotated tag did not settle at its tag object"
        );
        let back = self.dir("tagged");
        assert!(
            self.client.clone(&self.url, &back, Some(&self.seat)),
            "clone after tags failed"
        );
        let seen = self.at(&back, &["tag", "--list"]);
        assert!(
            seen.contains("plain") && seen.contains("signed"),
            "clone did not bring the tags back: {seen}"
        );
        assert!(
            self.push(&work, &["origin", "--delete", "plain"]),
            "push refused a tag delete"
        );
        assert!(
            self.tip("refs/tags/plain").is_none(),
            "a deleted tag still reads"
        );
    }

    pub fn namespace(&self) {
        let work = self.work("notes");
        assert!(
            self.push(&work, &["origin", "HEAD:refs/notes/probe"]),
            "push refused an ordinary git namespace"
        );
        let head = self.at(&work, &["rev-parse", "HEAD"]);
        assert_eq!(
            self.tip("refs/notes/probe"),
            Some(head),
            "the reference did not settle"
        );
        let back = self.dir("noted");
        assert!(
            self.client.clone(&self.url, &back, Some(&self.seat)),
            "clone after the namespace push failed"
        );
        assert!(
            self.client.fetch(
                &back,
                &self.seat,
                &["origin", "refs/notes/probe:refs/notes/probe"]
            ),
            "fetch refused the namespace it had just accepted"
        );
    }

    pub fn quoted(&self) {
        let body = json!({ "name": "refs/heads/a\"b", "after": self.main });
        let res = self.wire.post(&self.path("/ref"), body, &self.seat.head);
        assert_eq!(res.status, 400, "a quoted reference name");
    }
}
