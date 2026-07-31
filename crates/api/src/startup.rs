use crate::artifact::{self, Artifact};
use crate::door::{close, found};
use crate::halt;
use crate::model::shape;
use crate::rig::Berth;
use crate::runtime::{Hold, Kind, Runtime};
use crate::seam::{Seam, admit};
use crate::warden::Warden;
use axum::Router;
use axum::middleware;
use axum::routing::post;
use keel::adapt::db::Sqlite;
use keel::adapt::pg::Postgres;
use keel::{Core, Status, Wire, app, bind};
use keel_blob::{Shed, Vault};
use std::path::Path;
use std::sync::Arc;

const PREFIX: &str = "/api";

pub(crate) struct Seat<'a>(pub(crate) &'a str);

impl Seat<'_> {
    pub(crate) async fn bootstrap(&self, specs: &[String]) {
        let runtime = self.read();
        let sudo = match artifact::bootstrap(Path::new(self.0), specs) {
            Ok(sudo) => sudo,
            Err(err) => halt("artifact", &err),
        };
        match runtime.store.kind {
            Kind::Pg => provision(open(&runtime).await, &sudo).await,
            Kind::File => provision(self.file(&runtime).await, &sudo).await,
            Kind::Memory => halt("store", "bootstrap requires a durable store"),
        }
    }

    pub(crate) async fn serve(&self) {
        let runtime = self.read();
        if runtime.fresh && runtime.store.kind == Kind::File {
            halt("fresh", "wipe is supported only on postgres and memory");
        }
        let born = runtime.fresh || runtime.store.kind == Kind::Memory;
        match runtime.store.kind {
            Kind::Pg => {
                let held = start(open(&runtime).await, born).await;
                serve(raise(held, &runtime), &runtime, born).await;
            }
            Kind::File => {
                let held = start(self.file(&runtime).await, born).await;
                serve(raise(held, &runtime), &runtime, born).await;
            }
            Kind::Memory => {
                let held = start(memory().await, born).await;
                serve(raise(held, &runtime), &runtime, born).await;
            }
        }
    }

    fn read(&self) -> Runtime {
        match crate::runtime::load(Path::new(self.0)) {
            Ok(runtime) => runtime,
            Err(err) => halt("config", &err.to_string()),
        }
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

async fn memory() -> Sqlite {
    match Sqlite::memory().await {
        Ok(store) => store,
        Err(err) => halt("store", &err.to_string()),
    }
}

async fn start<W: Wire + 'static>(store: W, born: bool) -> Result<Core<W>, keel::adapt::Error> {
    if !born {
        return bind(shape(), store).await;
    }
    let mut estate = keel::bootstrap(shape(), store)?;
    let token = estate.mint().await?;
    eprintln!("codehull: sudo token {token}");
    estate.seal(&token).await
}

async fn provision<W: Wire + 'static>(store: W, sudo: &Artifact) {
    let mut estate = match keel::bootstrap(shape(), store) {
        Ok(estate) => estate,
        Err(err) => halt("bootstrap", &err.to_string()),
    };
    let token = match custody(&mut estate, sudo).await {
        Ok(token) => token,
        Err(err) => halt("bootstrap", &err),
    };
    let core = match estate.seal(&token).await.and_then(hold) {
        Ok(core) => core,
        Err(err) => halt("bootstrap", &err.to_string()),
    };
    if let Err(err) = Berth(&core).seed().await {
        halt("bootstrap", &err.to_string());
    }
}

async fn custody<W: Wire + 'static>(
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

fn hold<W: Wire + 'static>(core: Core<W>) -> Result<Arc<Core<W>>, keel::adapt::Error> {
    core.identify("Actor").map(Core::share)
}

fn raise<W: Wire + 'static>(
    made: Result<Core<W>, keel::adapt::Error>,
    runtime: &Runtime,
) -> Arc<Core<W>> {
    let built = made
        .map(|core| match runtime.cache.kind {
            Hold::Memory => core,
            Hold::None => core.bare(),
        })
        .and_then(hold);
    match built {
        Ok(core) => core,
        Err(err) => halt("bind", &err.to_string()),
    }
}

async fn serve<W: Wire + 'static>(core: Arc<Core<W>>, runtime: &Runtime, born: bool) {
    let berth = Berth(&core);
    let raised = match born {
        true => berth.seed().await,
        false => berth.rig().await,
    };
    let door = match raised {
        Ok(door) => door,
        Err(err) => halt("rig", &err.to_string()),
    };
    let plate = Router::new()
        .route("/org", post(found::<W>))
        .route("/repo/{id}/close", post(close::<W>))
        .with_state(core.clone());
    let base = app(core.clone(), &runtime.listen.prefix).merge(plate);
    let shelved = match hoard(&core, runtime) {
        Some(vault) => vault.shelf(base),
        None => base,
    };
    let seam = Seam {
        core: core.clone(),
        gate: door.clone(),
        warden: warden(runtime),
    };
    let api = door
        .screen(shelved)
        .layer(middleware::from_fn_with_state(seam, admit::<W>));
    let router = Router::new().nest(PREFIX, api);
    let addr = format!("{}:{}", runtime.listen.host, runtime.listen.port);
    let bound = match tokio::net::TcpListener::bind(&addr).await {
        Ok(bound) => bound,
        Err(err) => halt("listen", &err.to_string()),
    };
    let live = match bound.local_addr() {
        Ok(live) => live,
        Err(err) => halt("listen", &err.to_string()),
    };
    eprintln!(
        "{}",
        serde_json::json!({ "role": "api", "endpoint": format!("http://{live}") })
    );
    if let Err(err) = axum::serve(bound, router).await {
        halt("serve", &err.to_string());
    }
}

fn hoard<W: Wire + 'static>(core: &Arc<Core<W>>, runtime: &Runtime) -> Option<Vault<W>> {
    if runtime.blob.endpoint.is_empty() {
        return None;
    }
    let shed = Shed {
        endpoint: &runtime.blob.endpoint,
        name: "codehull",
        region: "us-east-1",
        key: &runtime.blob.key,
        secret: &runtime.blob.secret,
    };
    match Vault::open(core.clone(), shed) {
        Ok(vault) => Some(vault),
        Err(err) => halt("vault", &err.to_string()),
    }
}

fn warden(runtime: &Runtime) -> Arc<Warden> {
    match Warden::open(&runtime.oidc.issuer, &runtime.oidc.audience) {
        Ok(warden) => Arc::new(warden),
        Err(err) => halt("oidc", &err),
    }
}
