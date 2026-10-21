use super::{Haul, MAIN};
use russh::keys::ssh_key::LineEnding;
use russh::keys::{Algorithm, PrivateKey};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn present(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

fn over(key: &Path) -> String {
    format!(
        "core.sshCommand=ssh -i {} -o IdentitiesOnly=yes -o BatchMode=yes -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o LogLevel=ERROR",
        key.display()
    )
}

impl Haul {
    fn forge(&self, name: &str) -> (PathBuf, String) {
        let mut born =
            PrivateKey::random(&mut rand::rng(), Algorithm::Ed25519).expect("client key");
        born.set_comment("act");
        let key = self.dir(name);
        let text = born
            .to_openssh(LineEnding::LF)
            .expect("openssh private key");
        std::fs::write(&key, text.as_bytes()).expect("client key file");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600))
                .expect("client key mode");
        }
        let print = born.public_key().to_openssh().expect("openssh public key");
        (key, print)
    }

    fn ssh(&self, config: &str, work: &Path, args: &[&str]) -> bool {
        let work = work.to_string_lossy();
        let mut all = vec!["-c", config, "-C", &work];
        all.extend(args);
        self.client.status(&all)
    }

    pub fn shell(&self) {
        if !present("ssh") {
            eprintln!("ssh absent: the ssh plane stays unproven this run");
            return;
        }
        let (key, print) = self.forge("client");
        let made = self.wire.one(
            "actor:key",
            json!({ "title": "act", "print": print, "kind": "ssh", "owner": self.seat.id }),
            &self.seat.head,
        );
        assert_eq!(made.status, 200, "registering a public key");
        let config = over(&key);
        let port = self.stage.ssh.expect("ssh endpoint");
        let remote = format!("ssh://git@127.0.0.1:{port}/hauler/haul");
        let into = self.dir("over-ssh");
        let target = into.to_string_lossy();
        assert!(
            self.client
                .status(&["-c", &config, "clone", "--quiet", &remote, &target]),
            "ssh clone failed"
        );
        assert_eq!(
            Some(self.at(&into, &["rev-parse", "HEAD"])),
            self.tip(MAIN),
            "ssh clone head"
        );
        self.client.sign(&into);
        let moved = self.commit(&into, "over", "over ssh\n");
        assert!(
            self.ssh(&config, &into, &["push", "origin", "main"]),
            "ssh push"
        );
        assert_eq!(self.tip(MAIN), Some(moved), "an ssh push did not move main");
        assert!(
            self.ssh(&config, &into, &["push", "origin", "--delete", "flat"]),
            "ssh delete"
        );
        assert!(
            self.tip("refs/heads/flat").is_none(),
            "an ssh delete left the reference reading"
        );
        assert!(
            !self.ssh(
                &config,
                &into,
                &["push", "origin", "HEAD:refs/heads/main~bad"]
            ),
            "an invalid reference name was accepted over ssh"
        );
        let forked = self.commit(&into, "over", "penned over ssh\n");
        assert!(
            !self.ssh(&config, &into, &["push", "origin", "HEAD:refs/heads/a\"b"]),
            "a quoted reference name was accepted over ssh"
        );
        assert!(
            !self.client.stored(&self.bare, &forked),
            "a refused ssh push left its objects in the store"
        );
        assert_eq!(
            self.penned(),
            0,
            "a refused ssh push left an object pen behind"
        );
        assert!(
            self.ssh(&config, &into, &["push", "origin", "HEAD:refs/heads/over"]),
            "an ordinary ssh push"
        );
        assert!(
            self.client.stored(&self.bare, &forked),
            "an accepted ssh push did not admit its objects"
        );
        assert_eq!(
            self.penned(),
            0,
            "an accepted ssh push left an object pen behind"
        );
        let stray = over(&self.forge("stray").0);
        let other = self.dir("no-key").to_string_lossy().into_owned();
        assert!(
            !self
                .client
                .status(&["-c", &stray, "clone", "--quiet", &remote, &other]),
            "an unregistered key cloned over ssh"
        );
    }
}

#[test]
fn shell() {
    let haul = Haul::new();
    haul.plant("refs/heads/flat");
    haul.shell();
}
