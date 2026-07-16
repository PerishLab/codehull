use axum::extract::{Path as Route, Request, State};
use axum::http::StatusCode;
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::post;
use axum::{Extension, Json, Router};
use keel::Ends;
use keel::adapt::pg::Postgres;
use keel::atom::{int, string};
use keel::config;
use keel::resource;
use keel::store::Store;
use keel::{Core, Graph, Operator, app, bind};
use keel_blob::Vault;
use keel_gate::Gate;
use keel_relay::Relay;
use serde_json::{Map, Value, json};
use std::env;
use std::path::Path;
use std::sync::Arc;

#[resource]
struct Actor {
    #[field(string, unique)]
    login: string,
    #[field(string)]
    kind: string,
    #[field(bool)]
    barred: bool,
    #[relation(Repo, many2many)]
    stars: Repo,
    #[relation(Repo, many2many)]
    watches: Repo,
    #[relation(Actor, many2many)]
    follows: Actor,
}

#[resource]
struct Email {
    #[field(string, unique)]
    mail: string,
    #[field(bool)]
    primary: bool,
    #[relation(Actor, many2one, root)]
    actor: Actor,
}

#[resource]
struct Team {
    #[field(string, unique = org)]
    name: string,
    #[field(string)]
    mode: string,
    #[relation(Actor, many2one, root)]
    org: Actor,
    #[relation(Actor, many2many, crew)]
    members: Actor,
    #[relation(Repo, many2many)]
    repos: Repo,
}

#[resource]
struct Topic {
    #[field(string, unique)]
    name: string,
}

#[resource]
struct Repo {
    #[field(string, unique = owner)]
    name: string,
    #[field(string)]
    visibility: string,
    #[field(string)]
    trunk: string,
    #[field(bool)]
    archived: bool,
    #[relation(Actor, many2one, root)]
    owner: Actor,
    #[relation(Repo, many2one, opt)]
    fork: Repo,
    #[relation(Topic, many2many)]
    topics: Topic,
}

#[resource]
struct Label {
    #[field(string, unique = repo)]
    name: string,
    #[field(string)]
    color: string,
    #[relation(Repo, many2one, root)]
    repo: Repo,
}

#[resource]
struct Runner {
    #[field(string)]
    name: string,
    #[field(string, unique)]
    token: string,
    #[field(string)]
    labels: string,
    #[field(string)]
    status: string,
    #[relation(Repo, many2one, root)]
    repo: Repo,
}

#[resource]
struct Run {
    #[field(string)]
    event: string,
    #[field(string)]
    status: string,
    #[field(string)]
    commit: string,
    #[relation(Repo, many2one, root)]
    repo: Repo,
}

#[resource]
struct Secret {
    #[field(string, unique = repo)]
    name: string,
    #[field(string)]
    data: string,
    #[relation(Repo, many2one, root)]
    repo: Repo,
}

#[resource]
struct Variable {
    #[field(string, unique = repo)]
    name: string,
    #[field(string)]
    value: string,
    #[relation(Repo, many2one, root)]
    repo: Repo,
}

#[resource]
struct Key {
    #[field(string)]
    title: string,
    #[field(string, unique)]
    print: string,
    #[relation(Repo, many2one, root)]
    repo: Repo,
}

#[resource]
struct Milestone {
    #[field(string)]
    title: string,
    #[field(int)]
    due: int,
    #[field(bool)]
    closed: bool,
    #[relation(Repo, many2one, root)]
    repo: Repo,
}

#[resource]
struct Issue {
    #[field(serial, scope = repo)]
    index: int,
    #[field(string)]
    title: string,
    #[field(string)]
    body: string,
    #[field(bool)]
    closed: bool,
    #[relation(Repo, many2one, root)]
    repo: Repo,
    #[relation(Actor, many2one)]
    author: Actor,
    #[relation(Milestone, many2one, opt)]
    milestone: Milestone,
    #[relation(Actor, many2many)]
    assignees: Actor,
    #[relation(Label, many2many)]
    labels: Label,
    #[relation(Issue, many2many)]
    blocks: Issue,
}

#[resource]
struct Comment {
    #[field(string)]
    body: string,
    #[relation(Issue, many2one, root)]
    issue: Issue,
    #[relation(Actor, many2one)]
    author: Actor,
}

#[resource]
struct Reaction {
    #[field(string, unique = (actor, issue))]
    emoji: string,
    #[relation(Issue, many2one, root)]
    issue: Issue,
    #[relation(Actor, many2one)]
    actor: Actor,
}

#[resource]
struct Pull {
    #[field(string)]
    base: string,
    #[field(string)]
    head: string,
    #[field(bool)]
    merged: bool,
    #[relation(Issue, one2one, root)]
    issue: Issue,
    #[relation(Repo, many2one, opt)]
    source: Repo,
}

#[resource]
struct Review {
    #[field(string)]
    state: string,
    #[field(string)]
    body: string,
    #[relation(Pull, many2one, root)]
    pull: Pull,
    #[relation(Actor, many2one)]
    reviewer: Actor,
}

#[resource]
struct Note {
    #[field(string)]
    path: string,
    #[field(int)]
    line: int,
    #[field(string)]
    body: string,
    #[relation(Review, many2one, root)]
    review: Review,
}

keel_gate::gate!(Actor);

keel_relay::relay!(Actor);

keel_blob::blob!(Actor);

fn shape() -> Graph {
    let mut graph = Graph::new();
    graph
        .plug::<Actor>()
        .plug::<Email>()
        .plug::<Team>()
        .plug::<Topic>()
        .plug::<Repo>()
        .plug::<Label>()
        .plug::<Runner>()
        .plug::<Run>()
        .plug::<Secret>()
        .plug::<Variable>()
        .plug::<Key>()
        .plug::<Milestone>()
        .plug::<Issue>()
        .plug::<Comment>()
        .plug::<Reaction>()
        .plug::<Pull>()
        .plug::<Review>()
        .plug::<Note>();
    plug(&mut graph);
    wire(&mut graph);
    stock(&mut graph);
    graph
}

#[tokio::main]
async fn main() {
    let root = env::args().nth(1).unwrap_or_else(|| ".".into());
    let cfg = config::load(Path::new(&root));
    match env::var("KEEL_PG") {
        Ok(url) => {
            if env::var("KEEL_FRESH").is_ok() {
                fresh(&url);
            }
            let core = raise(bind(shape(), Postgres::at(url)), &cfg);
            serve(core, &cfg).await;
        }
        Err(_) => {
            let store = match cfg.open() {
                Ok(store) => store,
                Err(err) => halt("config", &err.to_string()),
            };
            let core = raise(bind(shape(), store), &cfg);
            serve(core, &cfg).await;
        }
    }
}

fn raise<S: Store + 'static>(
    made: Result<Core<S>, keel::adapt::Error>,
    cfg: &config::Config,
) -> Arc<Core<S>> {
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

async fn serve<S: Store + 'static>(core: Arc<Core<S>>, cfg: &config::Config) {
    if let Err(err) = seed(&core) {
        halt("seed", &err.to_string());
    }
    let door = match rig(&core) {
        Ok(door) => door,
        Err(err) => halt("rise", &err.to_string()),
    };
    let plate = Router::new()
        .route("/org", post(found::<S>))
        .route("/repo/{id}/close", post(close::<S>))
        .with_state(core.clone());
    let base = app(core.clone(), &cfg.listen.prefix).merge(plate);
    let shelved = match hoard(&core) {
        Some(vault) => vault.shelf(base),
        None => base,
    };
    let router = door
        .wall(shelved)
        .layer(middleware::from_fn_with_state(core.clone(), stamp::<S>));
    let addr = format!("{}:{}", cfg.listen.host, cfg.listen.port);
    let bound = match tokio::net::TcpListener::bind(&addr).await {
        Ok(bound) => bound,
        Err(err) => halt("listen", &err.to_string()),
    };
    eprintln!("forgejo: ready on http://{addr}");
    if let Err(err) = axum::serve(bound, router).await {
        halt("serve", &err.to_string());
    }
}

fn fresh(url: &str) {
    let url = url.to_string();
    let done = std::thread::spawn(move || {
        let mut client = postgres::Client::connect(&url, postgres::NoTls)?;
        client.batch_execute("DROP SCHEMA public CASCADE; CREATE SCHEMA public;")
    })
    .join();
    match done {
        Ok(Ok(())) => {}
        Ok(Err(err)) => halt("fresh", &err.to_string()),
        Err(_) => halt("fresh", "reset thread panicked"),
    }
}

fn hoard<S: Store + 'static>(core: &Arc<Core<S>>) -> Option<Vault<S>> {
    let endpoint = env::var("KEEL_S3").ok()?;
    let key = env::var("KEEL_S3_KEY").unwrap_or_else(|_| "forgejo".into());
    let secret = env::var("KEEL_S3_SECRET").unwrap_or_else(|_| "forgejo123".into());
    match Vault::open(
        core.clone(),
        &endpoint,
        "forgejo",
        "us-east-1",
        &key,
        &secret,
    ) {
        Ok(vault) => Some(vault),
        Err(err) => halt("vault", &err.to_string()),
    }
}

fn halt(seat: &str, note: &str) -> ! {
    eprintln!("forgejo: {seat}: {note}");
    std::process::exit(1)
}

fn rig<S: Store + 'static>(core: &Arc<Core<S>>) -> Result<Gate<S>, keel::adapt::Error> {
    let gate = hail(core, "gate")?;
    let mail = hail(core, "relay")?;
    Relay::rise(core.clone(), mail)?.run();
    Ok(Gate::rise(core.clone(), gate)?.bar("barred"))
}

fn hail<S: Store>(core: &Arc<Core<S>>, login: &str) -> Result<i64, keel::adapt::Error> {
    let held = core.query(&format!(r#"from Actor where login = "{login}""#))?;
    match held.rows().first() {
        Some(row) => Ok(row.key()),
        None => core.put(
            "Actor",
            &[("login", login), ("kind", "svc"), ("barred", "false")],
        ),
    }
}

fn seed<S: Store>(core: &Arc<Core<S>>) -> Result<(), keel::adapt::Error> {
    let sown = core.query(r#"from @grant where who = "anon" count"#)?;
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
        )?;
    }
    Ok(())
}

async fn stamp<S: Store>(
    State(core): State<Arc<Core<S>>>,
    mut req: Request,
    next: Next,
) -> Response {
    let login = req
        .headers()
        .get("x-login")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    if let Some(login) = login
        && let Some(key) = whom(&core, &login)
    {
        req.extensions_mut().insert(Operator(key));
    }
    next.run(req).await
}

fn whom<S: Store>(core: &Core<S>, login: &str) -> Option<i64> {
    let pack = core
        .query(&format!(r#"from Actor where login = "{login}""#))
        .ok()?;
    pack.rows().first().map(keel::Row::key)
}

async fn found<S: Store + 'static>(
    State(core): State<Arc<Core<S>>>,
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
        .batch(|tx| {
            let org = tx.put(
                "Actor",
                &[("login", &name), ("kind", "org"), ("barred", "false")],
            )?;
            let team = tx.put(
                "Team",
                &[
                    ("name", "owners"),
                    ("mode", "admin"),
                    ("org", &org.to_string()),
                ],
            )?;
            tx.tie(
                "Team",
                "members",
                Ends {
                    left: team,
                    right: actor,
                },
                &[],
            )?;
            tx.put(
                "@grant",
                &[
                    ("who", &format!("team {team}")),
                    ("verb", "*"),
                    ("unit", "Actor"),
                    ("scope", &format!("row {org}")),
                ],
            )?;
            Ok(org)
        })
        .map_err(|_| StatusCode::FORBIDDEN)?;
    Ok((StatusCode::CREATED, Json(json!({ "id": org }))))
}

async fn close<S: Store + 'static>(
    State(core): State<Arc<Core<S>>>,
    Route(id): Route<i64>,
    op: Option<Extension<Operator>>,
) -> Result<StatusCode, StatusCode> {
    let who = op
        .map(|Extension(Operator(id))| id)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let face = core.of(who);
    let kids = |unit: &str| -> Result<Vec<i64>, keel::adapt::Error> {
        let pack = face.query(&format!(r#"from {unit} where repo = "{id}""#))?;
        Ok(pack.rows().iter().map(keel::Row::key).collect())
    };
    let issues = kids("Issue").map_err(|_| StatusCode::FORBIDDEN)?;
    let labels = kids("Label").map_err(|_| StatusCode::FORBIDDEN)?;
    let milestones = kids("Milestone").map_err(|_| StatusCode::FORBIDDEN)?;
    face.batch(|tx| {
        for key in &issues {
            tx.end("Issue", *key)?;
        }
        for key in &labels {
            tx.end("Label", *key)?;
        }
        for key in &milestones {
            tx.end("Milestone", *key)?;
        }
        tx.end("Repo", id)?;
        Ok(())
    })
    .map_err(|_| StatusCode::CONFLICT)?;
    Ok(StatusCode::NO_CONTENT)
}
