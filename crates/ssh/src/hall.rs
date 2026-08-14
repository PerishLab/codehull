use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

pub type Later<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait Hall: Send + Sync + 'static {
    fn admit(&self, print: String) -> Later<'_, Option<i64>>;
    fn seat(&self, who: i64, path: String) -> Later<'_, Option<PathBuf>>;
    fn offer(&self, who: i64, path: String) -> Later<'_, Option<Vec<u8>>>;
    fn pen(&self, who: i64, path: String) -> Later<'_, Option<PathBuf>>;
    fn apply(
        &self,
        who: i64,
        path: String,
        orders: Vec<String>,
        pen: Option<PathBuf>,
    ) -> Later<'_, Vec<u8>>;
}
