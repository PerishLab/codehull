use super::line;
use super::point::Point;
use super::take::{CAPS, RECEIVE, sent};
use super::{Dock, Fault, Reach, actor, admit, bad, thaw, work};
use axum::Extension;
use axum::body::{Body, Bytes};
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::response::Response;
use codehull_repo::Error;
use futures_core::Stream;
use keel::{Operator, Wire};
use serde::Deserialize;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{DuplexStream, duplex};
use tokio::task::JoinHandle;
use tokio_util::io::ReaderStream;

const BUFFER: usize = 64 << 10;

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

impl<W: Wire + 'static> Dock<W> {
    pub(super) async fn upload(
        &self,
        id: i64,
        op: Option<Extension<Operator>>,
        headers: HeaderMap,
        want: Bytes,
    ) -> Result<Response, Fault> {
        let id = admit(self, id, actor(op)?, Reach::See).await?;
        let want = thaw(&headers, want)?;
        let store = self.store.clone();
        let (source, sink) = duplex(BUFFER);
        let task = tokio::spawn(async move {
            let repo = tokio::task::spawn_blocking(move || store.repository(id))
                .await
                .map_err(|error| Error::Io(format!("repository worker failed: {error}")))??;
            repo.upload(want, sink).await
        });
        let flow = Flow {
            source: ReaderStream::new(source),
            task: Some(task),
        };
        Ok(sent(REPLY, Body::from_stream(flow)))
    }
}

struct Flow {
    source: ReaderStream<DuplexStream>,
    task: Option<JoinHandle<Result<(), Error>>>,
}

impl Stream for Flow {
    type Item = Result<Bytes, io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Some(chunk) = std::task::ready!(Pin::new(&mut self.source).poll_next(cx)) {
            return Poll::Ready(Some(chunk));
        }
        let Some(task) = self.task.as_mut() else {
            return Poll::Ready(None);
        };
        let result = std::task::ready!(Pin::new(task).poll(cx));
        self.task = None;
        Poll::Ready(match result {
            Ok(Ok(())) => None,
            Ok(Err(error)) => Some(Err(io::Error::other(error))),
            Err(error) => Some(Err(io::Error::other(error))),
        })
    }
}

impl Drop for Flow {
    fn drop(&mut self) {
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}
