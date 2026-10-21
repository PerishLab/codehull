use super::shell::mount;
use super::{Refusal, custody, hold, router};
use crate::artifact::Artifact;
use crate::halt;
use crate::model::shape;
use crate::rig::Berth;
use crate::runtime::{Hold, Runtime};
use keel::{Core, Wire, bind};
use std::sync::Arc;

pub struct Estate<W: Wire>(pub W);

pub struct Raised<W: Wire> {
    pub core: Arc<Core<W>>,
    pub sudo: Option<String>,
}

impl<W: Wire + 'static> Estate<W> {
    pub async fn raise(self, runtime: &Runtime, born: bool) -> Result<Raised<W>, Refusal> {
        let (core, sudo) = self
            .start(born)
            .await
            .map_err(|err| ("bind", err.to_string()))?;
        let core = match runtime.cache.kind {
            Hold::Memory => core,
            Hold::None => core.bare(),
        };
        let core = hold(core).map_err(|err| ("bind", err.to_string()))?;
        Ok(Raised { core, sudo })
    }

    async fn start(self, born: bool) -> Result<(Core<W>, Option<String>), keel::adapt::Error> {
        if !born {
            return Ok((bind(shape(), self.0).await?, None));
        }
        let mut estate = keel::bootstrap(shape(), self.0)?;
        let token = estate.mint().await?;
        Ok((estate.seal(&token).await?, Some(token)))
    }

    pub(super) async fn provision(self, sudo: &Artifact) {
        let mut estate = match keel::bootstrap(shape(), self.0) {
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

    pub(super) async fn serve(
        self,
        runtime: &Runtime,
        repo: Option<codehull_repo::Store>,
        born: bool,
    ) {
        let raised = match self.raise(runtime, born).await {
            Ok(raised) => raised,
            Err((seat, note)) => halt(seat, &note),
        };
        if let Some(token) = &raised.sudo {
            eprintln!("codehull: sudo token {token}");
        }
        let repo = repo.map(Arc::new);
        let app = match router(raised.core.clone(), runtime, repo.clone(), born).await {
            Ok(app) => app,
            Err((seat, note)) => halt(seat, &note),
        };
        if let Some(store) = repo {
            mount(raised.core, store, runtime);
        }
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
        if let Err(err) = axum::serve(bound, app).await {
            halt("serve", &err.to_string());
        }
    }
}
