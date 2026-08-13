use crate::rig::{ORG, hail};
use crate::warden::{Bearer, Warden};
use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use keel::adapt::Error;
use keel::{Core, Ends, Operator, Tie, Wire};
use keel_gate::Gate;
use std::collections::BTreeSet;
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
        && let Some(key) = seat(&seam, &held).await
    {
        req.extensions_mut().insert(Operator(key));
    }
    next.run(req).await
}

async fn seat<W: Wire + 'static>(seam: &Seam<W>, held: &Bearer) -> Option<i64> {
    let key = match anchor(seam, &held.iss, &held.sub).await {
        Ok(key) => key,
        Err(err) => {
            eprintln!("codehull: seam: no anchor for {}: {err}", held.sub);
            return None;
        }
    };
    match Crew(&seam.core).settle(key, &held.teams).await {
        Ok(()) => Some(key),
        Err(err) => {
            eprintln!("codehull: seam: crew unsettled for {}: {err}", held.sub);
            None
        }
    }
}

struct Crew<'a, W: Wire>(&'a Arc<Core<W>>);

impl<W: Wire + 'static> Crew<'_, W> {
    async fn settle(&self, actor: i64, told: &[String]) -> Result<(), Error> {
        let org = hail(self.0, ORG, "upstream").await?;
        let held = self.seats(org).await?;
        let want: BTreeSet<&String> = told.iter().collect();
        for (name, team) in &held {
            let tied = self.0.ties("Team", "members", *team).await?;
            self.weigh(*team, actor, want.contains(name), &tied).await?;
        }
        for name in told {
            if held.iter().all(|(held, _)| held != name) {
                let team = self.raise(org, name).await?;
                self.join(team, actor).await?;
            }
        }
        Ok(())
    }

    async fn weigh(&self, team: i64, actor: i64, want: bool, tied: &[Tie]) -> Result<(), Error> {
        let mine: Vec<&Tie> = tied.iter().filter(|tie| tie.right() == actor).collect();
        if want {
            if mine.is_empty() {
                self.join(team, actor).await?;
            }
            return Ok(());
        }
        for tie in mine {
            self.0.cut("Team", "members", tie.key()).await?;
        }
        Ok(())
    }

    async fn seats(&self, org: i64) -> Result<Vec<(String, i64)>, Error> {
        let pack = self
            .0
            .query(&format!(r#"from Team where org = "{org}""#))
            .await?;
        Ok(pack
            .rows()
            .iter()
            .filter_map(|row| {
                row.cells()
                    .get("name")
                    .map(|name| (name.show().to_string(), row.key()))
            })
            .collect())
    }

    async fn raise(&self, org: i64, name: &str) -> Result<i64, Error> {
        self.0
            .put(
                "Team",
                &[("name", name), ("mode", "read"), ("org", &org.to_string())],
            )
            .await
    }

    async fn join(&self, team: i64, actor: i64) -> Result<(), Error> {
        self.0
            .tie(
                "Team",
                "members",
                Ends {
                    left: team,
                    right: actor,
                },
                &[],
            )
            .await
            .map(|_| ())
    }
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
