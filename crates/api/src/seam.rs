use crate::warden::Warden;
use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use keel::adapt::Error;
use keel::{Core, Operator, Wire};
use keel_gate::Gate;
use std::sync::Arc;

pub(crate) struct Seam<W: Wire> {
    pub(crate) core: Arc<Core<W>>,
    pub(crate) gate: Gate<W>,
    pub(crate) warden: Arc<Warden>,
}

impl<W: Wire> Clone for Seam<W> {
    fn clone(&self) -> Self {
        Seam {
            core: self.core.clone(),
            gate: self.gate.clone(),
            warden: self.warden.clone(),
        }
    }
}

pub(crate) async fn admit<W: Wire + 'static>(
    State(seam): State<Seam<W>>,
    mut req: Request,
    next: Next,
) -> Response {
    if let Some(token) = bearer(&req)
        && let Some(held) = seam.warden.read(&token)
        && let Ok(key) = anchor(&seam, &held.iss, &held.sub).await
        && crate::crew::Crew(&seam.core)
            .settle(key, &held.teams)
            .await
            .is_ok()
    {
        req.extensions_mut().insert(Operator(key));
    }
    next.run(req).await
}

fn bearer(req: &Request) -> Option<String> {
    req.headers()
        .get("authorization")?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(str::to_string)
}

async fn anchor<W: Wire + 'static>(seam: &Seam<W>, iss: &str, sub: &str) -> Result<i64, Error> {
    let held = seam
        .core
        .query(&format!(
            r#"from Actor where iss = "{iss}" and sub = "{sub}""#
        ))
        .await?;
    match held.rows().first() {
        Some(row) => Ok(row.key()),
        None => seam.gate.birth(&[("iss", iss), ("sub", sub)]).await,
    }
}
