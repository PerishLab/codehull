use crate::Hall;
use russh::keys::{Algorithm, PrivateKey, PublicKey};
use russh::server::{Auth, Config, Handler, Msg, Server, Session};
use russh::{Channel, ChannelId, Preferred};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncWriteExt, copy};
use tokio::net::TcpListener;
use tokio::process::Command;

const UPLOAD: &str = "git-upload-pack";
const PACK: &str = "upload-pack";

pub fn key(hold: &Path) -> Result<PrivateKey, String> {
    if let Ok(text) = std::fs::read_to_string(hold) {
        return PrivateKey::from_openssh(&text).map_err(|error| format!("host key: {error}"));
    }
    let born = PrivateKey::random(&mut rand::rng(), Algorithm::Ed25519)
        .map_err(|error| format!("host key: {error}"))?;
    let text = born
        .to_openssh(russh::keys::ssh_key::LineEnding::LF)
        .map_err(|error| format!("host key: {error}"))?;
    if let Some(parent) = hold.parent() {
        std::fs::create_dir_all(parent).map_err(|error| format!("host key: {error}"))?;
    }
    std::fs::write(hold, text.as_bytes()).map_err(|error| format!("host key: {error}"))?;
    Ok(born)
}

pub async fn serve(hall: Arc<dyn Hall>, held: PrivateKey, at: (&str, u16)) -> Result<(), String> {
    let config = Arc::new(Config {
        inactivity_timeout: Some(Duration::from_secs(600)),
        auth_rejection_time: Duration::from_secs(1),
        keys: vec![held],
        preferred: Preferred::default(),
        ..Default::default()
    });
    let socket = TcpListener::bind(at)
        .await
        .map_err(|error| format!("ssh listen: {error}"))?;
    let mut hold = Post { hall };
    hold.run_on_socket(config, &socket)
        .await
        .map_err(|error| format!("ssh serve: {error}"))
}

struct Post {
    hall: Arc<dyn Hall>,
}

struct Seat {
    hall: Arc<dyn Hall>,
    who: Option<i64>,
    held: HashMap<ChannelId, Channel<Msg>>,
}

impl Server for Post {
    type Handler = Seat;

    fn new_client(&mut self, _: Option<std::net::SocketAddr>) -> Seat {
        Seat {
            hall: self.hall.clone(),
            who: None,
            held: HashMap::new(),
        }
    }
}

impl Handler for Seat {
    type Error = russh::Error;

    async fn auth_publickey(&mut self, _: &str, key: &PublicKey) -> Result<Auth, Self::Error> {
        let print = key.to_openssh().unwrap_or_default();
        match self.hall.admit(print).await {
            Some(who) => {
                self.who = Some(who);
                Ok(Auth::Accept)
            }
            None => Ok(Auth::reject()),
        }
    }

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        reply: russh::server::ChannelOpenHandle,
        _: &mut Session,
    ) -> Result<(), Self::Error> {
        reply.accept().await;
        self.held.insert(channel.id(), channel);
        Ok(())
    }

    async fn exec_request(
        &mut self,
        id: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let text = String::from_utf8_lossy(data).to_string();
        let Some(who) = self.who else {
            return refuse(id, session, "unauthenticated");
        };
        let Some(path) = order(&text) else {
            return refuse(id, session, "only git-upload-pack is served over ssh");
        };
        let Some(root) = self.hall.seat(who, path).await else {
            return refuse(id, session, "no such repository");
        };
        let Some(channel) = self.held.remove(&id) else {
            return refuse(id, session, "channel is absent");
        };
        session.channel_success(id)?;
        let handle = session.handle();
        tokio::spawn(async move {
            let code = pipe(channel, &root).await;
            let _ = handle.exit_status_request(id, code).await;
            let _ = handle.eof(id).await;
            let _ = handle.close(id).await;
        });
        Ok(())
    }
}

fn refuse(id: ChannelId, session: &mut Session, note: &str) -> Result<(), russh::Error> {
    session.extended_data(id, 1, format!("codehull: {note}\n").into_bytes())?;
    session.channel_failure(id)?;
    Ok(())
}

async fn pipe(channel: Channel<Msg>, root: &Path) -> u32 {
    let mut child = match Command::new("git")
        .arg(PACK)
        .arg(root)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return 1,
    };
    let (mut sink, mut source) = (
        child.stdin.take().expect("child input"),
        child.stdout.take().expect("child output"),
    );
    let (mut reader, mut writer) = tokio::io::split(channel.into_stream());
    let up = tokio::spawn(async move {
        let _ = copy(&mut reader, &mut sink).await;
    });
    let _ = copy(&mut source, &mut writer).await;
    let _ = writer.shutdown().await;
    up.abort();
    child
        .wait()
        .await
        .ok()
        .and_then(|held| held.code())
        .map(|code| code as u32)
        .unwrap_or(1)
}

fn order(text: &str) -> Option<String> {
    let rest = text.strip_prefix(UPLOAD)?.trim_start();
    let held = rest.trim_matches(['\'', '"']).trim();
    if held.is_empty() {
        return None;
    }
    Some(held.trim_start_matches('/').to_owned())
}
