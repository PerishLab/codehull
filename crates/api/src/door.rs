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
        .get("name")
        .and_then(Value::as_str)
        .ok_or(StatusCode::BAD_REQUEST)?
        .to_string();
    let face = core.of(actor);
    let org = face
        .batch(async |tx| {
            let org = tx
                .put("Actor", &[("iss", crate::rig::ORG), ("sub", &name)])
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

pub(crate) async fn propose<W: Wire + 'static>(
    State(core): State<Arc<Core<W>>>,
    op: Option<Extension<Operator>>,
    Json(body): Json<Map<String, Value>>,
) -> Result<(StatusCode, Json<Value>), StatusCode> {
    let Some(Extension(Operator(actor))) = op else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let issue = body
        .get("issue")
        .and_then(Value::as_i64)
        .ok_or(StatusCode::BAD_REQUEST)?;
    let base = text(&body, "base")?;
    let head = text(&body, "head")?;
    let pack = core
        .of(actor)
        .query(&format!(r#"from Issue where id = "{issue}""#))
        .await
        .map_err(|_| StatusCode::FORBIDDEN)?;
    let row = pack.rows().first().ok_or(StatusCode::NOT_FOUND)?;
    let author = row
        .int("author_id")
        .or_else(|| row.int("author"))
        .ok_or(StatusCode::FORBIDDEN)?;
    if author != actor {
        return Err(StatusCode::FORBIDDEN);
    }
    let repo = row
        .int("repo_id")
        .or_else(|| row.int("repo"))
        .ok_or(StatusCode::FORBIDDEN)?;
    let key = core
        .sudo()
        .put(
            "Pull",
            &[
                ("base", &base),
                ("head", &head),
                ("weld", ""),
                ("repo", &repo.to_string()),
                ("issue", &issue.to_string()),
                ("author", &actor.to_string()),
            ],
        )
        .await
        .map_err(|_| StatusCode::CONFLICT)?;
    Ok((StatusCode::CREATED, Json(json!({ "id": key }))))
}

fn text(body: &Map<String, Value>, name: &str) -> Result<String, StatusCode> {
    body.get(name)
        .and_then(Value::as_str)
        .filter(|held| !held.is_empty())
        .map(str::to_owned)
        .ok_or(StatusCode::BAD_REQUEST)
}

pub(crate) async fn kids<W: Wire>(
    face: &keel::Face<'_, W>,
    unit: &str,
    bond: &str,
    roots: &[i64],
) -> Result<Vec<i64>, keel::adapt::Error> {
    let mut found = Vec::new();
    for root in roots {
        let pack = face
            .query(&format!(r#"from {unit} where {bond} = "{root}""#))
            .await?;
        found.extend(pack.rows().iter().map(keel::Row::key));
    }
    Ok(found)
}

const LEAVES: [&str; 13] = [
    "Verdict",
    "Weld",
    "Ref",
    "Release",
    "Package",
    "Run",
    "Variable",
    "repo:runner",
    "repo:secret",
    "repo:key",
    "repo:label",
    "Mirror",
    "Milestone",
];

pub(crate) async fn close<W: Wire + 'static>(
    State(core): State<Arc<Core<W>>>,
    Route(id): Route<i64>,
    op: Option<Extension<Operator>>,
) -> Result<StatusCode, StatusCode> {
    let who = op
        .map(|Extension(Operator(id))| id)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let face = core.of(who);
    let orders = sweep(&face, id).await.map_err(|_| StatusCode::FORBIDDEN)?;
    face.batch(async |tx| {
        for (unit, key) in &orders {
            tx.end(unit, *key).await?;
        }
        Ok(())
    })
    .await
    .map_err(|_| StatusCode::CONFLICT)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn sweep<W: Wire>(
    face: &keel::Face<'_, W>,
    id: i64,
) -> Result<Vec<(&'static str, i64)>, keel::adapt::Error> {
    let held = [id];
    let issues = kids(face, "Issue", "repo", &held).await?;
    let pulls = kids(face, "Pull", "repo", &held).await?;
    let shields = kids(face, "Shield", "repo", &held).await?;
    let projects = kids(face, "Project", "repo", &held).await?;
    let reviews = kids(face, "Review", "pull", &pulls).await?;
    let mut orders = Vec::new();
    stack(
        &mut orders,
        "Note",
        &kids(face, "Note", "review", &reviews).await?,
    );
    stack(&mut orders, "Review", &reviews);
    stack(
        &mut orders,
        "Comment",
        &kids(face, "Comment", "issue", &issues).await?,
    );
    stack(
        &mut orders,
        "Reaction",
        &kids(face, "Reaction", "issue", &issues).await?,
    );
    stack(
        &mut orders,
        "Demand",
        &kids(face, "Demand", "shield", &shields).await?,
    );
    stack(
        &mut orders,
        "Column",
        &kids(face, "Column", "project", &projects).await?,
    );
    stack(&mut orders, "Pull", &pulls);
    stack(&mut orders, "Issue", &issues);
    stack(&mut orders, "Shield", &shields);
    stack(&mut orders, "Project", &projects);
    for leaf in LEAVES {
        stack(&mut orders, leaf, &kids(face, leaf, "repo", &held).await?);
    }
    orders.push(("Repo", id));
    Ok(orders)
}

fn stack(orders: &mut Vec<(&'static str, i64)>, unit: &'static str, keys: &[i64]) {
    orders.extend(keys.iter().map(|key| (unit, *key)));
}
