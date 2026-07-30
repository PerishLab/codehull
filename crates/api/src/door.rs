use axum::extract::{Path as Route, State};
use axum::http::StatusCode;
use axum::{Extension, Json};
use keel::Ends;
use keel::{Core, Operator, Wire};
use serde_json::{Map, Value, json};
use std::sync::Arc;

pub(crate) async fn found<W: Wire + 'static>(
    State(core): State<Arc<Core<W>>>,
    op: Option<Extension<Operator>>,
    Json(body): Json<Map<String, Value>>,
) -> Result<(StatusCode, Json<Value>), StatusCode> {
    let Some(Extension(Operator(actor))) = op else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let name = body
        .get("login")
        .and_then(Value::as_str)
        .ok_or(StatusCode::BAD_REQUEST)?
        .to_string();
    let face = core.of(actor);
    let org = face
        .batch(async |tx| {
            let org = tx
                .put(
                    "Actor",
                    &[("login", &name), ("kind", "org"), ("barred", "false")],
                )
                .await?;
            let team = tx
                .put(
                    "Team",
                    &[
                        ("name", "owners"),
                        ("mode", "admin"),
                        ("org", &org.to_string()),
                    ],
                )
                .await?;
            tx.tie(
                "Team",
                "members",
                Ends {
                    left: team,
                    right: actor,
                },
                &[],
            )
            .await?;
            tx.put(
                "@grant",
                &[
                    ("who", &format!("team {team}")),
                    ("verb", "*"),
                    ("unit", "Actor"),
                    ("scope", &format!("row {org}")),
                ],
            )
            .await?;
            Ok(org)
        })
        .await
        .map_err(|_| StatusCode::FORBIDDEN)?;
    Ok((StatusCode::CREATED, Json(json!({ "id": org }))))
}

pub(crate) async fn kids<W: Wire>(
    face: &keel::Face<'_, W>,
    id: i64,
    unit: &str,
) -> Result<Vec<i64>, keel::adapt::Error> {
    let pack = face
        .query(&format!(r#"from {unit} where repo = "{id}""#))
        .await?;
    Ok(pack.rows().iter().map(keel::Row::key).collect())
}

pub(crate) async fn close<W: Wire + 'static>(
    State(core): State<Arc<Core<W>>>,
    Route(id): Route<i64>,
    op: Option<Extension<Operator>>,
) -> Result<StatusCode, StatusCode> {
    let who = op
        .map(|Extension(Operator(id))| id)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let face = core.of(who);
    let issues = kids(&face, id, "Issue")
        .await
        .map_err(|_| StatusCode::FORBIDDEN)?;
    let labels = kids(&face, id, "repo:label")
        .await
        .map_err(|_| StatusCode::FORBIDDEN)?;
    let milestones = kids(&face, id, "Milestone")
        .await
        .map_err(|_| StatusCode::FORBIDDEN)?;
    face.batch(async |tx| {
        for key in &issues {
            tx.end("Issue", *key).await?;
        }
        for key in &labels {
            tx.end("repo:label", *key).await?;
        }
        for key in &milestones {
            tx.end("Milestone", *key).await?;
        }
        tx.end("Repo", id).await?;
        Ok(())
    })
    .await
    .map_err(|_| StatusCode::CONFLICT)?;
    Ok(StatusCode::NO_CONTENT)
}
