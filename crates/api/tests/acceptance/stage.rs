#![allow(dead_code)]

pub use super::issuer::Issuer;
use serde_json::Value;
use std::io::{BufRead, BufReader, Lines};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStderr, Command, Stdio};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

struct Unit;

static SERIAL: Mutex<Unit> = Mutex::new(Unit);

const MEMORY: &str = "[listen]\nhost = \"127.0.0.1\"\nport = 0\nprefix = \"\"\n\n[store]\nkind = \"memory\"\n\n[cache]\nkind = \"memory\"\n";

pub struct Stage {
    pub origin: String,
    pub base: String,
    pub nest: PathBuf,
    pub issuer: Issuer,
    pub blob: bool,
    pub ssh: Option<u16>,
    exe: PathBuf,
    env: Vec<(String, String)>,
    log: Arc<Mutex<String>>,
    child: Option<Child>,
    shell: bool,
    room: tempfile::TempDir,
    hold: Option<MutexGuard<'static, Unit>>,
}

impl Stage {
    pub fn live(exe: &Path) -> Self {
        let store = std::env::var("API_STORE_URL")
            .ok()
            .filter(|url| !url.is_empty());
        let shed = std::env::var("API_BLOB_ENDPOINT")
            .ok()
            .filter(|url| !url.is_empty());
        let hold = store
            .as_ref()
            .map(|_| SERIAL.lock().unwrap_or_else(|poison| poison.into_inner()));
        let mut env = Vec::new();
        if let Some(url) = store {
            env.push(("API_STORE_KIND".to_string(), "pg".to_string()));
            env.push(("API_STORE_URL".to_string(), url));
            env.push(("API_FRESH".to_string(), "true".to_string()));
        }
        if let Some(url) = &shed {
            env.push(("API_BLOB_ENDPOINT".to_string(), url.clone()));
            env.push((
                "API_BLOB_KEY".to_string(),
                lookup("API_BLOB_KEY", "codehull"),
            ));
            env.push((
                "API_BLOB_SECRET".to_string(),
                lookup("API_BLOB_SECRET", "codehull123"),
            ));
        }
        let mut stage = Self::raise(exe, MEMORY, env);
        stage.hold = hold;
        stage.blob = shed.is_some();
        stage
    }

    pub fn external(exe: &Path, issuer: &str) -> Self {
        let env = vec![("API_OIDC_ISSUER".to_string(), issuer.to_string())];
        Self::raise(exe, MEMORY, env)
    }

    pub fn seat(exe: &Path, ssh: bool) -> Self {
        let mut config = "[listen]\nhost = \"127.0.0.1\"\nport = 0\nprefix = \"\"\n\n[store]\nkind = \"file\"\npath = \"codehull.sqlite\"\n\n[cache]\nkind = \"memory\"\n\n[repo]\npath = \"seats\"\n".to_string();
        if ssh {
            config.push_str("\n[ssh]\nkey = \"host.key\"\nhost = \"127.0.0.1\"\nport = 0\n");
        }
        Self::raise(exe, &config, Vec::new())
    }

    fn raise(exe: &Path, config: &str, env: Vec<(String, String)>) -> Self {
        let room = tempfile::tempdir().expect("stage room");
        let nest = room.path().to_path_buf();
        std::fs::write(nest.join("codehull.toml"), config).expect("stage config");
        let issuer = Issuer::new();
        let log = Arc::new(Mutex::new(String::new()));
        let mut stage = Self {
            base: String::new(),
            origin: String::new(),
            nest,
            issuer,
            blob: false,
            ssh: None,
            exe: exe.to_path_buf(),
            env,
            log,
            child: None,
            shell: config.contains("[ssh]"),
            room,
            hold: None,
        };
        if config.contains("[repo]") {
            let done = stage
                .command()
                .arg("bootstrap")
                .arg(&stage.nest)
                .stdout(Stdio::null())
                .output()
                .expect("bootstrap process");
            assert!(
                done.status.success(),
                "{}",
                String::from_utf8_lossy(&done.stderr)
            );
        }
        stage.restart();
        stage
    }

    pub fn restart(&mut self) {
        self.stop();
        let mut child = self
            .command()
            .arg("serve")
            .arg(&self.nest)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("serve process");
        let pipe = child.stderr.take().expect("serve stderr");
        self.child = Some(child);
        self.listen(pipe);
        self.ready();
    }

    fn listen(&mut self, pipe: ChildStderr) {
        let mut lines = BufReader::new(pipe).lines();
        let (mut api, mut ssh) = (None, None);
        while let Some(Ok(line)) = lines.next() {
            self.note(&line);
            let seen: Value = serde_json::from_str(&line).unwrap_or(Value::Null);
            let at = seen["endpoint"].as_str().map(str::to_string);
            let role = seen["role"].as_str();
            if role == Some("ssh") && at.is_none() {
                panic!("ssh exited before listening: {line}");
            }
            match role {
                Some("api") => api = at,
                Some("ssh") => ssh = at,
                _ => {}
            }
            if let Some(at) = api.as_deref()
                && (ssh.is_some() || !self.shell)
            {
                self.origin = at.to_string();
                self.base = format!("{at}/api");
                self.ssh = ssh.as_deref().map(tail);
                drain(self.log.clone(), lines);
                return;
            }
        }
        self.stop();
        panic!("serve exited before listening: {}", self.text());
    }

    fn note(&self, line: &str) {
        note(&self.log, line);
    }

    pub fn crown(&self) -> Vec<(String, String)> {
        for _ in 0..40 {
            let text = self.text();
            if let Some(token) = text
                .split("sudo token ")
                .nth(1)
                .and_then(|rest| rest.split_whitespace().next())
            {
                return vec![("authorization".into(), format!("sudo {token}"))];
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        panic!("no sudo token in boot log");
    }

    pub fn text(&self) -> String {
        self.log
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone()
    }

    fn command(&self) -> Command {
        let mut command = Command::new(&self.exe);
        command
            .env_clear()
            .env("HOME", &self.nest)
            .env("USERPROFILE", &self.nest)
            .env("LOCALAPPDATA", &self.nest)
            .env("API_OIDC_ISSUER", &self.issuer.url)
            .env("API_OIDC_AUDIENCE", "codehull")
            .envs(self.env.iter().map(|(key, value)| (key, value)));
        for key in ["PATH", "SYSTEMROOT", "WINDIR"] {
            if let Some(value) = std::env::var_os(key) {
                command.env(key, value);
            }
        }
        command
    }

    fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    fn ready(&mut self) {
        let url = format!("{}/health", self.base);
        for _ in 0..120 {
            if ureq::get(&url).call().is_ok() {
                return;
            }
            if let Some(Ok(Some(code))) = self.child.as_mut().map(Child::try_wait) {
                panic!("api exited {code}: {}", self.text());
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        panic!("timeout {url}: {}", self.text());
    }
}

impl Drop for Stage {
    fn drop(&mut self) {
        self.stop();
        if std::thread::panicking() {
            eprintln!("api log:\n{}", self.text());
        }
    }
}

fn note(log: &Mutex<String>, line: &str) {
    let mut held = log.lock().unwrap_or_else(|poison| poison.into_inner());
    held.push_str(line);
    held.push('\n');
}

fn drain(log: Arc<Mutex<String>>, lines: Lines<BufReader<ChildStderr>>) {
    std::thread::spawn(move || {
        for line in lines.map_while(Result::ok) {
            note(&log, &line);
        }
    });
}

fn tail(at: &str) -> u16 {
    at.rsplit(':')
        .next()
        .and_then(|port| port.parse().ok())
        .unwrap_or_else(|| panic!("unreadable endpoint {at}"))
}

fn lookup(key: &str, fallback: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| fallback.to_string())
}
