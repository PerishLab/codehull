use crate::rig::{ORG, hail};
use keel::adapt::Error;
use keel::{Core, Ends, Tie, Wire};
use std::collections::BTreeSet;
use std::sync::Arc;

pub(crate) const UPSTREAM: &str = "upstream";

pub(crate) struct Crew<'a, W: Wire>(pub(crate) &'a Arc<Core<W>>);

impl<W: Wire + 'static> Crew<'_, W> {
    pub(crate) async fn settle(&self, actor: i64, told: &[String]) -> Result<(), Error> {
        let org = hail(self.0, ORG, UPSTREAM).await?;
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
