use super::line;
use super::point::Point;
use super::take::{CAPS, RECEIVE, sent};
use super::{Dock, Fault, Reach, actor, admit, bad, thaw, work};
use axum::Extension;
use axum::body::{Body, Bytes};
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::response::Response;
use futures_core::Stream;
use keel::{Operator, Wire};
use serde::Deserialize;
use std::io::Write;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::mpsc::{Receiver, Sender, channel};

const FLOW: usize = 8;

const OFFER: &str = "application/x-git-upload-pack-advertisement";
const REPLY: &str = "application/x-git-upload-pack-result";
const UPLOAD: &str = "git-upload-pack";
const TAKE: &str = "application/x-git-receive-pack-advertisement";

#[derive(Deserialize)]
pub(super) struct Service {
    service: String,
}

pub(super) async fn refs<W: Wire + 'static>(
    State(dock): State<Dock<W>>,
    Path(id): Path<i64>,
    op: Option<Extension<Operator>>,
    Query(query): Query<Service>,
) -> Result<Response, Fault> {
    let who = actor(op)?;
    let reach = match query.service.as_str() {
        RECEIVE => Reach::Edit,
        _ => Reach::See,
    };
    let id = admit(&dock, id, who, reach).await?;
    let point = Point {
        dock: &dock,
        who,
        id,
    };
    point.align().await?;
    if query.service == RECEIVE {
        let listing = point.listing().await?;
        return Ok(sent(TAKE, line::offer(RECEIVE, &listing, CAPS)));
    }
    if query.service != UPLOAD {
        return Err(bad(format!("unsupported service {}", query.service)));
    }
    let store = dock.store.clone();
    let held = work(move || store.repository(id)?.advertise()).await?;
    let mut body = line::pkt(&format!("# service={UPLOAD}\n"));
    body.extend_from_slice(&line::flush());
    body.extend_from_slice(&held);
    Ok(sent(OFFER, body))
}

pub(super) async fn upload<W: Wire + 'static>(
    State(dock): State<Dock<W>>,
    Path(id): Path<i64>,
    op: Option<Extension<Operator>>,
    headers: HeaderMap,
    want: Bytes,
) -> Result<Response, Fault> {
    let id = admit(&dock, id, actor(op)?, Reach::See).await?;
    let want = thaw(&headers, want)?;
    let store = dock.store.clone();
    let (tx, rx) = channel(FLOW);
    let seat = tx.clone();
    tokio::task::spawn_blocking(move || {
        let mut sink = Spout(seat);
        let held = store
            .repository(id)
            .and_then(|repo| repo.upload(&want, &mut sink));
        if let Err(error) = held {
            let _ = tx.blocking_send(Err(std::io::Error::other(error.to_string())));
        }
    });
    Ok(sent(REPLY, Body::from_stream(Flow(rx))))
}

struct Spout(Sender<Result<Bytes, std::io::Error>>);

impl Write for Spout {
    fn write(&mut self, held: &[u8]) -> std::io::Result<usize> {
        match self.0.blocking_send(Ok(Bytes::copy_from_slice(held))) {
            Ok(()) => Ok(held.len()),
            Err(_) => Err(std::io::Error::other("the reader is gone")),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct Flow(Receiver<Result<Bytes, std::io::Error>>);

impl Stream for Flow {
    type Item = Result<Bytes, std::io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.0.poll_recv(cx)
    }
}
