use keel::adapt::Error;
use keel::{Core, Wire};
use keel_gate::Gate;
use keel_relay::Relay;
use std::sync::Arc;

pub(crate) const SVC: &str = "codehull:svc";
pub(crate) const ORG: &str = "codehull:org";

pub(crate) struct Berth<'a, W: Wire>(pub(crate) &'a Arc<Core<W>>);

impl<W: Wire + 'static> Berth<'_, W> {
    pub(crate) async fn seed(&self) -> Result<Gate<W>, Error> {
        let gate = self.gate().await?;
        gate.seed().await?;
        gate.sow(&ROWS).await?;
        self.relay().await?;
        Ok(gate)
    }

    pub(crate) async fn rig(&self) -> Result<Gate<W>, Error> {
        let gate = self.gate().await?;
        if !gate.ready().await? {
            return Err(Error::Adapt("missing keel-gate bootstrap grants".into()));
        }
        if !gate.sown(&ROWS).await? {
            return Err(Error::Adapt("missing codehull bootstrap grants".into()));
        }
        self.relay().await?;
        Ok(gate)
    }

    async fn gate(&self) -> Result<Gate<W>, Error> {
        let svc = hail(self.0, SVC, "gate").await?;
        Gate::rise(self.0.clone(), svc)
    }

    async fn relay(&self) -> Result<(), Error> {
        let mail = hail(self.0, SVC, "relay").await?;
        Relay::rise(self.0.clone(), mail).await?.run();
        Ok(())
    }
}

pub(crate) async fn hail<W: Wire + 'static>(
    core: &Arc<Core<W>>,
    iss: &str,
    sub: &str,
) -> Result<i64, Error> {
    let held = core
        .query(&format!(
            r#"from Actor where iss = "{iss}" and sub = "{sub}""#
        ))
        .await?;
    match held.rows().first() {
        Some(row) => Ok(row.key()),
        None => core.put("Actor", &[("iss", iss), ("sub", sub)]).await,
    }
}

const ROWS: [(&str, &str, &str, &str); 15] = [
    ("anon", "see", "Actor", "all"),
    ("all", "put", "Actor", r#"pred iss = "codehull:org""#),
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
