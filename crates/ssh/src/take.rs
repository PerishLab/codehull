use crate::Hall;
use radicula_process::Cancel;
use radicula_process::asynchronous::{Limits, Runner, State, Streams};
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, Sink};

const SEAT: &str = "GIT_OBJECT_DIRECTORY";
const SPARE: &str = "GIT_ALTERNATE_OBJECT_DIRECTORIES";
const OBJECTS: &str = "objects";

pub(crate) async fn take<S>(hall: Arc<dyn Hall>, who: i64, path: String, stream: S) -> u32
where
    S: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    let Some(root) = hall.seat(who, path.clone()).await else {
        return 1;
    };
    let Some(offer) = hall.offer(who, path.clone()).await else {
        return 1;
    };
    let (mut reader, mut writer) = tokio::io::split(stream);
    if writer.write_all(&offer).await.is_err() {
        return 1;
    }
    let Ok(orders) = orders(&mut reader).await else {
        return 1;
    };
    let mut pen = None;
    if wanted(&orders) {
        let Some(hold) = hall.pen(who, path.clone()).await else {
            return 1;
        };
        if let Err(settled) = index(reader, &root, &hold).await {
            if settled {
                let _ = tokio::fs::remove_dir_all(&hold).await;
            }
            return 1;
        }
        pen = Some(hold);
    }
    let report = hall.apply(who, path, orders, pen).await;
    if writer.write_all(&report).await.is_err() {
        return 1;
    }
    let _ = writer.shutdown().await;
    0
}

fn wanted(orders: &[String]) -> bool {
    orders.iter().any(|text| {
        text.split(' ')
            .nth(1)
            .is_some_and(|new| !new.bytes().all(|byte| byte == b'0'))
    })
}

async fn orders<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Vec<String>, ()> {
    let mut held = Vec::new();
    loop {
        let mut head = [0u8; 4];
        reader.read_exact(&mut head).await.map_err(drop)?;
        let text = str::from_utf8(&head).map_err(drop)?;
        let size = usize::from_str_radix(text, 16).map_err(drop)?;
        if size == 0 {
            return Ok(held);
        }
        if size < 4 {
            return Err(());
        }
        let mut body = vec![0u8; size - 4];
        reader.read_exact(&mut body).await.map_err(drop)?;
        let line = String::from_utf8(body).map_err(drop)?;
        let head = line.split('\0').next().unwrap_or_default().trim_end();
        held.push(head.to_owned());
    }
}

async fn index<R: AsyncRead + Send + Unpin + 'static>(
    mut reader: R,
    root: &Path,
    pen: &Path,
) -> Result<(), bool> {
    let mut peek = [0u8; 1];
    if reader.read(&mut peek).await.map_err(|_| true)? == 0 {
        return Ok(());
    }
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(root)
        .args(["index-pack", "--stdin", "--fix-thin"])
        .env(SEAT, pen)
        .env(SPARE, root.join(OBJECTS));
    let streams = Streams::<_, Sink, Sink> {
        input: Some(std::io::Cursor::new(peek).chain(reader)),
        stdout: None,
        stderr: None,
    };
    let mut session = runner()?.start(command, streams, Cancel::new());
    match session.wait().await {
        Ok(held) if held.status.success() => Ok(()),
        _ => Err(session.observer().state() == State::Settled),
    }
}

fn runner() -> Result<Runner, bool> {
    let limits = Limits {
        time: Duration::from_secs(600),
        output: 0,
    };
    let held = if cfg!(target_os = "linux") {
        Runner::group(limits)
    } else {
        Runner::new(limits)
    };
    held.map_err(|_| true)
}
