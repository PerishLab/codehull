#![allow(dead_code)]

use super::issuer::Issuer;
use serde_json::{Value, json};
use ureq::http::Response;
use ureq::{Agent, Body, RequestBuilder};

pub type Head = Vec<(String, String)>;

pub struct Reply {
    pub status: u16,
    pub body: Value,
    pub bytes: Vec<u8>,
    head: Head,
}

impl Reply {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.head
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    pub fn id(&self) -> i64 {
        self.body["id"].as_i64().expect("row id")
    }

    pub fn ok(&self) -> bool {
        (200..300).contains(&self.status)
    }
}

pub struct Seat {
    pub id: i64,
    pub head: Head,
}

impl Seat {
    pub fn token(&self) -> &str {
        self.head[0].1.trim_start_matches("Bearer ")
    }
}

pub enum Load<'a> {
    Bare,
    Json(&'a str),
    Raw(&'a [u8]),
}

pub type Inside = Box<dyn Fn(&str, &str, &Head, Load<'_>) -> Reply>;

pub struct Wire {
    pub base: String,
    agent: Agent,
    inside: Option<Inside>,
}

impl Wire {
    pub fn new(base: &str) -> Self {
        let agent = Agent::config_builder()
            .http_status_as_error(false)
            .max_redirects(0)
            .build()
            .into();
        Self {
            base: base.to_string(),
            agent,
            inside: None,
        }
    }

    pub fn within(base: &str, inside: Inside) -> Self {
        Self {
            inside: Some(inside),
            ..Self::new(base)
        }
    }

    fn local(&self, verb: &str, url: &str, head: &Head, load: Load<'_>) -> Option<Reply> {
        if !url.starts_with('/') {
            return None;
        }
        self.inside
            .as_ref()
            .map(|inside| inside(verb, url, head, load))
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base)
    }

    pub fn post(&self, path: &str, body: Value, head: &Head) -> Reply {
        let (url, text) = (self.url(path), body.to_string());
        self.local("POST", &url, head, Load::Json(&text))
            .unwrap_or_else(|| reply(dress(self.agent.post(url), head).send(text)))
    }

    pub fn patch(&self, path: &str, body: Value, head: &Head) -> Reply {
        let (url, text) = (self.url(path), body.to_string());
        self.local("PATCH", &url, head, Load::Json(&text))
            .unwrap_or_else(|| reply(dress(self.agent.patch(url), head).send(text)))
    }

    pub fn put(&self, path: &str, head: &Head) -> Reply {
        let url = self.url(path);
        self.local("PUT", &url, head, Load::Bare)
            .unwrap_or_else(|| reply(dress(self.agent.put(url), head).send_empty()))
    }

    pub fn knock(&self, path: &str, head: &Head) -> Reply {
        let url = self.url(path);
        self.local("POST", &url, head, Load::Bare)
            .unwrap_or_else(|| reply(dress(self.agent.post(url), head).send_empty()))
    }

    pub fn get(&self, path: &str, head: &Head) -> Reply {
        self.fetch(&self.url(path), head)
    }

    pub fn fetch(&self, url: &str, head: &Head) -> Reply {
        self.local("GET", url, head, Load::Bare)
            .unwrap_or_else(|| reply(dress(self.agent.get(url), head).call()))
    }

    pub fn cut(&self, path: &str, head: &Head) -> Reply {
        let url = self.url(path);
        self.local("DELETE", &url, head, Load::Bare)
            .unwrap_or_else(|| reply(dress(self.agent.delete(url), head).call()))
    }

    pub fn upload(&self, url: &str, bytes: &[u8]) -> Reply {
        reply(self.agent.put(url).send(bytes))
    }

    pub fn carry(&self, url: &str, head: &Head, bytes: &[u8]) -> Reply {
        if let Some(held) = self.local("POST", url, head, Load::Raw(bytes)) {
            return held;
        }
        let mut ask = self.agent.post(url);
        for (key, value) in head {
            ask = ask.header(key, value);
        }
        reply(ask.send(bytes))
    }

    pub fn want(&self, path: &str, body: Value, head: &Head) -> Value {
        let made = self.post(path, body, head);
        assert_eq!(made.status, 201, "POST {path}: {}", made.body);
        made.body
    }

    pub fn made(&self, path: &str, body: Value, head: &Head) -> i64 {
        self.want(path, body, head)["id"].as_i64().expect("row id")
    }

    pub fn query(&self, q: &str, head: &Head) -> Pack {
        let held = self.post("/query", json!({ "q": q }), head);
        assert!(held.ok(), "query {q}: {}", held.status);
        Pack(held.body)
    }

    pub fn one(&self, unit: &str, fields: Value, head: &Head) -> Reply {
        let deeds = json!({ "deeds": [{ "verb": "put", "unit": unit, "fields": fields }] });
        self.post("/batch", deeds, head)
    }

    pub fn deed(&self, unit: &str, fields: Value, head: &Head) -> i64 {
        let res = self.one(unit, fields, head);
        assert_eq!(res.status, 200, "batch put {unit}: {}", res.body);
        res.body["ids"][0].as_i64().expect("batch id")
    }

    pub fn join(&self, issuer: &Issuer, sub: &str, teams: &[&str]) -> Seat {
        let head = vec![(
            "authorization".to_string(),
            format!("Bearer {}", issuer.mint(sub, teams)),
        )];
        let rows = self
            .query(&format!("from Actor where sub = \"{sub}\""), &head)
            .roots();
        assert_eq!(rows.len(), 1, "anchor row for {sub}");
        Seat {
            id: rows[0]["id"].as_i64().expect("actor id"),
            head,
        }
    }

    pub fn private(&self, head: &Head, name: &str, owner: i64) -> i64 {
        self.repo(head, name, owner, "private")
    }

    pub fn public(&self, head: &Head, name: &str, owner: i64) -> i64 {
        self.repo(head, name, owner, "public")
    }

    fn repo(&self, head: &Head, name: &str, owner: i64, visibility: &str) -> i64 {
        let body = json!({
            "name": name,
            "visibility": visibility,
            "owner": owner,
            "trunk": "main",
            "archived": false,
        });
        self.made("/repo", body, head)
    }

    pub fn peek(&self, repo: i64, head: &Head) -> u16 {
        self.get(&format!("/repo/{repo}"), head).status
    }

    pub fn visible(&self, repo: i64, head: &Head) -> bool {
        self.peek(repo, head) == 200
    }

    pub fn admit(&self, head: &Head, team: i64, repo: i64) {
        let rule = json!({
            "who": format!("team {team}"),
            "verb": "see",
            "unit": "Repo",
            "scope": format!("row {repo}"),
        });
        self.want("/@grant", rule, head);
    }
}

pub struct Pack(pub Value);

impl Pack {
    pub fn roots(&self) -> Vec<Value> {
        let root = self.0["root"].as_str().unwrap_or_default();
        self.0["bags"][root].as_array().cloned().unwrap_or_default()
    }

    pub fn bond(&self, key: &str) -> Vec<Value> {
        self.0["bags"][key]
            .as_array()
            .cloned()
            .unwrap_or_else(|| panic!("missing bond bag {key}"))
    }

    pub fn count(&self) -> i64 {
        self.0["count"].as_i64().expect("count")
    }
}

pub fn num(value: &Value) -> i64 {
    value.as_i64().expect("expected number")
}

fn dress<B>(ask: RequestBuilder<B>, head: &Head) -> RequestBuilder<B> {
    let mut ask = ask.header("content-type", "application/json");
    for (key, value) in head {
        ask = ask.header(key, value);
    }
    ask
}

fn reply(result: Result<Response<Body>, ureq::Error>) -> Reply {
    let mut res = result.expect("request");
    let status = res.status().as_u16();
    let head: Head = res
        .headers()
        .iter()
        .map(|(key, value)| {
            (
                key.as_str().to_string(),
                value.to_str().unwrap_or_default().to_string(),
            )
        })
        .collect();
    let bytes = res
        .body_mut()
        .with_config()
        .limit(64 << 20)
        .read_to_vec()
        .unwrap_or_default();
    shape(status, head, bytes)
}

pub fn shape(status: u16, head: Head, bytes: Vec<u8>) -> Reply {
    let body = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()));
    Reply {
        status,
        body,
        bytes,
        head,
    }
}
