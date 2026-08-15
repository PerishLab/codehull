use super::line;
use super::point::Point;
use super::take::{CAPS, RECEIVE, sent};
use super::{Dock, Fault, actor, admit, bad, thaw, work};
use axum::Extension;
use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::response::Response;
use keel::{Operator, Wire};
use serde::Deserialize;

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
    let id = admit(&dock, id, who).await?;
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
    let id = admit(&dock, id, actor(op)?).await?;
    let want = thaw(&headers, want)?;
    let store = dock.store.clone();
    let held = work(move || store.repository(id)?.upload(&want)).await?;
    Ok(sent(REPLY, held))
}
