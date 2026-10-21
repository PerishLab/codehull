use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use serde_json::Value;
use std::collections::BTreeMap;

pub(crate) struct Warden {
    iss: String,
    aud: String,
    keys: BTreeMap<String, DecodingKey>,
}

pub(crate) struct Bearer {
    pub(crate) iss: String,
    pub(crate) sub: String,
    pub(crate) teams: Vec<String>,
}

impl Warden {
    pub(crate) fn open(iss: &str, aud: &str) -> Result<Self, String> {
        let iss = iss.trim_end_matches('/').to_string();
        let seat = probe(&format!("{iss}/.well-known/openid-configuration"))?;
        let held = seat
            .get("jwks_uri")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("{iss} names no jwks_uri"))?
            .to_string();
        let set: JwkSet = serde_json::from_value(probe(&held)?)
            .map_err(|err| format!("malformed jwks at {held}: {err}"))?;
        let mut keys = BTreeMap::new();
        for jwk in &set.keys {
            let Some(kid) = jwk.common.key_id.clone() else {
                continue;
            };
            let key = DecodingKey::from_jwk(jwk)
                .map_err(|err| format!("unusable key {kid} at {held}: {err}"))?;
            keys.insert(kid, key);
        }
        if keys.is_empty() {
            return Err(format!("{held} publishes no usable key"));
        }
        Ok(Warden {
            iss,
            aud: aud.to_string(),
            keys,
        })
    }

    pub(crate) fn read(&self, token: &str) -> Option<Bearer> {
        let head = decode_header(token).ok()?;
        let key = self.keys.get(&head.kid?)?;
        let mut rule = Validation::new(Algorithm::ES256);
        rule.set_issuer(&[&self.iss]);
        rule.set_audience(&[&self.aud]);
        let held = decode::<Value>(token, key, &rule).ok()?;
        let claims = held.claims;
        if claims.get("kind").and_then(Value::as_str) != Some("access") {
            return None;
        }
        Some(Bearer {
            iss: self.iss.clone(),
            sub: claims.get("sub").and_then(Value::as_str)?.to_string(),
            teams: claims
                .get("teams")
                .and_then(Value::as_array)
                .map(|held| {
                    held.iter()
                        .filter_map(|team| team.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default(),
        })
    }
}

fn probe(url: &str) -> Result<Value, String> {
    let mut held = ureq::get(url)
        .call()
        .map_err(|err| format!("cannot reach {url}: {err}"))?;
    if held.status().as_u16() != 200 {
        return Err(format!("{url} answered {}", held.status().as_u16()));
    }
    held.body_mut()
        .read_json()
        .map_err(|err| format!("malformed answer from {url}: {err}"))
}
