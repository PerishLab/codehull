mod haul;
mod line;
mod point;
pub(crate) mod port;
mod take;
mod verdict;
mod weld;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use codehull_repo::{Error, Object, Store};
use keel::{Core, Operator, Wire};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;

struct Dock<W: Wire> {
    core: Arc<Core<W>>,
    store: Arc<Store>,
}

impl<W: Wire> Clone for Dock<W> {
    fn clone(&self) -> Self {
        Self {
            core: self.core.clone(),
            store: self.store.clone(),
        }
    }
}

#[derive(Deserialize)]
struct Ingest {
    object: String,
    bundle: String,
}

type Fault = (StatusCode, Json<Value>);

pub(crate) fn routes<W: Wire + 'static>(core: Arc<Core<W>>, store: Arc<Store>) -> Router {
    let dock = Dock { core, store };
    Router::new()
        .route("/repo/{id}/git", get(show::<W>).put(make::<W>))
        .route("/repo/{id}/git/object", post(ingest::<W>))
        .route(
            "/repo/{id}/git/ref",
            get(point::read::<W>)
                .post(point::advance::<W>)
                .delete(point::drop::<W>),
        )
        .route("/repo/{id}/git/info/refs", get(haul::refs::<W>))
        .route("/repo/{id}/git/git-upload-pack", post(haul::upload::<W>))
        .route("/repo/{id}/git/git-receive-pack", post(take::take::<W>))
        .route("/repo/{id}/git/merge", post(weld::weld::<W>))
        .route(
            "/repo/{id}/git/verdict",
            get(verdict::read::<W>).post(verdict::cast::<W>),
        )
        .with_state(dock)
}

async fn make<W: Wire + 'static>(
    State(dock): State<Dock<W>>,
    Path(id): Path<i64>,
    op: Option<Extension<Operator>>,
) -> Result<Json<Value>, Fault> {
    let id = admit(&dock, id, actor(op)?).await?;
    let store = dock.store.clone();
    let repo = work(move || store.provision(id)).await?;
    Ok(Json(json!({ "id": repo.id() })))
}

async fn show<W: Wire + 'static>(
    State(dock): State<Dock<W>>,
    Path(id): Path<i64>,
    op: Option<Extension<Operator>>,
) -> Result<Json<Value>, Fault> {
    let id = admit(&dock, id, actor(op)?).await?;
    let store = dock.store.clone();
    let repo = work(move || store.repository(id)).await?;
    Ok(Json(json!({ "id": repo.id() })))
}

async fn ingest<W: Wire + 'static>(
    State(dock): State<Dock<W>>,
    Path(id): Path<i64>,
    op: Option<Extension<Operator>>,
    Json(body): Json<Ingest>,
) -> Result<Json<Value>, Fault> {
    let id = admit(&dock, id, actor(op)?).await?;
    let object = Object::parse(&body.object).map_err(fault)?;
    let bundle = STANDARD
        .decode(body.bundle)
        .map_err(|error| bad(format!("bundle is not base64: {error}")))?;
    let store = dock.store.clone();
    let held = object.clone();
    work(move || {
        let repo = store.repository(id)?;
        repo.ingest(&bundle, &held)
    })
    .await?;
    Ok(Json(json!({ "object": object.hex() })))
}

async fn admit<W: Wire + 'static>(dock: &Dock<W>, id: i64, who: i64) -> Result<u64, Fault> {
    let key = u64::try_from(id).map_err(|_| bad("repository id must be positive"))?;
    let held = dock
        .core
        .of(who)
        .query(&format!(
            r#"from Repo where id = "{id}" and owner = "{who}""#
        ))
        .await
        .map_err(|_| deny())?;
    if held.rows().len() != 1 {
        return Err(deny());
    }
    Ok(key)
}

fn actor(op: Option<Extension<Operator>>) -> Result<i64, Fault> {
    op.map(|Extension(Operator(actor))| actor).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "unauthorized" })),
        )
    })
}

async fn work<F, T>(job: F) -> Result<T, Fault>
where
    F: FnOnce() -> Result<T, Error> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(job)
        .await
        .map_err(|error| fault(Error::Io(format!("repository worker failed: {error}"))))?
        .map_err(fault)
}

fn bad(note: impl Into<String>) -> Fault {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({ "error": note.into() })),
    )
}

fn deny() -> Fault {
    (StatusCode::FORBIDDEN, Json(json!({ "error": "forbidden" })))
}

fn fault(error: Error) -> Fault {
    let status = match error {
        Error::Conflict(_) | Error::Foreign(_) => StatusCode::CONFLICT,
        Error::Invalid(_) => StatusCode::BAD_REQUEST,
        Error::Git(_) => StatusCode::UNPROCESSABLE_ENTITY,
        Error::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (status, Json(json!({ "error": error.to_string() })))
}
