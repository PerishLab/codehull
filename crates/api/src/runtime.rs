pub struct Runtime {
    pub fresh: bool,
    pub pg: Option<String>,
    pub port: Option<u16>,
    pub s3: Option<S3>,
}

pub struct S3 {
    pub endpoint: String,
    pub key: String,
    pub secret: String,
}

pub fn load() -> Result<Runtime, String> {
    let port = match read("SIDECAR_PORT") {
        Some(raw) => Some(
            raw.parse()
                .map_err(|err: std::num::ParseIntError| format!("SIDECAR_PORT: {err}"))?,
        ),
        None => None,
    };
    let s3 = read("KEEL_S3").map(|endpoint| S3 {
        endpoint,
        key: read("KEEL_S3_KEY").unwrap_or_else(|| "codehull".into()),
        secret: read("KEEL_S3_SECRET").unwrap_or_else(|| "codehull123".into()),
    });
    Ok(Runtime {
        fresh: std::env::var_os("KEEL_FRESH").is_some(),
        pg: read("KEEL_PG"),
        port,
        s3,
    })
}

fn read(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
}
