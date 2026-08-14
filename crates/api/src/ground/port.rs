use super::Dock;
use super::point::Point;
use codehull_repo::Store;
use codehull_ssh::{Hall, Later};
use keel::{Core, Wire};
use std::path::PathBuf;
use std::sync::Arc;

pub(crate) struct Port<W: Wire> {
    dock: Dock<W>,
}

impl<W: Wire + 'static> Port<W> {
    pub(crate) fn new(core: Arc<Core<W>>, store: Arc<Store>) -> Self {
        Self {
            dock: Dock { core, store },
        }
    }

    async fn owner(&self, print: String) -> Option<i64> {
        let material = material(&print)?;
        let pack = self
            .dock
            .core
            .sudo()
            .query(&format!(r#"from actor:key where print like "{material}""#))
            .await
            .ok()?;
        let row = pack.rows().iter().find(|row| {
            row.text("print")
                .is_some_and(|held| held.contains(&material))
        })?;
        row.int("owner_id").or_else(|| row.int("owner"))
    }

    async fn root(&self, who: i64, path: String) -> Option<PathBuf> {
        let (owner, name) = named(&path)?;
        let face = self.dock.core.of(who);
        let held = face
            .query(&format!(r#"from Actor where sub = "{owner}""#))
            .await
            .ok()?;
        let seat = held.rows().first()?.key();
        let pack = face
            .query(&format!(
                r#"from Repo where name = "{name}" and owner = "{seat}""#
            ))
            .await
            .ok()?;
        let id = u64::try_from(pack.rows().first()?.key()).ok()?;
        let point = Point {
            dock: &self.dock,
            who,
            id,
        };
        point.align().await.ok()?;
        let store = self.dock.store.clone();
        tokio::task::spawn_blocking(move || {
            store
                .repository(id)
                .ok()
                .map(|repo| repo.root().to_path_buf())
        })
        .await
        .ok()?
    }
}

impl<W: Wire + 'static> Hall for Port<W> {
    fn admit(&self, print: String) -> Later<'_, Option<i64>> {
        Box::pin(self.owner(print))
    }

    fn seat(&self, who: i64, path: String) -> Later<'_, Option<PathBuf>> {
        Box::pin(self.root(who, path))
    }
}

fn material(print: &str) -> Option<String> {
    let held = print.split_whitespace().nth(1)?;
    if !held.bytes().all(base64) || held.len() < 32 {
        return None;
    }
    Some(held.to_owned())
}

fn base64(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"+/=".contains(&byte)
}

fn named(path: &str) -> Option<(String, String)> {
    let held = path.trim_end_matches(".git");
    let (owner, name) = held.split_once('/')?;
    if !plain(owner) || !plain(name) {
        return None;
    }
    Some((owner.to_owned(), name.to_owned()))
}

fn plain(text: &str) -> bool {
    !text.is_empty() && !text.contains(['"', '\\', '/'])
}
