use super::{Dock, Fault, actor, admit, bad, deny, fault, work};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::{Extension, Json};
use codehull_repo::{Object, admitted};
use keel::{Operator, Wire};
use serde::Deserialize;
use serde_json::{Value, json};

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

pub(super) enum Held {
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
            point.cast(&name, &after).await?;
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
            let held = repo.references()?;
            let gone: Vec<String> = held
                .iter()
                .filter(|(name, _)| !wanted.iter().any(|(seen, _)| seen == name))
                .map(|(name, _)| name.clone())
                .collect();
            let moves: Vec<(String, Object)> = wanted
                .into_iter()
                .filter(|(name, object)| {
                    !held
                        .iter()
                        .any(|(seen, held)| seen == name && held == object)
                })
                .collect();
            repo.point(&moves, &gone)
        })
        .await
    }

    pub(super) async fn settle(
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

    pub(super) async fn welds(&self) -> Result<Vec<String>, Fault> {
        let id = self.id;
        let pack = self
            .dock
            .core
            .of(self.who)
            .query(&format!(r#"from Weld where repo = "{id}""#))
            .await
            .map_err(|_| deny())?;
        Ok(pack
            .rows()
            .iter()
            .filter_map(|row| row.text("mode").map(str::to_owned))
            .collect())
    }

    pub(super) async fn listing(&self) -> Result<Vec<(String, String)>, Fault> {
        let mut held = Vec::new();
        for row in self.rows(None).await? {
            let (Some(name), Some(object)) = (row.text("name"), row.text("object")) else {
                continue;
            };
            held.push((name.to_string(), object.to_string()));
        }
        Ok(held)
    }

    pub(super) async fn shed(&self, key: i64) -> Result<(), Fault> {
        self.dock
            .core
            .of(self.who)
            .batch(async |tx| tx.end("Ref", key).await)
            .await
            .map(|_| ())
            .map_err(|_| clash())
    }

    pub(super) async fn cast(&self, name: &str, object: &Object) -> Result<(), Fault> {
        let store = self.dock.store.clone();
        let id = self.id;
        let held = name.to_string();
        let seen = object.clone();
        work(move || store.repository(id)?.project(&held, &seen)).await
    }

    pub(super) async fn write(
        &self,
        moves: Vec<(String, Object)>,
        gone: Vec<String>,
    ) -> Result<(), Fault> {
        let store = self.dock.store.clone();
        let id = self.id;
        work(move || store.repository(id)?.point(&moves, &gone)).await
    }

    pub(super) async fn seats(&self) -> Result<Vec<(i64, String, String)>, Fault> {
        let mut held = Vec::new();
        for row in self.rows(None).await? {
            let (Some(name), Some(object)) = (row.text("name"), row.text("object")) else {
                continue;
            };
            held.push((row.key(), name.to_string(), object.to_string()));
        }
        Ok(held)
    }

    pub(super) async fn seen(&self, name: &str) -> Result<Option<(i64, String)>, Fault> {
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

pub(super) fn sane(name: &str) -> Result<String, Fault> {
    if !admitted(name) {
        return Err(bad(
            "reference must be below refs and outside the reserved namespaces",
        ));
    }
    if name.contains(['"', '\\', '*', ' ']) {
        return Err(bad("reference name carries a refused character"));
    }
    Ok(name.to_string())
}

pub(super) fn clash() -> Fault {
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
