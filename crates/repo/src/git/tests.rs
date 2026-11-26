use super::{Draw, Git};
use crate::Error;
use radicula_process::Cancel;
use radicula_process::asynchronous::Limits;
use std::path::Path;
use std::time::Duration;
use tokio::io::AsyncReadExt;

const BACKPRESSURE: &str = "alias.probe=!dd if=/dev/zero bs=65536 count=16 >&2 2>/dev/null; cat";
const WAIT: &str = "alias.probe=!printf '%s' \"$PPID\" > pid; printf data; exec sleep 30";

#[tokio::test]
async fn backpressure() {
    let temp = home();
    let (mut source, sink) = tokio::io::duplex(8192);
    let input: Vec<u8> = (0..=255).cycle().take(1 << 20).collect();
    let git = Git::at(temp.path());
    let draw = git.draw(
        ["-c", BACKPRESSURE, "probe"],
        input.clone(),
        sink,
        request(5, 4 << 20),
    );
    let mut output = Vec::new();
    let (held, received) = tokio::join!(draw, source.read_to_end(&mut output));
    held.unwrap();
    received.unwrap();
    assert_eq!(output, input);
}

#[tokio::test]
async fn diagnostics() {
    let temp = home();
    let error = Git::at(temp.path())
        .draw(
            ["-c", "alias.probe=!echo bad-pack >&2; exit 7", "probe"],
            Vec::new(),
            tokio::io::sink(),
            request(5, 4096),
        )
        .await
        .unwrap_err();
    assert_eq!(error, Error::Git("probe: bad-pack".into()));
}

#[tokio::test]
async fn output() {
    let temp = home();
    assert_eq!(
        wait(temp.path(), request(5, 1)).await,
        Error::Io("probe: git exceeded its output limit".into())
    );
    reaped(temp.path());
}

#[tokio::test]
async fn expiry() {
    let temp = home();
    assert_eq!(
        wait(temp.path(), request(2, 4096)).await,
        Error::Io("probe: git exceeded its time limit".into())
    );
    reaped(temp.path());
}

#[tokio::test]
async fn cancellation() {
    let temp = home();
    let request = request(5, 4096);
    let cancel = request.cancel.clone();
    let path = temp.path().join("pid");
    let signal = async {
        tokio::time::timeout(Duration::from_secs(3), async {
            while !path.exists() {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        cancel.cancel();
    };
    let (error, ()) = tokio::join!(wait(temp.path(), request), signal);
    assert_eq!(error, Error::Io("probe: git was cancelled".into()));
    reaped(temp.path());
}

#[tokio::test]
async fn sink() {
    let temp = home();
    let (source, sink) = tokio::io::duplex(8192);
    drop(source);
    let error = Git::at(temp.path())
        .draw(["-c", WAIT, "probe"], Vec::new(), sink, request(5, 4096))
        .await
        .unwrap_err();
    assert!(
        matches!(&error, Error::Io(note) if note.to_lowercase().contains("broken pipe")),
        "{error}"
    );
    reaped(temp.path());
}

async fn wait(root: &Path, request: Draw<'_>) -> Error {
    Git::at(root)
        .draw(
            ["-c", WAIT, "probe"],
            Vec::new(),
            tokio::io::sink(),
            request,
        )
        .await
        .unwrap_err()
}

fn reaped(root: &Path) {
    let pid = std::fs::read_to_string(root.join("pid")).unwrap();
    let pid: u32 = pid.parse().unwrap();
    assert!(!Path::new("/proc").join(pid.to_string()).exists());
}

fn request(seconds: u64, output: u64) -> Draw<'static> {
    Draw {
        action: "probe",
        limits: Limits {
            time: Duration::from_secs(seconds),
            output,
        },
        cancel: Cancel::new(),
    }
}

fn home() -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    Git::at(temp.path())
        .success(["init", "--bare", "."], "initialize test repository")
        .unwrap();
    temp
}
