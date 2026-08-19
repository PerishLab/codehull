const FLUSH: &[u8] = b"0000";

pub(super) struct Order {
    pub(super) old: String,
    pub(super) new: String,
    pub(super) name: String,
}

pub(super) fn pkt(text: &str) -> Vec<u8> {
    let size = text.len() + 4;
    format!("{size:04x}{text}").into_bytes()
}

pub(super) fn flush() -> Vec<u8> {
    FLUSH.to_vec()
}

pub(super) fn offer(service: &str, refs: &[(String, String)], caps: &str) -> Vec<u8> {
    let mut body = pkt(&format!("# service={service}\n"));
    body.extend_from_slice(&flush());
    body.extend_from_slice(&plain(refs, caps));
    body
}

pub(super) fn plain(refs: &[(String, String)], caps: &str) -> Vec<u8> {
    let mut body = Vec::new();
    if refs.is_empty() {
        let zero = "0".repeat(40);
        body.extend_from_slice(&pkt(&format!("{zero} capabilities^{{}}\0{caps}\n")));
    }
    for (spot, (name, object)) in refs.iter().enumerate() {
        let line = match spot {
            0 => format!("{object} {name}\0{caps}\n"),
            _ => format!("{object} {name}\n"),
        };
        body.extend_from_slice(&pkt(&line));
    }
    body.extend_from_slice(&flush());
    body
}

pub(super) fn triple(text: &str) -> Option<Order> {
    order(text.as_bytes()).ok()
}

pub(super) fn orders(body: &[u8]) -> Result<Option<(Vec<Order>, usize)>, String> {
    let mut held = Vec::new();
    let mut rest = body;
    let mut at = 0;
    while rest.len() >= 4 {
        let size = span(&rest[..4])?;
        if size == 0 {
            return Ok(Some((held, at + 4)));
        }
        if size < 4 {
            return Err("packet length is out of range".into());
        }
        if size > rest.len() {
            return Ok(None);
        }
        held.push(order(&rest[4..size])?);
        rest = &rest[size..];
        at += size;
    }
    Ok(None)
}

fn span(head: &[u8]) -> Result<usize, String> {
    let text = str::from_utf8(head).map_err(|_| "packet length is not text")?;
    usize::from_str_radix(text, 16).map_err(|_| "packet length is not hexadecimal".into())
}

fn order(payload: &[u8]) -> Result<Order, String> {
    let text = str::from_utf8(payload).map_err(|_| "command is not text")?;
    let head = text.split('\0').next().unwrap_or_default().trim_end();
    let mut parts = head.splitn(3, ' ');
    let (Some(old), Some(new), Some(name)) = (parts.next(), parts.next(), parts.next()) else {
        return Err("command is malformed".into());
    };
    Ok(Order {
        old: old.to_owned(),
        new: new.to_owned(),
        name: name.to_owned(),
    })
}
