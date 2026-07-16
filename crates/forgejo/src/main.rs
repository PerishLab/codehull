use axum::extract::{Request, State};
use axum::middleware::{self, Next};
use axum::response::Response;
use keel::adapt::db::Sqlite;
use keel::atom::{int, string};
use keel::config;
use keel::resource;
use keel::{Core, Graph, Operator, app, bind};
use keel_gate::Gate;
use keel_relay::Relay;
use std::env;
use std::path::Path;
use std::sync::Arc;

#[resource]
struct Actor {
    #[field(string, unique)]
    login: string,
    #[relation(Repo, many2many)]
    stars: Repo,
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
    #[relation(Actor, many2many)]
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

keel_gate::gate!(Actor);

keel_relay::relay!(Actor);

#[tokio::main]
async fn main() {
    let root = env::args().nth(1).unwrap_or_else(|| ".".into());
    let cfg = config::load(Path::new(&root));
    let store = match cfg.open() {
        Ok(store) => store,
        Err(err) => halt("config", &err.to_string()),
    };
    let mut graph = Graph::new();
    graph
        .plug::<Actor>()
        .plug::<Email>()
        .plug::<Team>()
        .plug::<Topic>()
        .plug::<Repo>()
        .plug::<Label>()
        .plug::<Milestone>()
        .plug::<Issue>()
        .plug::<Comment>()
        .plug::<Pull>()
        .plug::<Review>();
    plug(&mut graph);
    wire(&mut graph);
    let made = bind(graph, store)
        .and_then(|core| core.identify("Actor"))
        .map(|core| match cfg.cache.kind {
            config::Hold::Memory => core,
            config::Hold::None => core.bare(),
        });
    let core = match made {
        Ok(core) => core.share(),
        Err(err) => halt("bind", &err.to_string()),
    };
    if let Err(err) = seed(&core) {
        halt("seed", &err.to_string());
    }
    let door = match rig(&core) {
        Ok(door) => door,
        Err(err) => halt("rise", &err.to_string()),
    };
    let router = door
        .wall(app(core.clone(), &cfg.listen.prefix))
        .layer(middleware::from_fn_with_state(core.clone(), stamp));
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

fn halt(seat: &str, note: &str) -> ! {
    eprintln!("forgejo: {seat}: {note}");
    std::process::exit(1)
}

fn rig(core: &Arc<Core<Sqlite>>) -> Result<Gate<Sqlite>, keel::adapt::Error> {
    let gate = post(core, "gate")?;
    let mail = post(core, "relay")?;
    Relay::rise(core.clone(), mail)?.run();
    Gate::rise(core.clone(), gate)
}

fn post(core: &Arc<Core<Sqlite>>, login: &str) -> Result<i64, keel::adapt::Error> {
    let held = core.query(&format!(r#"from Actor where login = "{login}""#))?;
    match held.rows().first() {
        Some(row) => Ok(row.key()),
        None => core.put("Actor", &[("login", login)]),
    }
}

fn seed(core: &Arc<Core<Sqlite>>) -> Result<(), keel::adapt::Error> {
    let sown = core.query(r#"from @grant where who = "anon" count"#)?;
    if sown.count() != Some(0) {
        return Ok(());
    }
    let sudo = core.sudo();
    let rows: [(&str, &str, &str, &str); 9] = [
        ("anon", "put", "Actor", "all"),
        ("anon", "see", "Actor", "all"),
        ("anon", "see", "Repo", r#"pred visibility = "public""#),
        ("all", "put", "Repo", r#"pred owner = "@me""#),
        ("all", "put", "Issue", r#"pred author = "@me""#),
        ("all", "set", "Issue", r#"pred author = "@me""#),
        ("all", "see", "Issue", r#"pred author = "@me""#),
        ("all", "put", "Comment", r#"pred author = "@me""#),
        ("all", "see", "Comment", r#"pred author = "@me""#),
    ];
    for (who, verb, unit, scope) in rows {
        sudo.put(
            "@grant",
            &[("who", who), ("verb", verb), ("unit", unit), ("scope", scope)],
        )?;
    }
    Ok(())
}

async fn stamp(State(core): State<Arc<Core<Sqlite>>>, mut req: Request, next: Next) -> Response {
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

fn whom(core: &Core<Sqlite>, login: &str) -> Option<i64> {
    let pack = core
        .query(&format!(r#"from Actor where login = "{login}""#))
        .ok()?;
    pack.rows().first().map(keel::Row::key)
}
