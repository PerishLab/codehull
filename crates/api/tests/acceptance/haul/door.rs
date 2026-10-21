use super::Haul;
use serde_json::json;

impl Haul {
    pub fn denied(&self) {
        assert!(
            !self
                .client
                .clone(&self.url, &self.dir("denied"), Some(&self.stranger)),
            "a stranger cloned the seat"
        );
    }

    pub fn anonymous(&self) {
        assert!(
            !self.client.clone(&self.url, &self.dir("anon"), None),
            "an unsigned caller cloned the seat"
        );
    }

    fn owned(&self, org: &str, name: &str) -> (i64, String) {
        let head = &self.seat.head;
        let org = self.wire.made("/org", json!({ "name": org }), head);
        let repo = self.wire.private(head, name, org);
        (repo, self.wire.url(&format!("/repo/{repo}/git")))
    }

    pub fn organization(&self) {
        let (owned, seam) = self.owned("guild", "held");
        let opened = self
            .wire
            .put(&format!("/repo/{owned}/git"), &self.seat.head);
        assert_eq!(
            opened.status, 200,
            "an organization's seat refused provisioning"
        );
        let work = self.work("owned");
        assert!(
            self.push(&work, &[&seam, "HEAD:refs/heads/main"]),
            "a member could not push to a repository the organization owns"
        );
        assert!(
            self.client
                .clone(&seam, &self.dir("owned-back"), Some(&self.seat)),
            "a member could not clone a repository the organization owns"
        );
        let outside = self.wire.get(
            &format!("/repo/{owned}/git/ref?name=refs/heads/main"),
            &self.stranger.head,
        );
        assert_eq!(
            outside.status, 403,
            "a stranger reached an organization's repository"
        );
    }

    pub fn archive(&self) {
        let (shut, seam) = self.owned("attic", "kept");
        let head = &self.seat.head;
        self.wire.put(&format!("/repo/{shut}/git"), head);
        let work = self.work("attic");
        assert!(
            self.push(&work, &[&seam, "HEAD:refs/heads/main"]),
            "seeding an archived candidate failed"
        );
        let sealed = self
            .wire
            .patch(&format!("/repo/{shut}"), json!({ "archived": true }), head);
        assert_eq!(sealed.status, 200, "archiving");
        let read = self
            .wire
            .get(&format!("/repo/{shut}/git/ref?name=refs/heads/main"), head);
        assert_eq!(read.status, 200, "an archived repository stopped reading");
        assert!(
            !self.push(&work, &["--force", &seam, "HEAD:refs/heads/other"]),
            "an archived repository accepted a push"
        );
        let cast = self.wire.post(
            &format!("/repo/{shut}/git/verdict"),
            json!({ "commit": self.main, "context": "guard", "state": "success" }),
            head,
        );
        assert_eq!(cast.status, 409, "an archived repository took a verdict");
    }

    pub fn close(&self) {
        let head = &self.seat.head;
        let shut = self.wire.knock(&format!("/repo/{}/close", self.repo), head);
        assert_eq!(shut.status, 204, "closing a repository with references");
        let gone = self.wire.cut(&format!("/repo/{}", self.repo), head);
        assert_eq!(gone.status, 404, "a closed repository lingered");
    }
}
