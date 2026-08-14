use super::line::{self, Order};
use super::point::{Held, Point, sane};
use super::{Dock, Fault, actor, admit, bad, work};
use axum::Extension;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderValue, header};
use axum::response::{IntoResponse, Response};
use codehull_repo::Object;
use keel::{Operator, Wire};

pub(super) const RECEIVE: &str = "git-receive-pack";
pub(super) const CAPS: &str = "report-status delete-refs";
const REPLY: &str = "application/x-git-receive-pack-result";

pub(super) async fn take<W: Wire + 'static>(
    State(dock): State<Dock<W>>,
    Path(id): Path<i64>,
    op: Option<Extension<Operator>>,
    body: Bytes,
) -> Result<Response, Fault> {
    let who = actor(op)?;
    let id = admit(&dock, id, who).await?;
    let point = Point {
        dock: &dock,
        who,
        id,
    };
    point.align().await?;
    let (orders, pack) = line::orders(&body).map_err(bad)?;
    if !pack.is_empty() {
        let store = dock.store.clone();
        let held = pack.to_vec();
        work(move || store.repository(id)?.index(&held)).await?;
    }
    let mut report = line::pkt("unpack ok\n");
    for order in &orders {
        let note = apply(&point, order).await;
        report.extend_from_slice(&line::pkt(&match note {
            Ok(()) => format!("ok {}\n", order.name),
            Err(note) => format!("ng {} {note}\n", order.name),
        }));
    }
    report.extend_from_slice(&line::flush());
    Ok(sent(REPLY, report))
}

async fn apply<W: Wire + 'static>(point: &Point<'_, W>, order: &Order) -> Result<(), String> {
    let name = sane(&order.name).map_err(|_| "reference is outside refs/heads".to_owned())?;
    let old = Object::parse(&order.old).map_err(|_| "old object is malformed".to_owned())?;
    let new = Object::parse(&order.new).map_err(|_| "new object is malformed".to_owned())?;
    let seat = point
        .seen(&name)
        .await
        .map_err(|_| "reference is unreadable".to_owned())?;
    if new.absent() {
        let Some((key, _)) = seat else {
            return Err("reference is already absent".to_owned());
        };
        return point
            .strip(key, &name)
            .await
            .map_err(|_| "retirement refused".to_owned());
    }
    let before = (!old.absent()).then_some(old);
    match point.settle(&name, seat, before, &new).await {
        Err(_) => Err("reference refused the move".to_owned()),
        Ok(Held::Stale) => Err("reference moved under the expectation".to_owned()),
        Ok(Held::Moved) => point
            .cast(&name, &new)
            .await
            .map_err(|_| "projection refused".to_owned()),
    }
}

pub(super) fn sent(kind: &'static str, body: Vec<u8>) -> Response {
    (
        [
            (header::CONTENT_TYPE, HeaderValue::from_static(kind)),
            (header::CACHE_CONTROL, HeaderValue::from_static("no-cache")),
        ],
        body,
    )
        .into_response()
}
