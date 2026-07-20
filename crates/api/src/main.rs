mod door;
mod model;

use axum::Router;
use axum::middleware::{self};
use axum::routing::post;
use door::{close, found, stamp};
use keel::adapt::pg::Postgres;
use keel::{Core, Wire, app, bind, config};
use keel_blob::Vault;
use keel_gate::Gate;
use keel_relay::Relay;
use model::shape;
use std::env;
use std::path::Path;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let root = env::args().nth(1).unwrap_or_else(|| ".".into());
    let cfg = config::load(Path::new(&root));
    match env::var("KEEL_PG") {
        Ok(url) => {
            if env::var("KEEL_FRESH").is_ok() {
                fresh(&url).await;
            }
            let store = match Postgres::at(url).await {
                Ok(store) => store,
                Err(err) => halt("pg", &err.to_string()),
            };
            let core = raise(bind(shape(), store).await, &cfg);
            serve(core, &cfg).await;
        }
        Err(_) => {
            let store = match cfg.open().await {
                Ok(store) => store,
                Err(err) => halt("config", &err.to_string()),
            };
            let core = raise(bind(shape(), store).await, &cfg);
            serve(core, &cfg).await;
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

async fn serve<W: Wire + 'static>(core: Arc<Core<W>>, cfg: &config::Config) {
    if let Err(err) = seed(&core).await {
        halt("seed", &err.to_string());
    }
    let door = match rig(&core).await {
        Ok(door) => door,
        Err(err) => halt("rise", &err.to_string()),
    };
    let plate = Router::new()
        .route("/org", post(found::<W>))
        .route("/repo/{id}/close", post(close::<W>))
        .with_state(core.clone());
    let base = app(core.clone(), &cfg.listen.prefix).merge(plate);
    let shelved = match hoard(&core) {
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
    eprintln!("codehull: ready on http://{addr}");
    if let Err(err) = axum::serve(bound, router).await {
        halt("serve", &err.to_string());
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

fn hoard<W: Wire + 'static>(core: &Arc<Core<W>>) -> Option<Vault<W>> {
    let endpoint = env::var("KEEL_S3").ok()?;
    let key = env::var("KEEL_S3_KEY").unwrap_or_else(|_| "codehull".into());
    let secret = env::var("KEEL_S3_SECRET").unwrap_or_else(|_| "codehull123".into());
    match Vault::open(
        core.clone(),
        &endpoint,
        "codehull",
        "us-east-1",
        &key,
        &secret,
    ) {
        Ok(vault) => Some(vault),
        Err(err) => halt("vault", &err.to_string()),
    }
}

fn halt(seat: &str, note: &str) -> ! {
    eprintln!("codehull: {seat}: {note}");
    std::process::exit(1)
}

async fn rig<W: Wire + 'static>(core: &Arc<Core<W>>) -> Result<Gate<W>, keel::adapt::Error> {
    let gate = hail(core, "gate").await?;
    let mail = hail(core, "relay").await?;
    Relay::rise(core.clone(), mail).await?.run();
    Ok(Gate::rise(core.clone(), gate).await?.bar("barred"))
}

async fn hail<W: Wire>(core: &Arc<Core<W>>, login: &str) -> Result<i64, keel::adapt::Error> {
    let held = core
        .query(&format!(r#"from Actor where login = "{login}""#))
        .await?;
    match held.rows().first() {
        Some(row) => Ok(row.key()),
        None => {
            core.put(
                "Actor",
                &[("login", login), ("kind", "svc"), ("barred", "false")],
            )
            .await
        }
    }
}

async fn seed<W: Wire>(core: &Arc<Core<W>>) -> Result<(), keel::adapt::Error> {
    let sown = core
        .query(r#"from @grant where who = "anon" count"#)
        .await?;
    if sown.count() != Some(0) {
        return Ok(());
    }
    let sudo = core.sudo();
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
