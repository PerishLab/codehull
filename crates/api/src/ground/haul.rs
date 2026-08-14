use super::{Dock, Fault, actor, admit, bad, work};
use axum::Extension;
use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderValue, header};
use axum::response::{IntoResponse, Response};
use keel::{Operator, Wire};
use serde::Deserialize;

const OFFER: &str = "application/x-git-upload-pack-advertisement";
const REPLY: &str = "application/x-git-upload-pack-result";
const UPLOAD: &str = "git-upload-pack";

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
    if query.service != UPLOAD {
        return Err(bad(format!("unsupported service {}", query.service)));
    }
    let id = admit(&dock, id, actor(op)?).await?;
    let store = dock.store.clone();
    let held = work(move || store.repository(id)?.advertise()).await?;
    let mut body = banner(UPLOAD);
    body.extend_from_slice(&held);
    Ok(sent(OFFER, body))
}

pub(super) async fn upload<W: Wire + 'static>(
    State(dock): State<Dock<W>>,
    Path(id): Path<i64>,
    op: Option<Extension<Operator>>,
    want: Bytes,
) -> Result<Response, Fault> {
    let id = admit(&dock, id, actor(op)?).await?;
    let store = dock.store.clone();
    let held = work(move || store.repository(id)?.upload(&want)).await?;
    Ok(sent(REPLY, held))
}

fn banner(service: &str) -> Vec<u8> {
    let line = format!("# service={service}\n");
    let size = line.len() + 4;
    format!("{size:04x}{line}0000").into_bytes()
}

fn sent(kind: &'static str, body: Vec<u8>) -> Response {
    (
        [
            (header::CONTENT_TYPE, HeaderValue::from_static(kind)),
            (header::CACHE_CONTROL, HeaderValue::from_static("no-cache")),
        ],
        body,
    )
        .into_response()
}
