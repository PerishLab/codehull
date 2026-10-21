use crate::halt;
use crate::runtime::Runtime;
use keel::{Core, Wire};
use std::path::Path;
use std::sync::Arc;

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
    let hall = Arc::new(crate::ground::port::Port::new(core, store));
    let (host, port) = (runtime.ssh.host.clone(), runtime.ssh.port);
    tokio::spawn(async move {
        if let Err(err) = shell(hall, held, (&host, port)).await {
            eprintln!("{}", serde_json::json!({ "role": "ssh", "halt": err }));
        }
    });
}

async fn shell(
    hall: Arc<dyn codehull_ssh::Hall>,
    held: codehull_ssh::PrivateKey,
    at: (&str, u16),
) -> Result<(), String> {
    let socket = tokio::net::TcpListener::bind(at)
        .await
        .map_err(|error| format!("ssh listen: {error}"))?;
    let live = socket
        .local_addr()
        .map_err(|error| format!("ssh listen: {error}"))?;
    eprintln!(
        "{}",
        serde_json::json!({ "role": "ssh", "endpoint": format!("ssh://{live}") })
    );
    codehull_ssh::serve(hall, held, socket).await
}
