use super::line::{self, Order};
use super::point::{Held, Point, sane};
use super::{CEILING, Dock, Fault, GZIP, Reach, actor, admit, bad, work};
use axum::Extension;
use axum::body::{Body, Bytes};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, header};
use axum::response::{IntoResponse, Response};
use codehull_repo::{Object, Pen, Store};
use flate2::read::GzDecoder;
use http_body_util::BodyExt;
use keel::{Operator, Wire};
use std::io::{Cursor, Read};
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, channel};

const FLOW: usize = 8;

pub(super) const RECEIVE: &str = "git-receive-pack";
pub(super) const CAPS: &str = "report-status delete-refs";
const REPLY: &str = "application/x-git-receive-pack-result";

struct Plan {
    name: String,
    seat: Option<(i64, String)>,
    before: Option<Object>,
    after: Option<Object>,
}

pub(super) struct Take<'a, W: Wire> {
    pub(super) point: &'a Point<'a, W>,
    pub(super) pen: Option<Pen>,
}

pub(super) async fn take<W: Wire + 'static>(
    State(dock): State<Dock<W>>,
    Path(id): Path<i64>,
    op: Option<Extension<Operator>>,
    headers: HeaderMap,
    body: Body,
) -> Result<Response, Fault> {
    let who = actor(op)?;
    let id = admit(&dock, id, who, Reach::Edit).await?;
    let point = Point {
        dock: &dock,
        who,
        id,
    };
    point.align().await?;
    let (orders, pen) = intake(&dock, id, &headers, body).await?;
    let held = Take { point: &point, pen };
    Ok(sent(REPLY, held.settle(&orders).await))
}

impl<W: Wire + 'static> Take<'_, W> {
    pub(super) async fn settle(mut self, orders: &[Order]) -> Vec<u8> {
        let seats = self.point.seats().await.unwrap_or_default();
        let gone = self.absent(orders).await;
        let mut plans = Vec::new();
        for order in orders {
            plans.push(weigh(order, &seats, &gone));
        }
        let wanted = plans.iter().any(admits);
        let held = match self.pen.take() {
            Some(pen) => kept(pen, wanted).await,
            None => true,
        };
        let mut done = Vec::new();
        let mut moves = Vec::new();
        let mut gone = Vec::new();
        for plan in plans {
            done.push(self.run(plan, held, &mut moves, &mut gone).await);
        }
        let cast = self.point.write(moves, gone).await;
        let mut report = line::pkt("unpack ok\n");
        for (order, seen) in orders.iter().zip(done) {
            let held = match (seen, &cast) {
                (Ok(true), Err(_)) => Err("projection refused".to_owned()),
                (seen, _) => seen.map(|_| ()),
            };
            report.extend_from_slice(&line::pkt(&match held {
                Ok(()) => format!("ok {}\n", order.name),
                Err(note) => format!("ng {} {note}\n", order.name),
            }));
        }
        report.extend_from_slice(&line::flush());
        report
    }

    async fn absent(&self, orders: &[Order]) -> Vec<Object> {
        let wanted: Vec<Object> = orders
            .iter()
            .filter_map(|order| Object::parse(&order.new).ok())
            .filter(|object| !object.absent())
            .collect();
        let store = self.point.dock.store.clone();
        let id = self.point.id;
        let seat = self.pen.clone();
        work(move || store.repository(id)?.absent(seat.as_ref(), &wanted))
            .await
            .unwrap_or_default()
    }

    async fn run(
        &self,
        plan: Result<Plan, String>,
        held: bool,
        moves: &mut Vec<(String, Object)>,
        gone: &mut Vec<String>,
    ) -> Result<bool, String> {
        let plan = plan?;
        if plan.after.is_some() && !held {
            return Err("objects were not admitted".to_owned());
        }
        let Some(after) = plan.after else {
            let Some((key, _)) = plan.seat else {
                return Err("reference is already absent".to_owned());
            };
            self.point
                .shed(key)
                .await
                .map_err(|_| "retirement refused".to_owned())?;
            gone.push(plan.name);
            return Ok(true);
        };
        match self
            .point
            .settle(&plan.name, plan.seat, plan.before, &after)
            .await
        {
            Err(_) => Err("reference refused the move".to_owned()),
            Ok(Held::Stale) => Err("reference moved under the expectation".to_owned()),
            Ok(Held::Moved) => {
                moves.push((plan.name, after));
                Ok(true)
            }
        }
    }
}

fn weigh(order: &Order, seats: &[(i64, String, String)], gone: &[Object]) -> Result<Plan, String> {
    let name = sane(&order.name).map_err(|_| "reference is reserved or outside refs".to_owned())?;
    let old = Object::parse(&order.old).map_err(|_| "old object is malformed".to_owned())?;
    let new = Object::parse(&order.new).map_err(|_| "new object is malformed".to_owned())?;
    let seat = seats
        .iter()
        .find(|(_, held, _)| *held == name)
        .map(|(key, _, object)| (*key, object.clone()));
    if new.absent() {
        if seat.is_none() {
            return Err("reference is already absent".to_owned());
        }
        return Ok(Plan {
            name,
            seat,
            before: None,
            after: None,
        });
    }
    if gone.contains(&new) {
        return Err("objects are absent".to_owned());
    }
    Ok(Plan {
        name,
        seat,
        before: (!old.absent()).then_some(old),
        after: Some(new),
    })
}

struct Bridge {
    rx: Receiver<Bytes>,
    held: Bytes,
}

impl Read for Bridge {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        while self.held.is_empty() {
            match self.rx.blocking_recv() {
                Some(next) => self.held = next,
                None => return Ok(0),
            }
        }
        let size = out.len().min(self.held.len());
        out[..size].copy_from_slice(&self.held[..size]);
        self.held = self.held.slice(size..);
        Ok(size)
    }
}

async fn intake<W: Wire + 'static>(
    dock: &Dock<W>,
    id: u64,
    headers: &HeaderMap,
    body: Body,
) -> Result<(Vec<Order>, Option<Pen>), Fault> {
    let coded = headers
        .get(header::CONTENT_ENCODING)
        .and_then(|held| held.to_str().ok())
        .is_some_and(|held| held.contains(GZIP));
    let (tx, rx) = channel(FLOW);
    let store = dock.store.clone();
    let task = tokio::task::spawn_blocking(move || sift(store, id, coded, rx));
    let mut body = body;
    while let Some(frame) = body.frame().await {
        let frame = frame.map_err(|error| bad(format!("cannot read the request: {error}")))?;
        let Ok(data) = frame.into_data() else {
            continue;
        };
        if tx.send(data).await.is_err() {
            break;
        }
    }
    drop(tx);
    task.await.map_err(|_| bad("the request was dropped"))?
}

fn sift(
    store: Arc<Store>,
    id: u64,
    coded: bool,
    rx: Receiver<Bytes>,
) -> Result<(Vec<Order>, Option<Pen>), Fault> {
    let bridge = Bridge {
        rx,
        held: Bytes::new(),
    };
    let mut src: Box<dyn Read> = match coded {
        true => Box::new(GzDecoder::new(bridge)),
        false => Box::new(bridge),
    };
    let (orders, rest) = heads(&mut src)?;
    let repo = store.repository(id).map_err(super::fault)?;
    let mut src = Cursor::new(rest).chain(src);
    let mut peek = [0u8; 1];
    if src
        .read(&mut peek)
        .map_err(|error| bad(error.to_string()))?
        == 0
    {
        return Ok((orders, None));
    }
    let pen = repo.pen().map_err(super::fault)?;
    let mut src = Cursor::new(peek).chain(src);
    match repo.index(&pen, &mut src) {
        Ok(()) => Ok((orders, Some(pen))),
        Err(error) => {
            let _ = pen.wipe();
            Err(super::fault(error))
        }
    }
}

fn heads(src: &mut dyn Read) -> Result<(Vec<Order>, Vec<u8>), Fault> {
    let mut held = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        if let Some((orders, at)) = line::orders(&held).map_err(bad)? {
            let rest = held.split_off(at);
            return Ok((orders, rest));
        }
        if held.len() as u64 > CEILING {
            return Err(bad("update request has no flush"));
        }
        let size = src
            .read(&mut chunk)
            .map_err(|error| bad(format!("cannot read the request: {error}")))?;
        if size == 0 {
            return Err(bad("update request has no flush"));
        }
        held.extend_from_slice(&chunk[..size]);
    }
}

async fn kept(pen: Pen, wanted: bool) -> bool {
    work(move || match wanted {
        true => pen.keep(),
        false => pen.wipe(),
    })
    .await
    .is_ok()
        && wanted
}

fn admits(plan: &Result<Plan, String>) -> bool {
    plan.as_ref().is_ok_and(|plan| plan.after.is_some())
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
