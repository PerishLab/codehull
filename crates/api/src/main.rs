use axum::extract::{Path as Route, Request, State};
use axum::http::StatusCode;
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::post;
use axum::{Extension, Json, Router};
use keel::Ends;
use keel::Wire;
use keel::adapt::pg::Postgres;
use keel::atom::url as link;
use keel::atom::{int, string};
use keel::config;
use keel::resource;
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
struct UserKey {
    #[field(string)]
    title: string,
    #[field(string, unique)]
    print: string,
    #[field(string)]
    kind: string,
    #[relation(Actor, many2one, root)]
    owner: Actor,
}

#[resource]
struct Mirror {
    #[field(url)]
    remote: link,
    #[field(int)]
    interval: int,
    #[relation(Repo, one2one, root)]
    repo: Repo,
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
struct OrgLabel {
    #[field(string, unique = org)]
    name: string,
    #[field(string)]
    color: string,
    #[relation(Actor, many2one, root)]
    org: Actor,
}

#[resource]
struct OrgRunner {
    #[field(string)]
    name: string,
    #[field(string, unique)]
    token: string,
    #[field(string)]
    labels: string,
    #[field(string)]
    status: string,
    #[relation(Actor, many2one, root)]
    org: Actor,
}

#[resource]
struct OrgSecret {
    #[field(string, unique = org)]
    name: string,
    #[field(string)]
    data: string,
    #[relation(Actor, many2one, root)]
    org: Actor,
}

#[resource]
struct Project {
    #[field(string)]
    title: string,
    #[field(bool)]
    closed: bool,
    #[relation(Repo, many2one, root)]
    repo: Repo,
}

#[resource]
struct Column {
    #[field(string)]
    title: string,
    #[field(int)]
    sort: int,
    #[relation(Project, many2one, root)]
    project: Project,
    #[relation(Issue, many2many, spot = int)]
    cards: Issue,
}

#[resource]
struct Release {
    #[field(string, unique = repo)]
    tag: string,
    #[field(string)]
    title: string,
    #[field(string)]
    body: string,
    #[field(bool)]
    draft: bool,
    #[relation(Repo, many2one, root)]
    repo: Repo,
    #[relation(Actor, many2one)]
    author: Actor,
}

#[resource]
struct Shield {
    #[field(string, unique = repo)]
    branch: string,
    #[field(bool)]
    force: bool,
    #[field(int)]
    approvals: int,
    #[relation(Repo, many2one, root)]
    repo: Repo,
}

#[resource]
struct Package {
    #[field(string, unique = repo)]
    name: string,
    #[field(string)]
    kind: string,
    #[field(string)]
    version: string,
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
        .plug::<UserKey>()
        .plug::<Mirror>()
        .plug::<Email>()
        .plug::<Team>()
        .plug::<Topic>()
        .plug::<Repo>()
        .plug::<Label>()
        .plug::<OrgLabel>()
        .plug::<OrgRunner>()
        .plug::<OrgSecret>()
        .plug::<Project>()
        .plug::<Column>()
        .plug::<Release>()
        .plug::<Shield>()
        .plug::<Package>()
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

async fn stamp<W: Wire>(
    State(core): State<Arc<Core<W>>>,
    mut req: Request,
    next: Next,
) -> Response {
    let login = req
        .headers()
        .get("x-login")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    if let Some(login) = login
        && let Some(key) = whom(&core, &login).await
    {
        req.extensions_mut().insert(Operator(key));
    }
    next.run(req).await
}

async fn whom<W: Wire>(core: &Core<W>, login: &str) -> Option<i64> {
    let pack = core
        .query(&format!(r#"from Actor where login = "{login}""#))
        .await
        .ok()?;
    pack.rows().first().map(keel::Row::key)
}

async fn found<W: Wire + 'static>(
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

async fn kids<W: Wire>(
    face: &keel::Face<'_, W>,
    id: i64,
    unit: &str,
) -> Result<Vec<i64>, keel::adapt::Error> {
    let pack = face
        .query(&format!(r#"from {unit} where repo = "{id}""#))
        .await?;
    Ok(pack.rows().iter().map(keel::Row::key).collect())
}

async fn close<W: Wire + 'static>(
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
    let labels = kids(&face, id, "Label")
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
            tx.end("Label", *key).await?;
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
