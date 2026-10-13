use crate::halt;
use keel::adapt::db::Sqlite;

pub(super) async fn memory() -> Sqlite {
    match Sqlite::memory().await {
        Ok(store) => store,
        Err(err) => halt("store", &err.to_string()),
    }
}
