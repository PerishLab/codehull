use jsonwebtoken::jwk::Jwk;
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde_json::json;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const LEAD: [u8; 35] = [
    0x30, 0x41, 0x02, 0x01, 0x00, 0x30, 0x13, 0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01,
    0x06, 0x08, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07, 0x04, 0x27, 0x30, 0x25, 0x02, 0x01,
    0x01, 0x04, 0x20,
];

pub struct Issuer {
    pub url: String,
    key: EncodingKey,
    halt: Arc<AtomicBool>,
}

impl Issuer {
    pub fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("issuer port");
        let url = format!("http://{}", listener.local_addr().expect("issuer addr"));
        let (key, jwk) = pair();
        let mut public = serde_json::to_value(&jwk).expect("jwk");
        public["kid"] = json!("act");
        public["alg"] = json!("ES256");
        public["use"] = json!("sig");
        let keys = json!({ "keys": [public] }).to_string();
        let disco = json!({ "issuer": url, "jwks_uri": format!("{url}/.well-known/jwks.json") })
            .to_string();
        let halt = Arc::new(AtomicBool::new(false));
        let stop = halt.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                if stop.load(Ordering::SeqCst) {
                    return;
                }
                if let Ok(stream) = stream {
                    answer(stream, &disco, &keys);
                }
            }
        });
        Self { url, key, halt }
    }

    pub fn mint(&self, sub: &str, teams: &[&str]) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_secs();
        let mut head = Header::new(Algorithm::ES256);
        head.kid = Some("act".to_string());
        let body = json!({
            "iss": self.url,
            "sub": sub,
            "teams": teams,
            "aud": "codehull",
            "kind": "access",
            "scope": "openid",
            "iat": now,
            "exp": now + 3600,
        });
        encode(&head, &body, &self.key).expect("mint")
    }
}

impl Drop for Issuer {
    fn drop(&mut self) {
        self.halt.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.url.trim_start_matches("http://"));
    }
}

fn pair() -> (EncodingKey, Jwk) {
    loop {
        let mut der = LEAD.to_vec();
        der.extend(rand::random::<[u8; 32]>());
        let key = EncodingKey::from_ec_der(&der);
        if let Ok(jwk) = Jwk::from_encoding_key(&key, Algorithm::ES256) {
            return (key, jwk);
        }
    }
}

fn answer(mut stream: TcpStream, disco: &str, keys: &str) {
    let mut ask = String::new();
    let mut reader = BufReader::new(stream.try_clone().expect("issuer stream"));
    if reader.read_line(&mut ask).is_err() {
        return;
    }
    let mut line = String::new();
    while reader.read_line(&mut line).is_ok_and(|read| read > 2) {
        line.clear();
    }
    let path = ask.split_whitespace().nth(1).unwrap_or_default();
    let (status, body) = match path {
        "/.well-known/openid-configuration" => ("200 OK", disco),
        "/.well-known/jwks.json" => ("200 OK", keys),
        _ => ("404 Not Found", "not found"),
    };
    let _ = write!(
        stream,
        "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
}
