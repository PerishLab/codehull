use keel::adapt::Error;
use keel::{Core, Wire};
use keel_gate::Gate;
use keel_relay::Relay;
use std::sync::Arc;

mod grant;

pub(crate) const SVC: &str = "codehull:svc";
pub(crate) const ORG: &str = "codehull:org";

pub(crate) struct Berth<'a, W: Wire>(pub(crate) &'a Arc<Core<W>>);

impl<W: Wire + 'static> Berth<'_, W> {
    pub(crate) async fn seed(&self) -> Result<Gate<W>, Error> {
        let gate = self.gate().await?;
        gate.seed().await?;
        gate.sow(&grant::rows(&SEEDS)).await?;
        self.relay().await?;
        Ok(gate)
    }

    pub(crate) async fn rig(&self) -> Result<Gate<W>, Error> {
        let gate = self.gate().await?;
        if !gate.ready().await? {
            return Err(Error::Adapt("missing keel-gate bootstrap grants".into()));
        }
        if !gate.sown(&grant::rows(&SEEDS)).await? {
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

struct Seed {
    who: &'static str,
    verb: &'static str,
    unit: &'static str,
    scope: &'static str,
}

const SEEDS: [Seed; 17] = [
    Seed {
        who: "anon",
        verb: "see",
        unit: "Actor",
        scope: "all",
    },
    Seed {
        who: "all",
        verb: "put",
        unit: "Actor",
        scope: r#"pred iss = "codehull:org""#,
    },
    Seed {
        who: "anon",
        verb: "see",
        unit: "Repo",
        scope: r#"pred visibility = "public""#,
    },
    Seed {
        who: "all",
        verb: "put",
        unit: "Repo",
        scope: r#"pred owner = "@me""#,
    },
    Seed {
        who: "all",
        verb: "put",
        unit: "Issue",
        scope: r#"pred author = "@me""#,
    },
    Seed {
        who: "all",
        verb: "set",
        unit: "Issue",
        scope: r#"pred author = "@me""#,
    },
    Seed {
        who: "all",
        verb: "see",
        unit: "Issue",
        scope: r#"pred author = "@me""#,
    },
    Seed {
        who: "all",
        verb: "put",
        unit: "Comment",
        scope: r#"pred author = "@me""#,
    },
    Seed {
        who: "all",
        verb: "see",
        unit: "Comment",
        scope: r#"pred author = "@me""#,
    },
    Seed {
        who: "all",
        verb: "put",
        unit: "Reaction",
        scope: r#"pred actor = "@me""#,
    },
    Seed {
        who: "all",
        verb: "see",
        unit: "Reaction",
        scope: r#"pred actor = "@me""#,
    },
    Seed {
        who: "all",
        verb: "put",
        unit: "Asset",
        scope: r#"pred owner = "@me""#,
    },
    Seed {
        who: "all",
        verb: "see",
        unit: "Asset",
        scope: r#"pred owner = "@me""#,
    },
    Seed {
        who: "all",
        verb: "see",
        unit: "Pull",
        scope: r#"pred author = "@me""#,
    },
    Seed {
        who: "all",
        verb: "set",
        unit: "Pull",
        scope: r#"pred author = "@me" and weld = """#,
    },
    Seed {
        who: "all",
        verb: "put",
        unit: "Review",
        scope: r#"pred reviewer = "@me""#,
    },
    Seed {
        who: "all",
        verb: "see",
        unit: "Review",
        scope: r#"pred reviewer = "@me""#,
    },
];
