use super::weld::Weld;
use super::{Fault, deny};
use axum::Json;
use axum::http::StatusCode;
use keel::Wire;
use serde_json::json;

const HEADS: &str = "refs/heads/";
const SUCCESS: &str = "success";
const APPROVE: &str = "approve";

impl<W: Wire + 'static> Weld<'_, W> {
    pub(super) async fn cleared(&self, id: u64, base: &str, tip: &str) -> Result<(), Fault> {
        let Some(branch) = base.strip_prefix(HEADS) else {
            return Ok(());
        };
        let pack = self
            .dock
            .core
            .of(self.who)
            .query(&format!(
                r#"from Shield where repo = "{id}" and branch = "{branch}""#
            ))
            .await
            .map_err(|_| deny())?;
        let Some(row) = pack.rows().first() else {
            return Ok(());
        };
        for context in self.demands(row.key()).await? {
            self.judged(id, tip, &context).await?;
        }
        self.backed(row.int("approvals").unwrap_or_default()).await
    }

    async fn demands(&self, shield: i64) -> Result<Vec<String>, Fault> {
        let pack = self
            .dock
            .core
            .of(self.who)
            .query(&format!(r#"from Demand where shield = "{shield}""#))
            .await
            .map_err(|_| deny())?;
        Ok(pack
            .rows()
            .iter()
            .filter_map(|row| row.text("context").map(str::to_owned))
            .collect())
    }

    async fn judged(&self, id: u64, tip: &str, context: &str) -> Result<(), Fault> {
        let pack = self
            .dock
            .core
            .of(self.who)
            .query(&format!(
                r#"from Verdict where repo = "{id}" and commit = "{tip}" and context = "{context}""#
            ))
            .await
            .map_err(|_| deny())?;
        let held = pack.rows().first().and_then(|row| row.text("state"));
        match held {
            Some(SUCCESS) => Ok(()),
            Some(state) => Err(barred(format!("{context} reads {state}"))),
            None => Err(barred(format!("{context} carries no verdict"))),
        }
    }

    async fn backed(&self, wanted: i64) -> Result<(), Fault> {
        let (Some(pull), true) = (self.pull, wanted > 0) else {
            return Ok(());
        };
        let pack = self
            .dock
            .core
            .of(self.who)
            .query(&format!(r#"from Review where pull = "{pull}""#))
            .await
            .map_err(|_| deny())?;
        let held = pack
            .rows()
            .iter()
            .filter(|row| row.text("state") == Some(APPROVE))
            .count();
        if i64::try_from(held).unwrap_or_default() < wanted {
            return Err(barred(format!("{held} of {wanted} approvals")));
        }
        Ok(())
    }
}

fn barred(note: String) -> Fault {
    (
        StatusCode::PRECONDITION_FAILED,
        Json(json!({ "error": note })),
    )
}
