use super::point::{Held, Point, clash, sane};
use super::{Dock, Fault, actor, admit, bad, deny, work};
use axum::extract::{Path, State};
use axum::{Extension, Json};
use codehull_repo::Object;
use keel::{Operator, Wire};
use serde::Deserialize;
use serde_json::{Value, json};

const FORWARD: &str = "forward";
const JOIN: &str = "join";
const SQUASH: &str = "squash";

#[derive(Deserialize)]
pub(super) struct Ask {
    base: String,
    head: String,
    mode: String,
    note: Option<String>,
}

pub(super) async fn weld<W: Wire + 'static>(
    State(dock): State<Dock<W>>,
    Path(id): Path<i64>,
    op: Option<Extension<Operator>>,
    Json(body): Json<Ask>,
) -> Result<Json<Value>, Fault> {
    let who = actor(op)?;
    let id = admit(&dock, id, who).await?;
    let point = Point {
        dock: &dock,
        who,
        id,
    };
    let mode = mode(&body.mode)?;
    if !allowed(&point, mode).await? {
        return Err(bad(format!("{mode} is not a permitted merge here")));
    }
    let base = sane(&body.base)?;
    let head = sane(&body.head)?;
    let (key, held) = point
        .seen(&base)
        .await?
        .ok_or_else(|| bad("base is absent"))?;
    let (_, tip) = point
        .seen(&head)
        .await?
        .ok_or_else(|| bad("head is absent"))?;
    let old = Object::parse(&held).map_err(|_| deny())?;
    let new = Object::parse(&tip).map_err(|_| deny())?;
    let note = body
        .note
        .unwrap_or_else(|| format!("Merge {head} into {base}"));
    let plan = Plan {
        mode,
        old: old.clone(),
        new,
        note,
    };
    let after = shape(&dock, id, plan).await?;
    match point
        .settle(&base, Some((key, held)), Some(old), &after)
        .await?
    {
        Held::Stale => Err(clash()),
        Held::Moved => {
            point.cast(&base, &after).await?;
            Ok(Json(
                json!({ "name": base, "object": after.hex(), "mode": mode }),
            ))
        }
    }
}

struct Plan {
    mode: &'static str,
    old: Object,
    new: Object,
    note: String,
}

async fn shape<W: Wire + 'static>(dock: &Dock<W>, id: u64, plan: Plan) -> Result<Object, Fault> {
    let store = dock.store.clone();
    work(move || {
        let repo = store.repository(id)?;
        if plan.mode == FORWARD {
            return match repo.ancestor(&plan.old, &plan.new)? {
                true => Ok(plan.new),
                false => Err(codehull_repo::Error::Conflict(
                    "base is not an ancestor of head".into(),
                )),
            };
        }
        let tree = repo.weld(&plan.old, &plan.new)?;
        let parents: Vec<&Object> = match plan.mode {
            SQUASH => vec![&plan.old],
            _ => vec![&plan.old, &plan.new],
        };
        repo.commit(&tree, &parents, &plan.note)
    })
    .await
}

fn mode(held: &str) -> Result<&'static str, Fault> {
    match held {
        FORWARD => Ok(FORWARD),
        JOIN => Ok(JOIN),
        SQUASH => Ok(SQUASH),
        _ => Err(bad("merge mode must be forward, join or squash")),
    }
}

async fn allowed<W: Wire + 'static>(point: &Point<'_, W>, mode: &str) -> Result<bool, Fault> {
    Ok(point.welds().await?.iter().any(|held| held == mode))
}
