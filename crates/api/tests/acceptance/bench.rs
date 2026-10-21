#![allow(dead_code)]

use super::issuer::Issuer;
use super::wire::{Head, Inside, Load, Reply, Wire, shape};
use api::runtime::{Kind, Runtime};
use api::{Estate, Raised, router, shell};
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::Request;
use keel::adapt::db::Sqlite;
use keel::adapt::pg::Postgres;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use tokio::net::TcpListener;
use tokio::runtime::{Builder, Handle, Runtime as Engine};
use tower::ServiceExt;

struct Unit;

static SERIAL: Mutex<Unit> = Mutex::new(Unit);

pub struct Bench {
    pub origin: String,
    pub base: String,
    pub nest: PathBuf,
    pub issuer: Issuer,
    pub blob: bool,
    pub ssh: Option<u16>,
    pub app: Router,
    sudo: String,
    engine: Engine,
    room: tempfile::TempDir,
    hold: Option<MutexGuard<'static, Unit>>,
}

struct Built {
    app: Router,
    sudo: String,
    served: Option<String>,
    ssh: Option<u16>,
}

impl Bench {
    pub fn new() -> Self {
        Self::raise(false, false)
    }

    pub fn seat(socket: bool) -> Self {
        Self::raise(true, socket)
    }

    fn raise(repo: bool, socket: bool) -> Self {
        let room = tempfile::tempdir().expect("bench room");
        let nest = room.path().to_path_buf();
        std::fs::write(nest.join("codehull.toml"), "[store]\nkind = \"memory\"\n")
            .expect("bench config");
        let issuer = Issuer::new();
        let runtime = plan(&nest, &issuer);
        let hold = (runtime.store.kind == Kind::Pg)
            .then(|| SERIAL.lock().unwrap_or_else(|poison| poison.into_inner()));
        let mut builder = Builder::new_multi_thread();
        if !socket {
            builder.worker_threads(2);
        }
        let engine = builder.enable_all().build().expect("bench engine");
        let seats = repo.then(|| nest.join("seats"));
        let built = engine.block_on(stock(&runtime, seats, socket));
        let origin = built.served.unwrap_or_default();
        Self {
            base: format!("{origin}/api"),
            origin,
            nest,
            issuer,
            blob: !runtime.blob.endpoint.is_empty(),
            ssh: built.ssh,
            app: built.app,
            sudo: built.sudo,
            engine,
            room,
            hold,
        }
    }

    pub fn wire(&self) -> Wire {
        Wire::within(
            &self.base,
            inside(self.app.clone(), self.engine.handle().clone()),
        )
    }

    pub fn crown(&self) -> Head {
        vec![("authorization".into(), format!("sudo {}", self.sudo))]
    }
}

fn plan(nest: &Path, issuer: &Issuer) -> Runtime {
    let mut runtime = api::runtime::load(nest).expect("bench runtime");
    runtime.oidc.issuer = issuer.url.clone();
    runtime.oidc.audience = "codehull".to_string();
    if !runtime.store.url.is_empty() {
        runtime.store.kind = Kind::Pg;
        runtime.fresh = true;
    }
    if !runtime.blob.endpoint.is_empty() {
        fill(&mut runtime.blob.key, "codehull");
        fill(&mut runtime.blob.secret, "codehull123");
    }
    runtime
}

fn fill(field: &mut String, fallback: &str) {
    if field.is_empty() {
        *field = fallback.to_string();
    }
}

async fn stock(runtime: &Runtime, seats: Option<PathBuf>, socket: bool) -> Built {
    if runtime.store.kind != Kind::Pg {
        let store = Sqlite::memory().await.expect("memory store");
        return erect(store, runtime, seats, socket).await;
    }
    let mut store = Postgres::at(&runtime.store.url).await.expect("pg store");
    store.wipe().await.expect("pg wipe");
    erect(store, runtime, seats, socket).await
}

async fn erect<W: keel::Wire + 'static>(
    store: W,
    runtime: &Runtime,
    seats: Option<PathBuf>,
    socket: bool,
) -> Built {
    let Raised { core, sudo } = Estate(store)
        .raise(runtime, true)
        .await
        .unwrap_or_else(refused);
    let repo = seats
        .as_deref()
        .map(|path| Arc::new(codehull_repo::Store::create(path).expect("repo store")));
    let app = router(core.clone(), runtime, repo.clone(), true)
        .await
        .unwrap_or_else(refused);
    let mut built = Built {
        app: app.clone(),
        sudo: sudo.expect("sudo token"),
        served: None,
        ssh: None,
    };
    if !socket {
        return built;
    }
    built.served = Some(serve(app).await);
    if let (Some(store), Some(path)) = (repo, seats) {
        let key = path.with_file_name("host.key");
        let held = codehull_ssh::key(&key).expect("host key");
        built.ssh = Some(listen(core, store, held).await);
    }
    built
}

async fn serve(app: Router) -> String {
    let socket = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("http listener");
    let at = socket.local_addr().expect("http address");
    tokio::spawn(async move { axum::serve(socket, app).await });
    format!("http://{at}")
}

async fn listen<W: keel::Wire + 'static>(
    core: Arc<keel::Core<W>>,
    store: Arc<codehull_repo::Store>,
    held: codehull_ssh::PrivateKey,
) -> u16 {
    let socket = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("ssh listener");
    let port = socket.local_addr().expect("ssh address").port();
    tokio::spawn(async move { shell(core, store, held, socket).await });
    port
}

fn refused<T>((seat, note): (&'static str, String)) -> T {
    panic!("codehull: {seat}: {note}")
}

fn inside(app: Router, handle: Handle) -> Inside {
    Box::new(move |verb: &str, url: &str, head: &Head, load: Load<'_>| {
        handle.block_on(send(app.clone(), ask(verb, url, head, load)))
    })
}

fn ask(verb: &str, url: &str, head: &Head, load: Load<'_>) -> Request<Body> {
    let mut made = Request::builder().method(verb).uri(url);
    if !matches!(load, Load::Raw(_)) {
        made = made.header("content-type", "application/json");
    }
    for (key, value) in head {
        made = made.header(key.as_str(), value.as_str());
    }
    let body = match load {
        Load::Bare => Body::empty(),
        Load::Json(text) => Body::from(text.to_string()),
        Load::Raw(bytes) => Body::from(bytes.to_vec()),
    };
    made.body(body).expect("in-process request")
}

async fn send(app: Router, request: Request<Body>) -> Reply {
    let res = app.oneshot(request).await.expect("router");
    let status = res.status().as_u16();
    let head = res
        .headers()
        .iter()
        .map(|(key, value)| {
            (
                key.as_str().to_string(),
                value.to_str().unwrap_or_default().to_string(),
            )
        })
        .collect();
    let bytes = to_bytes(res.into_body(), 64 << 20)
        .await
        .expect("router body");
    shape(status, head, bytes.to_vec())
}
