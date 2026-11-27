#![allow(dead_code)]

use keel::Wire;
use keel::adapt::pg::Postgres;
use tokio::runtime::{Builder, Runtime};

pub struct Scratch {
    maintenance: String,
    name: String,
    url: String,
    spin: Runtime,
}

impl Scratch {
    pub fn open() -> Option<Self> {
        let maintenance = store()?;
        Some(Self::at(&maintenance))
    }

    pub fn at(maintenance: &str) -> Self {
        let name = format!("codehull_act_{:032x}", rand::random::<u128>());
        let url = retarget(maintenance, &name);
        let spin = Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("scratch database runtime");
        spin.block_on(change(
            maintenance,
            &format!("CREATE DATABASE {name} TEMPLATE template0"),
        ))
        .expect("create scratch database");
        Self {
            maintenance: maintenance.to_string(),
            name,
            url,
            spin,
        }
    }

    pub fn url(&self) -> &str {
        &self.url
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let sql = format!("DROP DATABASE {} WITH (FORCE)", self.name);
        let result = self.spin.block_on(change(&self.maintenance, &sql));
        if let Err(err) = result {
            if std::thread::panicking() {
                eprintln!("scratch database cleanup failed for {}: {err}", self.name);
            } else {
                panic!("scratch database cleanup failed for {}: {err}", self.name);
            }
        }
    }
}

async fn change(url: &str, sql: &str) -> Result<(), keel::adapt::Error> {
    Postgres::at(url).await?.script(sql).await
}

fn store() -> Option<String> {
    ["CODEHULL_STORE_URL", "API_STORE_URL"]
        .into_iter()
        .filter_map(|key| std::env::var(key).ok())
        .find(|url| !url.is_empty())
}

fn retarget(url: &str, name: &str) -> String {
    if let Some((scheme, rest)) = url.split_once("://") {
        let (head, query) = rest
            .split_once('?')
            .map_or((rest, None), |(h, q)| (h, Some(q)));
        let authority = head.split_once('/').map_or(head, |(a, _)| a);
        let mut out = format!("{scheme}://{authority}/{name}");
        if let Some(query) = query {
            let kept: Vec<&str> = query
                .split('&')
                .filter(|pair| !pair.is_empty() && !pair.starts_with("dbname="))
                .collect();
            if !kept.is_empty() {
                out.push('?');
                out.push_str(&kept.join("&"));
            }
        }
        return out;
    }
    let mut pairs: Vec<_> = url
        .split_whitespace()
        .filter(|pair| !pair.starts_with("dbname="))
        .map(str::to_owned)
        .collect();
    pairs.push(format!("dbname={name}"));
    pairs.join(" ")
}
