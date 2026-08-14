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
    expect: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct Offer {
    mode: String,
    note: Option<String>,
    expect: Option<String>,
}

struct Weld<'a, W: Wire> {
    dock: &'a Dock<W>,
    who: i64,
}

pub(super) async fn weld<W: Wire + 'static>(
    State(dock): State<Dock<W>>,
    Path(id): Path<i64>,
    op: Option<Extension<Operator>>,
    Json(body): Json<Ask>,
) -> Result<Json<Value>, Fault> {
    let who = actor(op)?;
    let id = admit(&dock, id, who).await?;
    let held = Weld { dock: &dock, who };
    Ok(Json(held.fuse(id, body).await?))
}

pub(super) async fn pull<W: Wire + 'static>(
    State(dock): State<Dock<W>>,
    Path(id): Path<i64>,
    op: Option<Extension<Operator>>,
    Json(body): Json<Offer>,
) -> Result<Json<Value>, Fault> {
    let who = actor(op)?;
    let held = Weld { dock: &dock, who };
    let (repo, ask) = held.proposal(id, body).await?;
    let seat = admit(&dock, repo, who).await?;
    let mut done = held.fuse(seat, ask).await?;
    let hex = done
        .get("object")
        .and_then(Value::as_str)
        .ok_or_else(deny)?
        .to_string();
    held.stamp(id, &hex).await?;
    done["pull"] = json!(id);
    Ok(Json(done))
}

impl<W: Wire + 'static> Weld<'_, W> {
    async fn fuse(&self, id: u64, ask: Ask) -> Result<Value, Fault> {
        let point = Point {
            dock: self.dock,
            who: self.who,
            id,
        };
        let mode = mode(&ask.mode)?;
        if !point.welds().await?.iter().any(|held| held == mode) {
            return Err(bad(format!("{mode} is not a permitted merge here")));
        }
        let base = sane(&ask.base)?;
        let head = sane(&ask.head)?;
        let (key, held) = point
            .seen(&base)
            .await?
            .ok_or_else(|| bad("base is absent"))?;
        let (_, tip) = point
            .seen(&head)
            .await?
            .ok_or_else(|| bad("head is absent"))?;
        if ask.expect.is_some_and(|want| want != tip) {
            return Err(clash());
        }
        let old = Object::parse(&held).map_err(|_| deny())?;
        let new = Object::parse(&tip).map_err(|_| deny())?;
        let note = ask
            .note
            .unwrap_or_else(|| format!("Merge {head} into {base}"));
        let plan = Plan {
            mode,
            old: old.clone(),
            new,
            note,
        };
        let after = self.shape(id, plan).await?;
        match point
            .settle(&base, Some((key, held)), Some(old), &after)
            .await?
        {
            Held::Stale => Err(clash()),
            Held::Moved => {
                point.cast(&base, &after).await?;
                Ok(json!({ "name": base, "object": after.hex(), "mode": mode }))
            }
        }
    }

    async fn proposal(&self, id: i64, offer: Offer) -> Result<(i64, Ask), Fault> {
        let pack = self
            .dock
            .core
            .of(self.who)
            .query(&format!(r#"from Pull where id = "{id}""#))
            .await
            .map_err(|_| deny())?;
        let row = pack.rows().first().ok_or_else(deny)?;
        let (Some(base), Some(head)) = (row.text("base"), row.text("head")) else {
            return Err(deny());
        };
        if row.int("source_id").or_else(|| row.int("source")).is_some() {
            return Err(bad("a proposal from another repository is not merged here"));
        }
        let issue = row
            .int("issue_id")
            .or_else(|| row.int("issue"))
            .ok_or_else(deny)?;
        let ask = Ask {
            base: base.to_string(),
            head: head.to_string(),
            mode: offer.mode,
            note: offer.note,
            expect: offer.expect,
        };
        Ok((self.seat(issue).await?, ask))
    }

    async fn seat(&self, issue: i64) -> Result<i64, Fault> {
        let pack = self
            .dock
            .core
            .of(self.who)
            .query(&format!(r#"from Issue where id = "{issue}""#))
            .await
            .map_err(|_| deny())?;
        let row = pack.rows().first().ok_or_else(deny)?;
        row.int("repo_id")
            .or_else(|| row.int("repo"))
            .ok_or_else(deny)
    }

    async fn stamp(&self, id: i64, hex: &str) -> Result<(), Fault> {
        self.dock
            .core
            .of(self.who)
            .set("Pull", id, &[("weld", hex)])
            .await
            .map_err(|_| deny())
    }

    async fn shape(&self, id: u64, plan: Plan) -> Result<Object, Fault> {
        let store = self.dock.store.clone();
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
}

struct Plan {
    mode: &'static str,
    old: Object,
    new: Object,
    note: String,
}

fn mode(held: &str) -> Result<&'static str, Fault> {
    match held {
        FORWARD => Ok(FORWARD),
        JOIN => Ok(JOIN),
        SQUASH => Ok(SQUASH),
        _ => Err(bad("merge mode must be forward, join or squash")),
    }
}
