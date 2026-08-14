use super::line::{self, Order};
use super::point::{Held, Point, sane};
use super::{Dock, Fault, actor, admit, bad, work};
use axum::Extension;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderValue, header};
use axum::response::{IntoResponse, Response};
use codehull_repo::{Object, Pen};
use keel::{Operator, Wire};

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
    let pen = match pack.is_empty() {
        true => None,
        false => Some(stow(&dock, id, pack).await?),
    };
    let held = Take { point: &point, pen };
    Ok(sent(REPLY, held.settle(&orders).await))
}

impl<W: Wire + 'static> Take<'_, W> {
    pub(super) async fn settle(mut self, orders: &[Order]) -> Vec<u8> {
        let mut plans = Vec::new();
        for order in orders {
            plans.push(self.weigh(order).await);
        }
        let wanted = plans.iter().any(admits);
        let held = match self.pen.take() {
            Some(pen) => kept(pen, wanted).await,
            None => true,
        };
        let mut report = line::pkt("unpack ok\n");
        for (order, plan) in orders.iter().zip(plans) {
            report.extend_from_slice(&line::pkt(&match self.run(plan, held).await {
                Ok(()) => format!("ok {}\n", order.name),
                Err(note) => format!("ng {} {note}\n", order.name),
            }));
        }
        report.extend_from_slice(&line::flush());
        report
    }

    async fn weigh(&self, order: &Order) -> Result<Plan, String> {
        let name =
            sane(&order.name).map_err(|_| "reference is reserved or outside refs".to_owned())?;
        let old = Object::parse(&order.old).map_err(|_| "old object is malformed".to_owned())?;
        let new = Object::parse(&order.new).map_err(|_| "new object is malformed".to_owned())?;
        let seat = self
            .point
            .seen(&name)
            .await
            .map_err(|_| "reference is unreadable".to_owned())?;
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
        self.stored(&new)
            .await
            .map_err(|_| "objects are absent".to_owned())?;
        Ok(Plan {
            name,
            seat,
            before: (!old.absent()).then_some(old),
            after: Some(new),
        })
    }

    async fn stored(&self, object: &Object) -> Result<(), Fault> {
        let store = self.point.dock.store.clone();
        let id = self.point.id;
        let held = object.clone();
        let seat = self.pen.clone();
        work(move || {
            let repo = store.repository(id)?;
            match &seat {
                Some(pen) => repo.sees(pen, &held),
                None => repo.holds(&held),
            }
        })
        .await
    }

    async fn run(&self, plan: Result<Plan, String>, held: bool) -> Result<(), String> {
        let plan = plan?;
        if plan.after.is_some() && !held {
            return Err("objects were not admitted".to_owned());
        }
        self.apply(plan).await
    }

    async fn apply(&self, plan: Plan) -> Result<(), String> {
        let Some(after) = plan.after else {
            let Some((key, _)) = plan.seat else {
                return Err("reference is already absent".to_owned());
            };
            return self
                .point
                .strip(key, &plan.name)
                .await
                .map_err(|_| "retirement refused".to_owned());
        };
        match self
            .point
            .settle(&plan.name, plan.seat, plan.before, &after)
            .await
        {
            Err(_) => Err("reference refused the move".to_owned()),
            Ok(Held::Stale) => Err("reference moved under the expectation".to_owned()),
            Ok(Held::Moved) => self
                .point
                .cast(&plan.name, &after)
                .await
                .map_err(|_| "projection refused".to_owned()),
        }
    }
}

async fn stow<W: Wire + 'static>(dock: &Dock<W>, id: u64, pack: &[u8]) -> Result<Pen, Fault> {
    let store = dock.store.clone();
    let pen = work(move || store.repository(id)?.pen()).await?;
    let store = dock.store.clone();
    let held = pack.to_vec();
    let seat = pen.clone();
    match work(move || store.repository(id)?.index(&seat, &held)).await {
        Ok(()) => Ok(pen),
        Err(fault) => {
            let _ = work(move || pen.wipe()).await;
            Err(fault)
        }
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
