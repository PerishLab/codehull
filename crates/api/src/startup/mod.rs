mod estate;
pub mod observation;
mod shell;
mod store;
pub use estate::{Estate, Raised};
pub use shell::shell;
use store::memory;

use crate::artifact::{self, Artifact};
use crate::door::{close, found, propose};
use crate::halt;
use crate::rig::Berth;
use crate::runtime::{Kind, Runtime};
use crate::seam::warden::Warden;
use crate::seam::{Seam, admit};
use axum::Router;
use axum::middleware;
use axum::routing::post;
use keel::adapt::db::Sqlite;
use keel::adapt::pg::Postgres;
use keel::{Core, Status, Wire, app};
use keel_blob::{Shed, Vault};
use std::path::Path;
use std::sync::Arc;

const PREFIX: &str = "/api";

pub struct Seat<'a>(pub &'a str);

impl Seat<'_> {
    #[locus::trace(with = plumb::observation::view())]
    pub async fn bootstrap(&self, specs: &[String]) {
        let runtime = self.read();
        let sudo = match artifact::bootstrap(Path::new(self.0), specs) {
            Ok(sudo) => sudo,
            Err(err) => halt("artifact", &err),
        };
        match runtime.store.kind {
            Kind::Pg => Estate(open(&runtime).await).provision(&sudo).await,
            Kind::File => Estate(self.file(&runtime).await).provision(&sudo).await,
            Kind::Memory => halt("store", "bootstrap requires a durable store"),
        }
    }

    pub async fn serve(&self) {
        let runtime = self.read();
        if runtime.fresh && runtime.store.kind == Kind::File {
            halt("fresh", "wipe is supported only on postgres and memory");
        }
        let born = runtime.fresh || runtime.store.kind == Kind::Memory;
        let repo = self.repo(&runtime);
        match runtime.store.kind {
            Kind::Pg => {
                Estate(open(&runtime).await)
                    .serve(&runtime, repo, born)
                    .await
            }
            Kind::File => {
                Estate(self.file(&runtime).await)
                    .serve(&runtime, repo, born)
                    .await
            }
            Kind::Memory => Estate(memory().await).serve(&runtime, repo, born).await,
        }
    }

    fn repo(&self, runtime: &Runtime) -> Option<codehull_repo::Store> {
        if runtime.repo.path.is_empty() {
            return None;
        }
        let path = plumb::config::rebase(Path::new(&runtime.repo.path), Path::new(self.0));
        if let Some(parent) = path.parent()
            && let Err(err) = std::fs::create_dir_all(parent)
        {
            halt("repo", &err.to_string());
        }
        match codehull_repo::Store::create(&path) {
            Ok(store) => Some(store),
            Err(err) => halt("repo", &err.to_string()),
        }
    }

    fn read(&self) -> Runtime {
        let mut runtime = match crate::runtime::load(Path::new(self.0)) {
            Ok(runtime) => runtime,
            Err(err) => halt("config", &err.to_string()),
        };
        if !runtime.ssh.key.is_empty() {
            let held = plumb::config::rebase(Path::new(&runtime.ssh.key), Path::new(self.0));
            runtime.ssh.key = held.to_string_lossy().into_owned();
        }
        runtime
    }

    async fn file(&self, runtime: &Runtime) -> Sqlite {
        let path = plumb::config::rebase(Path::new(&runtime.store.path), Path::new(self.0));
        if let Some(parent) = path.parent()
            && let Err(err) = std::fs::create_dir_all(parent)
        {
            halt("store", &err.to_string());
        }
        match Sqlite::file(&path).await {
            Ok(store) => store,
            Err(err) => halt("store", &err.to_string()),
        }
    }
}

async fn open(runtime: &Runtime) -> Postgres {
    let mut store = match Postgres::at(&runtime.store.url).await {
        Ok(store) => store,
        Err(err) => halt("pg", &err.to_string()),
    };
    if runtime.fresh
        && let Err(err) = store.wipe().await
    {
        halt("fresh", &err.to_string());
    }
    store
}

pub(super) async fn custody<W: Wire + 'static>(
    estate: &mut keel::Bootstrap<W>,
    sudo: &Artifact,
) -> Result<String, String> {
    if let Some(bytes) = sudo.load()? {
        return artifact::text(&bytes);
    }
    if estate.status().await.map_err(|err| err.to_string())? != Status::Vacant {
        return Err("sudo custody is absent for an occupied estate".to_string());
    }
    let minted = estate.mint().await.map_err(|err| err.to_string())?;
    if sudo.keep(minted.as_bytes())? {
        return Ok(minted);
    }
    let bytes = sudo
        .load()?
        .ok_or_else(|| "sudo artifact lost during creation".to_string())?;
    artifact::text(&bytes)
}

pub(super) fn hold<W: Wire + 'static>(core: Core<W>) -> Result<Arc<Core<W>>, keel::adapt::Error> {
    core.identify("Actor").map(Core::share)
}

pub type Refusal = (&'static str, String);

pub async fn router<W: Wire + 'static>(
    core: Arc<Core<W>>,
    runtime: &Runtime,
    repo: Option<Arc<codehull_repo::Store>>,
    born: bool,
) -> Result<Router, Refusal> {
    let berth = Berth(&core);
    let raised = match born {
        true => berth.seed().await,
        false => berth.rig().await,
    };
    let door = raised.map_err(|err| ("rig", err.to_string()))?;
    let plate = Router::new()
        .route("/org", post(found::<W>))
        .route("/propose", post(propose::<W>))
        .route("/repo/{id}/close", post(close::<W>))
        .with_state(core.clone());
    let mut base = app(core.clone(), &runtime.listen.prefix).merge(plate);
    if let Some(store) = repo {
        base = base.merge(crate::ground::routes(core.clone(), store));
    }
    let shelved = match hoard(&core, runtime)? {
        Some(vault) => vault.shelf(base),
        None => base,
    };
    let seam = Seam {
        core: core.clone(),
        gate: door.clone(),
        warden: warden(runtime)?,
    };
    let api = door
        .screen(shelved)
        .layer(middleware::from_fn_with_state(seam, admit::<W>));
    Ok(Router::new().nest(PREFIX, api))
}

fn hoard<W: Wire + 'static>(
    core: &Arc<Core<W>>,
    runtime: &Runtime,
) -> Result<Option<Vault<W>>, Refusal> {
    if runtime.blob.endpoint.is_empty() {
        return Ok(None);
    }
    let shed = Shed {
        endpoint: &runtime.blob.endpoint,
        name: "codehull",
        region: "us-east-1",
        key: &runtime.blob.key,
        secret: &runtime.blob.secret,
    };
    Vault::open(core.clone(), shed)
        .map(Some)
        .map_err(|err| ("vault", err.to_string()))
}

fn warden(runtime: &Runtime) -> Result<Arc<Warden>, Refusal> {
    Warden::open(&runtime.oidc.issuer, &runtime.oidc.audience)
        .map(Arc::new)
        .map_err(|err| ("oidc", err))
}
