use super::{Dock, Fault, actor, admit, bad, deny, fault, work};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::{Extension, Json};
use codehull_repo::Object;
use keel::{Operator, Wire};
use serde::Deserialize;
use serde_json::{Value, json};

const HEADS: &str = "refs/heads/";

#[derive(Deserialize)]
pub(super) struct Move {
    name: String,
    before: Option<String>,
    after: String,
}

#[derive(Deserialize)]
pub(super) struct Name {
    name: String,
}

enum Held {
    Moved,
    Stale,
}

pub(super) async fn read<W: Wire + 'static>(
    State(dock): State<Dock<W>>,
    Path(id): Path<i64>,
    op: Option<Extension<Operator>>,
    Query(query): Query<Name>,
) -> Result<Json<Value>, Fault> {
    let who = actor(op)?;
    let id = admit(&dock, id, who).await?;
    let name = sane(&query.name)?;
    let held = Point {
        dock: &dock,
        who,
        id,
    }
    .seen(&name)
    .await?;
    Ok(Json(json!({
        "name": name,
        "object": held.as_ref().map(|(_, object)| object.clone()),
    })))
}

pub(super) async fn advance<W: Wire + 'static>(
    State(dock): State<Dock<W>>,
    Path(id): Path<i64>,
    op: Option<Extension<Operator>>,
    Json(body): Json<Move>,
) -> Result<Json<Value>, Fault> {
    let who = actor(op)?;
    let id = admit(&dock, id, who).await?;
    let name = sane(&body.name)?;
    let after = Object::parse(&body.after).map_err(fault)?;
    let before = body
        .before
        .as_deref()
        .map(Object::parse)
        .transpose()
        .map_err(fault)?;
    let store = dock.store.clone();
    let held = after.clone();
    work(move || store.repository(id)?.holds(&held)).await?;
    let point = Point {
        dock: &dock,
        who,
        id,
    };
    let seat = point.seen(&name).await?;
    match point.settle(&name, seat, before, &after).await? {
        Held::Stale => Err(clash()),
        Held::Moved => {
            let store = dock.store.clone();
            let held = name.clone();
            let object = after.clone();
            work(move || store.repository(id)?.project(&held, &object)).await?;
            Ok(Json(json!({ "name": name, "object": after.hex() })))
        }
    }
}

pub(super) async fn drop<W: Wire + 'static>(
    State(dock): State<Dock<W>>,
    Path(id): Path<i64>,
    op: Option<Extension<Operator>>,
    Query(query): Query<Name>,
) -> Result<StatusCode, Fault> {
    let who = actor(op)?;
    let id = admit(&dock, id, who).await?;
    let name = sane(&query.name)?;
    let Some((key, _)) = Point {
        dock: &dock,
        who,
        id,
    }
    .seen(&name)
    .await?
    else {
        return Err(gone());
    };
    dock.core
        .of(who)
        .batch(async |tx| tx.end("Ref", key).await)
        .await
        .map_err(|_| clash())?;
    let store = dock.store.clone();
    let held = name.clone();
    work(move || store.repository(id)?.retire(&held)).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) struct Point<'a, W: Wire> {
    pub(super) dock: &'a Dock<W>,
    pub(super) who: i64,
    pub(super) id: u64,
}

impl<W: Wire + 'static> Point<'_, W> {
    pub(super) async fn align(&self) -> Result<(), Fault> {
        let mut wanted = Vec::new();
        for row in self.rows(None).await? {
            let (Some(name), Some(hex)) = (row.text("name"), row.text("object")) else {
                continue;
            };
            wanted.push((name.to_string(), Object::parse(hex).map_err(fault)?));
        }
        let store = self.dock.store.clone();
        let id = self.id;
        work(move || {
            let repo = store.repository(id)?;
            for (name, _) in repo.references()? {
                if !wanted.iter().any(|(held, _)| *held == name) {
                    repo.retire(&name)?;
                }
            }
            for (name, object) in &wanted {
                repo.project(name, object)?;
            }
            Ok(())
        })
        .await
    }

    async fn settle(
        &self,
        name: &str,
        seat: Option<(i64, String)>,
        before: Option<Object>,
        after: &Object,
    ) -> Result<Held, Fault> {
        match (&seat, &before) {
            (None, Some(_)) | (Some(_), None) => return Ok(Held::Stale),
            (Some((_, held)), Some(want)) if held != want.hex() => return Ok(Held::Stale),
            _ => (),
        }
        let key = seat.map(|(key, _)| key);
        let hex = after.hex().to_string();
        let name = name.to_string();
        let id = self.id.to_string();
        self.dock
            .core
            .of(self.who)
            .batch(async |tx| {
                if let Some(key) = key {
                    tx.end("Ref", key).await?;
                }
                tx.put("Ref", &[("name", &name), ("object", &hex), ("repo", &id)])
                    .await
            })
            .await
            .map(|_| Held::Moved)
            .map_err(|_| clash())
    }

    async fn seen(&self, name: &str) -> Result<Option<(i64, String)>, Fault> {
        let pack = self.rows(Some(name)).await?;
        let Some(row) = pack.first() else {
            return Ok(None);
        };
        let object = row.text("object").ok_or_else(deny)?.to_string();
        Ok(Some((row.key(), object)))
    }

    async fn rows(&self, name: Option<&str>) -> Result<Vec<keel::Row>, Fault> {
        let filter = match name {
            Some(name) => format!(r#" and name = "{name}""#),
            None => String::new(),
        };
        let id = self.id;
        let pack = self
            .dock
            .core
            .of(self.who)
            .query(&format!(r#"from Ref where repo = "{id}"{filter}"#))
            .await
            .map_err(|_| deny())?;
        Ok(pack.rows().to_vec())
    }
}

fn sane(name: &str) -> Result<String, Fault> {
    if !name.starts_with(HEADS) {
        return Err(bad("reference must be below refs/heads"));
    }
    if name.contains(['"', '\\', '*', ' ']) {
        return Err(bad("reference name carries a refused character"));
    }
    Ok(name.to_string())
}

fn clash() -> Fault {
    (
        StatusCode::CONFLICT,
        Json(json!({ "error": "reference moved under the expectation" })),
    )
}

fn gone() -> Fault {
    (
        StatusCode::NOT_FOUND,
        Json(json!({ "error": "reference is absent" })),
    )
}
