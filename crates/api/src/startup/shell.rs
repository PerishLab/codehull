use crate::halt;
use crate::runtime::Runtime;
use keel::{Core, Wire};
use std::path::Path;
use std::sync::Arc;
use tokio::net::TcpListener;

pub(super) fn mount<W: Wire + 'static>(
    core: Arc<Core<W>>,
    store: Arc<codehull_repo::Store>,
    runtime: &Runtime,
) {
    if runtime.ssh.key.is_empty() {
        return;
    }
    let held = match codehull_ssh::key(Path::new(&runtime.ssh.key)) {
        Ok(held) => held,
        Err(err) => halt("ssh", &err),
    };
    let (host, port) = (runtime.ssh.host.clone(), runtime.ssh.port);
    tokio::spawn(async move {
        if let Err(err) = listen(core, store, held, (&host, port)).await {
            eprintln!("{}", serde_json::json!({ "role": "ssh", "halt": err }));
        }
    });
}

async fn listen<W: Wire + 'static>(
    core: Arc<Core<W>>,
    store: Arc<codehull_repo::Store>,
    held: codehull_ssh::PrivateKey,
    at: (&str, u16),
) -> Result<(), String> {
    let socket = TcpListener::bind(at)
        .await
        .map_err(|error| format!("ssh listen: {error}"))?;
    let live = socket
        .local_addr()
        .map_err(|error| format!("ssh listen: {error}"))?;
    eprintln!(
        "{}",
        serde_json::json!({ "role": "ssh", "endpoint": format!("ssh://{live}") })
    );
    shell(core, store, held, socket).await
}

pub async fn shell<W: Wire + 'static>(
    core: Arc<Core<W>>,
    store: Arc<codehull_repo::Store>,
    held: codehull_ssh::PrivateKey,
    socket: TcpListener,
) -> Result<(), String> {
    let hall = Arc::new(crate::ground::port::Port::new(core, store));
    codehull_ssh::serve(hall, held, socket).await
}
