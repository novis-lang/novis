//! `POST /api/replay`: one recorded request sent again to the development server that started this
//! service (`rule:testing/two-requests-compare-and-a-replay-asks-first`).
//!
//! **One target, and never the browser's.** The address is the one a development `nvs serve`
//! wrote on its child's standard input once it was bound ([`super::Target`]). The body names a
//! request by its trace id and span id and nothing about where it goes, so this route cannot be
//! turned into a request to another host. A service started by hand has no target, and the answer
//! is a `409` that says so. `GET /api/replay` gives the target, or `null`, so the viewer names it
//! in the dialog and says before it is opened when there is none.
//!
//! **What is sent.** HTTP/1.1 written by hand over a `std` socket, with `Connection: close`, and
//! of the answer only its head is read. The recorded method, path, query and headers, and the
//! body when the program read it. A header the spool redacted is sent only when the body gives its value, and is left out
//! otherwise; a value for any other header is a `400`. `traceparent` and `tracestate` are left out,
//! so the replay gets its own trace id, and so are the headers that belong to one connection. A
//! typed value is in this request's memory alone and is written nowhere, and the server that
//! answers records it redacted again. A body that was cut at the cap or withheld is not sent, and
//! the answer is a `409`.
//!
//! **What comes back.** The status, and the trace id and span id of the server's `traceresponse`
//! when it sent one, so the viewer opens the replay beside its original. The response's body is not
//! read: the server records it, and compare reads it from the index.
//!
//! **What it spends:** one recorded request's input, under the spool's body cap, and the head of
//! the answer, at most [`HEAD_MAX`] bytes, while it is sent. The viewer connection's thread waits at
//! most [`WAIT`] for the answer.

use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use hyper::StatusCode;
use nvs_runtime::spool::REDACTED;
use rusqlite::{Connection, OptionalExtension as _, params};
use serde::{Deserialize, Serialize};

use super::api::{Api, answer, error, hex};

/// The most bytes of the answer's head that are read.
const HEAD_MAX: usize = 64 * 1024;

/// How long a write or a read waits on the server. A handler may run as long as it likes, and this
/// is how long the viewer waits for it.
const WAIT: Duration = Duration::from_secs(120);

/// Headers a replay never sends: those of one connection, the length it writes itself, and the
/// trace context, so the replay gets its own trace id.
const LEFT_OUT: [&str; 11] = [
    "connection",
    "keep-alive",
    "proxy-connection",
    "transfer-encoding",
    "te",
    "trailer",
    "upgrade",
    "content-length",
    "expect",
    "traceparent",
    "tracestate",
];

/// The body of `POST /api/replay`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Asked {
    trace: String,
    span: String,
    /// A value for each redacted header the developer typed, by its name.
    #[serde(default)]
    headers: HashMap<String, String>,
}

/// The answer: the replay's status, and its request when the server named it.
#[derive(Serialize)]
struct Replayed {
    status: u16,
    trace: Option<String>,
    span: Option<String>,
}

/// A recorded request, as it is sent again.
struct Recorded {
    method: String,
    path: String,
    query: String,
    headers: Vec<(String, String)>,
    body: Option<Vec<u8>>,
}

/// Why a recorded request cannot be sent.
enum Unsendable {
    Missing,
    NoInput,
    Cut,
    Withheld,
}

/// `GET /api/replay`: `{"target": "<address>"}`, or a `null` target when this service was started
/// by hand. The dialog names it before it is opened, and says when there is none.
pub(crate) fn target(target: Option<SocketAddr>) -> (StatusCode, Vec<u8>) {
    let body = serde_json::json!({ "target": target.map(|target| target.to_string()) });
    (StatusCode::OK, body.to_string().into_bytes())
}

/// `POST /api/replay` with `body`, sent to `target`: the status and the JSON body.
pub(crate) fn replay(api: &Api, target: Option<SocketAddr>, body: &[u8]) -> (StatusCode, Vec<u8>) {
    let Some(target) = target else {
        return error(
            StatusCode::CONFLICT,
            "This service was started by hand, so it has no server to send a replay to. \
             A development `nvs serve` starts a service that can replay.",
        );
    };
    let asked: Asked = match serde_json::from_slice(body) {
        Ok(asked) => asked,
        Err(err) => {
            return error(
                StatusCode::BAD_REQUEST,
                &format!("The body is not valid: {err}"),
            );
        }
    };
    if let Err(message) = hex("trace", &asked.trace, 32).and_then(|_| hex("span", &asked.span, 16))
    {
        return error(StatusCode::BAD_REQUEST, &message);
    }
    let recorded = match api.read(|db| recorded(db, &asked.trace, &asked.span)) {
        Ok(Ok(recorded)) => recorded,
        Ok(Err(unsendable)) => {
            let (status, message) = match unsendable {
                Unsendable::Missing => (
                    StatusCode::NOT_FOUND,
                    "No request in the index has this trace id and span id.",
                ),
                Unsendable::NoInput => (
                    StatusCode::CONFLICT,
                    "This request was recorded without its input, so it cannot be sent again. \
                     A server records the input in development only.",
                ),
                Unsendable::Cut => (
                    StatusCode::CONFLICT,
                    "The recorded body is not complete, so the request cannot be sent again.",
                ),
                Unsendable::Withheld => (
                    StatusCode::CONFLICT,
                    "The body was not recorded because it has a secret, so the request cannot \
                     be sent again.",
                ),
            };
            return error(status, message);
        }
        Err(err) => return answer::<()>(Err(err)),
    };
    let head = match head(&recorded, &asked.headers, target) {
        Ok(head) => head,
        Err(message) => return error(StatusCode::BAD_REQUEST, &message),
    };
    match send(target, &head, recorded.body.as_deref().unwrap_or_default()) {
        Ok(replayed) => answer(Ok(replayed)),
        Err(err) => error(
            StatusCode::BAD_GATEWAY,
            &format!("The server at `{target}` did not answer: {err}"),
        ),
    }
}

/// The recorded request with `trace` and `span`, its body put together from its parts.
fn recorded(
    db: &mut Connection,
    trace: &str,
    span: &str,
) -> rusqlite::Result<Result<Recorded, Unsendable>> {
    let tx = db.transaction()?;
    let found = tx
        .query_row(
            "SELECT r.method, r.path, r.writer, r.seq, i.request, i.query, i.headers, i.body, \
             i.body_read, i.body_cut, i.body_chunks, i.body_withheld \
             FROM requests r LEFT JOIN inputs i ON i.request = r.id \
             WHERE r.trace = ?1 AND r.span = ?2 ORDER BY r.id DESC LIMIT 1",
            params![trace, span],
            |row| {
                Ok((
                    (
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                    ),
                    row.get::<_, Option<i64>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<Vec<u8>>>(7)?,
                    row.get::<_, Option<bool>>(8)?,
                    row.get::<_, Option<bool>>(9)?,
                    row.get::<_, Option<i64>>(10)?,
                    row.get::<_, Option<bool>>(11)?,
                ))
            },
        )
        .optional()?;
    let Some((
        (method, path, writer, seq),
        input,
        query,
        headers,
        inline,
        read,
        cut,
        chunks,
        withheld,
    )) = found
    else {
        return Ok(Err(Unsendable::Missing));
    };
    if input.is_none() {
        return Ok(Err(Unsendable::NoInput));
    }
    if withheld == Some(true) {
        return Ok(Err(Unsendable::Withheld));
    }
    if cut == Some(true) {
        return Ok(Err(Unsendable::Cut));
    }
    let body = match (read, chunks) {
        (Some(true), Some(chunks)) => {
            // The parts this writer wrote after the previous request of the trace and before this
            // one's own line, as `GET /api/request` splits a shared trace id.
            let mut statement = tx.prepare_cached(
                "SELECT bytes FROM body_parts WHERE trace = ?1 AND of = 'request' \
                 AND writer = ?2 AND seq < ?3 AND seq > coalesce((SELECT max(seq) FROM requests \
                 WHERE trace = ?1 AND writer = ?2 AND seq < ?3), -1) ORDER BY offset",
            )?;
            let parts = statement
                .query_map(params![trace, writer, seq], |row| row.get::<_, Vec<u8>>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            if i64::try_from(parts.len()).ok() != Some(chunks) {
                return Ok(Err(Unsendable::Cut));
            }
            Some(parts.concat())
        }
        (Some(true), None) => Some(inline.unwrap_or_default()),
        _ => None,
    };
    let headers = headers
        .and_then(|text| serde_json::from_str::<Option<Vec<(String, String)>>>(&text).ok())
        .flatten()
        .unwrap_or_default();
    Ok(Ok(Recorded {
        method,
        path,
        query: query.unwrap_or_default(),
        headers,
        body,
    }))
}

/// The request line and headers to send, ending in the blank line. `typed` is a value for each
/// redacted header the developer gave. The error is the message for a `400`.
fn head(
    recorded: &Recorded,
    typed: &HashMap<String, String>,
    target: SocketAddr,
) -> Result<String, String> {
    let token = |text: &str| !text.is_empty() && text.bytes().all(|b| b.is_ascii_graphic());
    if !token(&recorded.method) || !token(&recorded.path) || !recorded.path.starts_with('/') {
        return Err("The recorded method or path cannot be sent.".to_owned());
    }
    let typed: HashMap<String, &str> = typed
        .iter()
        .map(|(name, value)| (name.to_ascii_lowercase(), value.as_str()))
        .collect();
    for (name, value) in &typed {
        let redacted = recorded
            .headers
            .iter()
            .any(|(have, was)| have.eq_ignore_ascii_case(name) && was == REDACTED);
        if !redacted {
            return Err(format!(
                "`{name}` is not a redacted header of this request, so it takes no value."
            ));
        }
        if value.bytes().any(|b| matches!(b, b'\r' | b'\n' | b'\0')) {
            return Err(format!("The value of `{name}` has a line break."));
        }
    }
    let mut head = format!("{} {}", recorded.method, recorded.path);
    if !recorded.query.is_empty() {
        if !recorded.query.bytes().all(|b| b.is_ascii_graphic()) {
            return Err("The recorded query cannot be sent.".to_owned());
        }
        head.push('?');
        head.push_str(&recorded.query);
    }
    head.push_str(" HTTP/1.1\r\n");
    let mut host = false;
    for (name, value) in &recorded.headers {
        let lower = name.to_ascii_lowercase();
        if LEFT_OUT.contains(&lower.as_str()) {
            continue;
        }
        let value = if value == REDACTED {
            match typed.get(&lower) {
                Some(value) => *value,
                None => continue,
            }
        } else {
            value.as_str()
        };
        if !token(name) || value.bytes().any(|b| matches!(b, b'\r' | b'\n' | b'\0')) {
            continue;
        }
        host |= lower == "host";
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    if !host {
        head.push_str(&format!("Host: {target}\r\n"));
    }
    head.push_str("Connection: close\r\n");
    Ok(head)
}

/// Sends `head`, its length and `body` to `target`, and reads the answer's head.
fn send(target: SocketAddr, head: &str, body: &[u8]) -> io::Result<Replayed> {
    let mut stream = TcpStream::connect_timeout(&target, WAIT)?;
    stream.set_read_timeout(Some(WAIT))?;
    stream.set_write_timeout(Some(WAIT))?;
    let mut request = Vec::with_capacity(head.len() + 32 + body.len());
    request.extend_from_slice(head.as_bytes());
    request.extend_from_slice(format!("Content-Length: {}\r\n\r\n", body.len()).as_bytes());
    request.extend_from_slice(body);
    stream.write_all(&request)?;
    drop(request);
    let mut answer = Vec::new();
    let mut chunk = [0u8; 8 * 1024];
    let end = loop {
        if let Some(end) = answer.windows(4).position(|w| w == b"\r\n\r\n") {
            break end;
        }
        if answer.len() > HEAD_MAX {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "the head of its answer is too long",
            ));
        }
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        answer.extend_from_slice(&chunk[..read]);
    };
    let head = String::from_utf8_lossy(&answer[..end]);
    let mut lines = head.split("\r\n");
    let status = lines
        .next()
        .and_then(|line| line.split(' ').nth(1)?.parse::<u16>().ok())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "its answer is not HTTP"))?;
    let named = lines.find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if !name.trim().eq_ignore_ascii_case("traceresponse") {
            return None;
        }
        let mut parts = value.trim().split('-');
        let (trace, span) = (parts.nth(1)?, parts.next()?);
        Some((hex("trace", trace, 32).ok()?, hex("span", span, 16).ok()?))
    });
    Ok(Replayed {
        status,
        trace: named.as_ref().map(|(trace, _)| trace.clone()),
        span: named.map(|(_, span)| span),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recorded(headers: &[(&str, &str)]) -> Recorded {
        Recorded {
            method: "POST".to_owned(),
            path: "/orders".to_owned(),
            query: "page=2".to_owned(),
            headers: headers
                .iter()
                .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
                .collect(),
            body: None,
        }
    }

    /// A redacted header is sent only with a typed value, the trace context and the connection's
    /// headers are left out, and a value for a header that was not redacted is refused.
    #[test]
    fn a_replay_sends_a_redacted_header_only_when_its_value_is_typed() {
        let target: SocketAddr = "127.0.0.1:8000".parse().expect("an address");
        let request = recorded(&[
            ("Host", "example.com"),
            ("Authorization", REDACTED),
            ("Cookie", REDACTED),
            (
                "traceparent",
                "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01",
            ),
            ("Connection", "keep-alive"),
            ("Accept", "*/*"),
        ]);
        let typed = HashMap::from([("authorization".to_owned(), "Bearer typed".to_owned())]);
        let head = head(&request, &typed, target).expect("the head is built");
        assert_eq!(
            head,
            "POST /orders?page=2 HTTP/1.1\r\nHost: example.com\r\nAuthorization: Bearer typed\r\n\
             Accept: */*\r\nConnection: close\r\n"
        );
        let forged = HashMap::from([("accept".to_owned(), "text/html".to_owned())]);
        assert!(super::head(&request, &forged, target).is_err());
        let split = HashMap::from([("cookie".to_owned(), "a=1\r\nX-Evil: 1".to_owned())]);
        assert!(super::head(&request, &split, target).is_err());
        let bare = super::head(&recorded(&[]), &HashMap::new(), target).expect("the head is built");
        assert!(bare.contains("Host: 127.0.0.1:8000\r\n"), "{bare}");
    }
}
