mod door;
mod model;
mod runtime;

use axum::Router;
use axum::middleware::{self};
use axum::routing::post;
use clap::Parser;
use door::{close, found, stamp};
use keel::adapt::pg::Postgres;
use keel::{Core, Wire, app, bind, config};
use keel_blob::{Shed, Vault};
use keel_gate::Gate;
use keel_relay::Relay;
use model::shape;
use std::path::Path;
use std::sync::Arc;

#[derive(Parser)]
struct Cli {
    #[arg(default_value = ".")]
    root: String,
    #[arg(long, hide = true)]
    sidecar_stamp: Option<String>,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let _stamp = cli.sidecar_stamp;
    let root = cli.root;
    let runtime = match runtime::load() {
        Ok(runtime) => runtime,
        Err(err) => halt("environment", &err),
    };
    let mut cfg = config::load(Path::new(&root));
    if let Some(port) = runtime.port {
        cfg.listen.port = port;
    }
    match &runtime.pg {
        Some(url) => {
            if runtime.fresh {
                fresh(url).await;
            }
            let store = match Postgres::at(url).await {
                Ok(store) => store,
                Err(err) => halt("pg", &err.to_string()),
            };
            let core = raise(bind(shape(), store).await, &cfg);
            serve(core, &cfg, &runtime).await;
        }
        None => {
            let store = match cfg.open().await {
                Ok(store) => store,
                Err(err) => halt("config", &err.to_string()),
            };
            let core = raise(bind(shape(), store).await, &cfg);
            serve(core, &cfg, &runtime).await;
        }
    }
}

fn raise<W: Wire + 'static>(
    made: Result<Core<W>, keel::adapt::Error>,
    cfg: &config::Config,
) -> Arc<Core<W>> {
    let built = made
        .and_then(|core| core.identify("Actor"))
        .map(|core| match cfg.cache.kind {
            config::Hold::Memory => core,
            config::Hold::None => core.bare(),
        });
    match built {
        Ok(core) => core.share(),
        Err(err) => halt("bind", &err.to_string()),
    }
}

async fn serve<W: Wire + 'static>(
    core: Arc<Core<W>>,
    cfg: &config::Config,
    runtime: &runtime::Runtime,
) {
    if let Err(err) = core.seed().await {
        halt("seed", &err.to_string());
    }
    let door = match core.rig().await {
        Ok(door) => door,
        Err(err) => halt("rise", &err.to_string()),
    };
    let plate = Router::new()
        .route("/org", post(found::<W>))
        .route("/repo/{id}/close", post(close::<W>))
        .with_state(core.clone());
    let base = app(core.clone(), &cfg.listen.prefix).merge(plate);
    let shelved = match core.hoard(runtime) {
        Some(vault) => vault.shelf(base),
        None => base,
    };
    let router = door
        .wall(shelved)
        .layer(middleware::from_fn_with_state(core.clone(), stamp::<W>));
    let addr = format!("{}:{}", cfg.listen.host, cfg.listen.port);
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

trait Rise<W: Wire> {
    fn hoard(&self, runtime: &runtime::Runtime) -> Option<Vault<W>>;
    async fn rig(&self) -> Result<Gate<W>, keel::adapt::Error>;
    async fn hail(&self, login: &str) -> Result<i64, keel::adapt::Error>;
    async fn seed(&self) -> Result<(), keel::adapt::Error>;
}

impl<W: Wire + 'static> Rise<W> for Arc<Core<W>> {
    fn hoard(&self, runtime: &runtime::Runtime) -> Option<Vault<W>> {
        let held = runtime.s3.as_ref()?;
        let shed = Shed {
            endpoint: &held.endpoint,
            name: "codehull",
            region: "us-east-1",
            key: &held.key,
            secret: &held.secret,
        };
        match Vault::open(self.clone(), shed) {
            Ok(vault) => Some(vault),
            Err(err) => halt("vault", &err.to_string()),
        }
    }

    async fn rig(&self) -> Result<Gate<W>, keel::adapt::Error> {
        let gate = self.hail("gate").await?;
        let mail = self.hail("relay").await?;
        Relay::rise(self.clone(), mail).await?.run();
        Ok(Gate::rise(self.clone(), gate).await?.bar("barred"))
    }

    async fn hail(&self, login: &str) -> Result<i64, keel::adapt::Error> {
        let held = self
            .query(&format!(r#"from Actor where login = "{login}""#))
            .await?;
        match held.rows().first() {
            Some(row) => Ok(row.key()),
            None => {
                self.put(
                    "Actor",
                    &[("login", login), ("kind", "svc"), ("barred", "false")],
                )
                .await
            }
        }
    }

    async fn seed(&self) -> Result<(), keel::adapt::Error> {
        let sown = self
            .query(r#"from @grant where who = "anon" count"#)
            .await?;
        if sown.count() != Some(0) {
            return Ok(());
        }
        let sudo = self.sudo();
        let rows: [(&str, &str, &str, &str); 16] = [
            ("anon", "put", "Actor", r#"pred kind = "user""#),
            ("anon", "see", "Actor", "all"),
            ("all", "put", "Actor", r#"pred kind = "org""#),
            ("anon", "see", "Repo", r#"pred visibility = "public""#),
            ("all", "put", "Repo", r#"pred owner = "@me""#),
            ("all", "put", "Issue", r#"pred author = "@me""#),
            ("all", "set", "Issue", r#"pred author = "@me""#),
            ("all", "see", "Issue", r#"pred author = "@me""#),
            ("all", "put", "Comment", r#"pred author = "@me""#),
            ("all", "see", "Comment", r#"pred author = "@me""#),
            ("all", "put", "Reaction", r#"pred actor = "@me""#),
            ("all", "see", "Reaction", r#"pred actor = "@me""#),
            ("all", "put", "Asset", r#"pred owner = "@me""#),
            ("all", "see", "Asset", r#"pred owner = "@me""#),
            ("all", "put", "Review", r#"pred reviewer = "@me""#),
            ("all", "see", "Review", r#"pred reviewer = "@me""#),
        ];
        for (who, verb, unit, scope) in rows {
            sudo.put(
                "@grant",
                &[
                    ("who", who),
                    ("verb", verb),
                    ("unit", unit),
                    ("scope", scope),
                ],
            )
            .await?;
        }
        Ok(())
    }
}

async fn fresh(url: &str) {
    let mut store = match Postgres::at(url).await {
        Ok(store) => store,
        Err(err) => halt("fresh", &err.to_string()),
    };
    let wipe = store
        .script("DROP SCHEMA public CASCADE; CREATE SCHEMA public;")
        .await;
    if let Err(err) = wipe {
        halt("fresh", &err.to_string());
    }
}

fn halt(seat: &str, note: &str) -> ! {
    eprintln!("codehull: {seat}: {note}");
    std::process::exit(1)
}
