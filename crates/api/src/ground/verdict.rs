use super::{Dock, Fault, Reach, actor, admit, bad, deny};
use axum::extract::{Path, Query, State};
use axum::{Extension, Json};
use codehull_repo::Object;
use keel::{Operator, Wire};
use serde::Deserialize;
use serde_json::{Value, json};

const PENDING: &str = "pending";
const SUCCESS: &str = "success";
const FAILURE: &str = "failure";

#[derive(Deserialize)]
pub(super) struct Cast {
    commit: String,
    context: String,
    state: String,
    note: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct At {
    commit: String,
}

struct Ruling {
    key: i64,
    context: String,
    state: String,
    note: String,
}

struct Court<'a, W: Wire> {
    dock: &'a Dock<W>,
    who: i64,
    id: u64,
}

pub(super) async fn cast<W: Wire + 'static>(
    State(dock): State<Dock<W>>,
    Path(id): Path<i64>,
    op: Option<Extension<Operator>>,
    Json(body): Json<Cast>,
) -> Result<Json<Value>, Fault> {
    let who = actor(op)?;
    let id = admit(&dock, id, who, Reach::Edit).await?;
    let court = Court {
        dock: &dock,
        who,
        id,
    };
    let commit = Object::parse(&body.commit)
        .map_err(|_| bad("commit must be one full hexadecimal Git ID"))?;
    let state = state(&body.state)?;
    let context = plain(&body.context)?;
    let note = body.note.unwrap_or_default();
    if note.contains(['"', '\\']) {
        return Err(bad("note carries a refused character"));
    }
    court.settle(&commit, &context, (state, &note)).await?;
    Ok(Json(
        json!({ "commit": commit.hex(), "context": context, "state": state }),
    ))
}

pub(super) async fn read<W: Wire + 'static>(
    State(dock): State<Dock<W>>,
    Path(id): Path<i64>,
    op: Option<Extension<Operator>>,
    Query(query): Query<At>,
) -> Result<Json<Value>, Fault> {
    let who = actor(op)?;
    let id = admit(&dock, id, who, Reach::See).await?;
    let commit = Object::parse(&query.commit)
        .map_err(|_| bad("commit must be one full hexadecimal Git ID"))?;
    let court = Court {
        dock: &dock,
        who,
        id,
    };
    let held = court.at(&commit).await?;
    Ok(Json(json!({
        "commit": commit.hex(),
        "state": rollup(&held),
        "verdicts": held.iter().map(entry).collect::<Vec<Value>>(),
    })))
}

impl<W: Wire + 'static> Court<'_, W> {
    async fn settle(
        &self,
        commit: &Object,
        context: &str,
        held: (&'static str, &str),
    ) -> Result<(), Fault> {
        let seen = self
            .at(commit)
            .await?
            .into_iter()
            .find(|ruling| ruling.context == context);
        let id = self.id.to_string();
        let hex = commit.hex().to_string();
        let context = context.to_owned();
        let (state, note) = (held.0.to_owned(), held.1.to_owned());
        self.dock
            .core
            .of(self.who)
            .batch(async |tx| {
                if let Some(Ruling { key, .. }) = seen {
                    tx.end("Verdict", key).await?;
                }
                tx.put(
                    "Verdict",
                    &[
                        ("commit", &hex),
                        ("context", &context),
                        ("state", &state),
                        ("note", &note),
                        ("repo", &id),
                    ],
                )
                .await
            })
            .await
            .map(drop)
            .map_err(|_| deny())
    }

    async fn at(&self, commit: &Object) -> Result<Vec<Ruling>, Fault> {
        let (id, hex) = (self.id, commit.hex());
        let pack = self
            .dock
            .core
            .of(self.who)
            .query(&format!(
                r#"from Verdict where repo = "{id}" and commit = "{hex}""#
            ))
            .await
            .map_err(|_| deny())?;
        Ok(pack
            .rows()
            .iter()
            .filter_map(|row| {
                Some(Ruling {
                    key: row.key(),
                    context: row.text("context")?.to_owned(),
                    state: row.text("state")?.to_owned(),
                    note: row.text("note")?.to_owned(),
                })
            })
            .collect())
    }
}

fn entry(held: &Ruling) -> Value {
    json!({ "context": held.context, "state": held.state, "note": held.note })
}

fn rollup(held: &[Ruling]) -> Value {
    if held.is_empty() {
        return Value::Null;
    }
    if held.iter().any(|seen| seen.state == FAILURE) {
        return json!(FAILURE);
    }
    if held.iter().any(|seen| seen.state == PENDING) {
        return json!(PENDING);
    }
    json!(SUCCESS)
}

fn state(held: &str) -> Result<&'static str, Fault> {
    match held {
        PENDING => Ok(PENDING),
        SUCCESS => Ok(SUCCESS),
        FAILURE => Ok(FAILURE),
        _ => Err(bad("state must be pending, success or failure")),
    }
}

fn plain(held: &str) -> Result<String, Fault> {
    if held.is_empty() || held.contains(['"', '\\']) {
        return Err(bad("context is empty or carries a refused character"));
    }
    Ok(held.to_owned())
}
