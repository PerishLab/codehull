use crate::Hall;
use std::path::Path;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, copy};
use tokio::process::Command;

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
        if index(&mut reader, &root, &hold).await.is_err() {
            let _ = tokio::fs::remove_dir_all(&hold).await;
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

async fn index<R: AsyncRead + Unpin>(reader: &mut R, root: &Path, pen: &Path) -> Result<(), ()> {
    let mut peek = [0u8; 1];
    if reader.read(&mut peek).await.map_err(drop)? == 0 {
        return Ok(());
    }
    let mut child = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["index-pack", "--stdin", "--fix-thin"])
        .env(SEAT, pen)
        .env(SPARE, root.join(OBJECTS))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(drop)?;
    let mut sink = child.stdin.take().ok_or(())?;
    sink.write_all(&peek).await.map_err(drop)?;
    copy(reader, &mut sink).await.map_err(drop)?;
    sink.shutdown().await.map_err(drop)?;
    drop(sink);
    match child.wait().await.map_err(drop)?.success() {
        true => Ok(()),
        false => Err(()),
    }
}
