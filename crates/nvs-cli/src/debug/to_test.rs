//! `nvs debug to-test <trace-id>`: a recorded request printed as a test that asserts its recorded
//! response (`rule:testing/a-recorded-request-becomes-a-test`). `GET /api/to-test` is the viewer's
//! button, and it calls the same [`read`] and [`print()`].
//!
//! **What the test sends.** The recorded method, path, query and headers, and the body when the
//! program read it, through `Core\Test::request` (`rule:testing/in-process-request`). The headers
//! a replay leaves out ([`super::replay::LEFT_OUT`]) are left out here too. A header the spool
//! redacted is written as `'redacted'` under a comment that asks for a test credential, so the test
//! compiles as printed. A request body the spool cut or withheld is refused with the reason, because
//! part of a body is another request.
//!
//! **What the test asserts.** The status, `Content-Type`, `Location` on a redirect, and the body. A
//! JSON body is compared with `assertEquals` against `Core\Json::decode` of the recorded text, so
//! spacing and escapes do not fail it. Key order does, because two arrays are equal only with their
//! keys in the same order (`rule:expressions/equality-semantics`), and a program writes its keys in
//! one order. Any other body is compared as text, or as bytes through `Core\Encoding::fromHex`
//! when it is not text a literal can hold. A response body the spool cut or withheld, or a request
//! recorded with no response, gives a test that asserts the status and the headers only.
//!
//! **Formatted once.** The text goes through [`nvs_fmt::format`], so it is what `nvs fmt` would
//! write.
//!
//! **What it spends:** one request's record, under the spool's body cap, and the text of one test,
//! for as long as the command or the viewer's request runs. Development only.

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::PathBuf;
use std::process::ExitCode;

use hyper::StatusCode;
use nvs_runtime::spool::REDACTED;
use rusqlite::{Connection, OpenFlags, params};

use super::api::{Api, Key, answer, error, hex};
use super::replay::LEFT_OUT;

/// Print a test that sends a recorded request again and checks the response.
///
/// The test sends the request with `Core\Test::request`. It checks the status, the `Content-Type`
/// header, the `Location` header of a redirect, and the body. A JSON body is compared as data, so
/// spaces in it do not matter.
///
/// A header that was removed when the request was recorded, such as `Authorization`, has the value
/// `'redacted'`. Replace it with a test credential. If the recorded request body is incomplete, no
/// test is printed. If the recorded response body is incomplete, the test does not check the body.
///
/// Save the test in your project and run it with `nvs test .`.
#[derive(clap::Args)]
pub(crate) struct Args {
    /// The trace id of the request, as the debug viewer shows it.
    #[arg(value_name = "TRACE_ID")]
    trace: String,
    /// The span id of the request. You need it only when several requests have the same trace id.
    #[arg(long, value_name = "SPAN_ID")]
    span: Option<String>,
    /// Write the test to this new file. If the file exists, nothing is written.
    #[arg(long, value_name = "FILE")]
    write: Option<PathBuf>,
}

/// A recorded request with its input and what it answered.
pub(super) struct Recorded {
    trace: String,
    method: String,
    path: String,
    query: String,
    headers: Vec<(String, String)>,
    /// The request body, when the program read one.
    body: Option<Vec<u8>>,
    status: i64,
    response_headers: Vec<(String, String)>,
    /// The response body, or why the test does not check it.
    response_body: Result<Vec<u8>, Unchecked>,
}

/// Why the test does not check the response body.
#[derive(Clone, Copy)]
enum Unchecked {
    /// The request was recorded with no response.
    NotRecorded,
    /// The spool cut the body at `[limits] request_body`.
    Cut,
    /// The spool did not record the body, because it has a secret.
    Withheld,
}

/// Why no test is printed.
pub(super) enum Refused {
    Missing,
    Several,
    NoInput,
    Cut,
    Withheld,
    Method(String),
}

impl Refused {
    /// The sentence the command prints and the viewer shows.
    fn message(&self) -> String {
        match self {
            Self::Missing => "No request in the index has this trace id and span id.".to_owned(),
            Self::Several => {
                "Several requests have this trace id. Add the span id to choose one.".to_owned()
            }
            Self::NoInput => "This request was recorded without its input, so no test can send it \
                              again. A server records the input in development only."
                .to_owned(),
            Self::Cut => "The recorded request body is not complete. A test that sends part of \
                          the body tests a different request, so no test is printed."
                .to_owned(),
            Self::Withheld => "The request body was not recorded because it has a secret, so no \
                               test is printed."
                .to_owned(),
            Self::Method(method) => {
                format!("`Core\\Test::request` cannot send a `{method}` request.")
            }
        }
    }
}

impl Args {
    /// Runs `nvs debug to-test`: prints the test, or writes it with `--write`.
    pub(crate) fn run(self) -> ExitCode {
        let trace = match hex("trace", &self.trace, 32) {
            Ok(trace) => trace,
            Err(message) => return fail(&message),
        };
        let span = match self.span.as_deref().map(|span| hex("span", span, 16)) {
            Some(Err(message)) => return fail(&message),
            Some(Ok(span)) => Some(span),
            None => None,
        };
        let Some(folder) = nvs_config::data::current() else {
            return fail("`nvs debug to-test` needs a data folder, and Novis cannot use one");
        };
        let path = folder.debug_index();
        if !path.is_file() {
            return fail(&format!(
                "there is no debug index at `{}`. `nvs debug` creates it from recorded requests.",
                path.display()
            ));
        }
        let read = Connection::open_with_flags(
            &path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .and_then(|mut db| {
            db.busy_timeout(std::time::Duration::from_secs(1))?;
            read(&mut db, &trace, span.as_deref())
        });
        let recorded = match read {
            Ok(Ok(recorded)) => recorded,
            Ok(Err(refused)) => return fail(&refused.message()),
            Err(err) => return fail(&format!("cannot read `{}`: {err}", path.display())),
        };
        let text = match print(&recorded) {
            Ok(text) => text,
            Err(refused) => return fail(&refused.message()),
        };
        let Some(file) = self.write else {
            print!("{text}");
            return ExitCode::SUCCESS;
        };
        let written = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&file)
            .and_then(|mut out| out.write_all(text.as_bytes()));
        match written {
            Ok(()) => {
                eprintln!("wrote the test to `{}`", file.display());
                ExitCode::SUCCESS
            }
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => fail(&format!(
                "`{}` already exists. `--write` never replaces a file.",
                file.display()
            )),
            Err(err) => fail(&format!("cannot write `{}`: {err}", file.display())),
        }
    }
}

/// Prints `message` as an error, and the exit code that goes with it.
fn fail(message: &str) -> ExitCode {
    eprintln!("error: {message}");
    ExitCode::FAILURE
}

/// `GET /api/to-test?trace=&span=`: `{"text": "<the test>"}`, or the reason there is none.
pub(crate) fn api(api: &Api, query: Option<&str>) -> (StatusCode, Vec<u8>) {
    let key = match Key::parse(query.unwrap_or_default()) {
        Ok(key) => key,
        Err(message) => return error(StatusCode::BAD_REQUEST, &message),
    };
    let recorded = match api.read(|db| read(db, &key.trace, key.span.as_deref())) {
        Ok(Ok(recorded)) => recorded,
        Ok(Err(refused)) => {
            let status = match refused {
                Refused::Missing => StatusCode::NOT_FOUND,
                Refused::Several => StatusCode::BAD_REQUEST,
                _ => StatusCode::CONFLICT,
            };
            return error(status, &refused.message());
        }
        Err(err) => return answer::<()>(Err(err)),
    };
    match print(&recorded) {
        Ok(text) => answer(Ok(serde_json::json!({ "text": text }))),
        Err(refused) => error(StatusCode::CONFLICT, &refused.message()),
    }
}

/// The request `trace` and `span` name, with its input and its response.
pub(super) fn read(
    db: &mut Connection,
    trace: &str,
    span: Option<&str>,
) -> rusqlite::Result<Result<Recorded, Refused>> {
    let tx = db.transaction()?;
    let mut found = tx
        .prepare_cached(
            "SELECT r.method, r.path, r.status, r.writer, r.seq, i.request, i.query, i.headers, \
             i.body, i.body_read, i.body_cut, i.body_chunks, i.body_withheld, \
             i.response_headers, i.response_body, i.response_read, i.response_cut, \
             i.response_chunks, i.response_withheld \
             FROM requests r LEFT JOIN inputs i ON i.request = r.id \
             WHERE r.trace = ?1 AND (?2 IS NULL OR r.span = ?2) ORDER BY r.id DESC LIMIT 2",
        )?
        .query_map(params![trace, span], |row| {
            Ok(Row {
                method: row.get(0)?,
                path: row.get(1)?,
                status: row.get(2)?,
                writer: row.get(3)?,
                seq: row.get(4)?,
                input: row.get(5)?,
                query: row.get(6)?,
                headers: row.get(7)?,
                body: row.get(8)?,
                body_flags: (row.get(9)?, row.get(10)?, row.get(11)?, row.get(12)?),
                response_headers: row.get(13)?,
                response_body: row.get(14)?,
                response_flags: (row.get(15)?, row.get(16)?, row.get(17)?, row.get(18)?),
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if found.len() > 1 {
        return Ok(Err(Refused::Several));
    }
    let Some(row) = found.pop() else {
        return Ok(Err(Refused::Missing));
    };
    if row.input.is_none() || row.headers.is_none() {
        return Ok(Err(Refused::NoInput));
    }
    let mut parts = tx.prepare_cached(
        "SELECT bytes FROM body_parts WHERE trace = ?1 AND of = ?2 AND writer = ?3 AND seq < ?4 \
         AND seq > coalesce((SELECT max(seq) FROM requests WHERE trace = ?1 AND writer = ?3 \
         AND seq < ?4), -1) ORDER BY offset",
    )?;
    // The body of `of`, or why it is not whole. `None` is a body the program did not read.
    let mut whole = |of: &str, inline: Option<Vec<u8>>, flags: Flags| {
        let (read, cut, chunks, withheld) = flags;
        if read != Some(true) {
            return Ok::<_, rusqlite::Error>(Ok(None));
        }
        if withheld == Some(true) {
            return Ok(Err(Unchecked::Withheld));
        }
        if cut == Some(true) {
            return Ok(Err(Unchecked::Cut));
        }
        let Some(chunks) = chunks else {
            return Ok(Ok(Some(inline.unwrap_or_default())));
        };
        let found = parts
            .query_map(params![trace, of, row.writer, row.seq], |row| {
                row.get::<_, Vec<u8>>(0)
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        if i64::try_from(found.len()).ok() != Some(chunks) {
            return Ok(Err(Unchecked::Cut));
        }
        Ok(Ok(Some(found.concat())))
    };
    let body = match whole("request", row.body, row.body_flags)? {
        Ok(body) => body,
        Err(Unchecked::Withheld) => return Ok(Err(Refused::Withheld)),
        Err(_) => return Ok(Err(Refused::Cut)),
    };
    let response_body = match row.response_flags.0 {
        None => Err(Unchecked::NotRecorded),
        Some(_) => {
            whole("response", row.response_body, row.response_flags)?.map(Option::unwrap_or_default)
        }
    };
    Ok(Ok(Recorded {
        trace: trace.to_owned(),
        method: row.method,
        path: row.path,
        query: row.query.unwrap_or_default(),
        headers: pairs(row.headers.as_deref()),
        body,
        status: row.status,
        response_headers: pairs(row.response_headers.as_deref()),
        response_body,
    }))
}

/// A body's `read`, `cut`, `chunks` and `withheld` columns.
type Flags = (Option<bool>, Option<bool>, Option<i64>, Option<bool>);

/// One row of [`read`]'s query.
struct Row {
    method: String,
    path: String,
    status: i64,
    writer: String,
    seq: i64,
    input: Option<i64>,
    query: Option<String>,
    headers: Option<String>,
    body: Option<Vec<u8>>,
    body_flags: Flags,
    response_headers: Option<String>,
    response_body: Option<Vec<u8>>,
    response_flags: Flags,
}

/// The `[name, value]` pairs of a recorded headers column.
fn pairs(text: Option<&str>) -> Vec<(String, String)> {
    text.and_then(|text| serde_json::from_str::<Option<Vec<(String, String)>>>(text).ok())
        .flatten()
        .unwrap_or_default()
}

/// The test for `recorded`, formatted.
pub(super) fn print(recorded: &Recorded) -> Result<String, Refused> {
    let method = METHODS
        .iter()
        .find(|case| case.eq_ignore_ascii_case(&recorded.method))
        .ok_or_else(|| Refused::Method(recorded.method.clone()))?;
    let mut target = recorded.path.clone();
    if !recorded.query.is_empty() {
        target.push('?');
        target.push_str(&recorded.query);
    }
    let mut out = String::from("<?nvs\n");
    let _ = writeln!(
        out,
        "// This test sends the request with the trace id {} again.\n\
         // It checks that the program gives the same response.\n\
         use Core\\Http\\Method;\nuse Core\\Test;\n",
        recorded.trace
    );
    let _ = writeln!(
        out,
        "class {}\n{{\n#[Test]\npublic function givesTheRecordedResponse(): void\n{{",
        class_name(method, &recorded.path, &recorded.trace)
    );
    let headers = sent_headers(&recorded.headers);
    let mut options = Vec::new();
    if !headers.is_empty() {
        out.push_str("array<string> $headers = [\n");
        for (name, value) in &headers {
            let Some(key) = literal(name) else { continue };
            if value == REDACTED {
                out.push_str(
                    "// This value was removed when the request was recorded. \
                     Put a test credential here.\n",
                );
                let _ = writeln!(out, "{key} => 'redacted',");
            } else if let Some(value) = literal(value) {
                let _ = writeln!(out, "{key} => {value},");
            }
        }
        out.push_str("];\n");
        options.push("headers: $headers".to_owned());
    }
    if let Some(body) = recorded.body.as_deref().filter(|body| !body.is_empty()) {
        options.push(format!("body: {}", value(body)));
    }
    let options = if options.is_empty() {
        String::new()
    } else {
        format!(", {{ {} }}", options.join(", "))
    };
    let _ = writeln!(
        out,
        "var $response = Test::request(Method::{method}, {}{options});\n",
        literal(&target).unwrap_or_else(|| "''".to_owned())
    );
    let _ = writeln!(
        out,
        "Test::assertEquals($response->status(), {});",
        recorded.status
    );
    let header = |name: &str| {
        recorded
            .response_headers
            .iter()
            .find(|(have, _)| have.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    };
    let content_type = header("content-type");
    if let Some(expected) = content_type.and_then(literal) {
        let _ = writeln!(
            out,
            "Test::assertEquals($response->header('content-type'), {expected});"
        );
    }
    if (300..400).contains(&recorded.status)
        && let Some(expected) = header("location").and_then(literal)
    {
        let _ = writeln!(
            out,
            "Test::assertEquals($response->header('location'), {expected});"
        );
    }
    match &recorded.response_body {
        Ok(body) => out.push_str(&body_assertion(body, content_type)),
        Err(Unchecked::NotRecorded) => {
            out.push_str("// The response body was not recorded, so this test does not check it.\n")
        }
        Err(Unchecked::Cut) => out.push_str(
            "// The recorded response body is not complete, so this test does not check it.\n",
        ),
        Err(Unchecked::Withheld) => out.push_str(
            "// The response body was not recorded because it has a secret. \
             This test does not check it.\n",
        ),
    }
    out.push_str("}\n}\n");
    let mut map = nvs_diagnostics::SourceMap::new();
    let id = map.add("to-test.nvs", out.clone());
    // The printer refuses only text that does not parse, which this module never writes. The
    // unformatted text is still a test, so it is printed rather than lost.
    Ok(nvs_fmt::format(map.file(id)).unwrap_or(out))
}

/// The cases of `Core\Http\Method`, as `Core\Test::request` takes them.
const METHODS: [&str; 8] = [
    "Get", "Head", "Options", "Trace", "Post", "Put", "Patch", "Delete",
];

/// The request headers a test sends: each name once, in the order it arrived, a repeated name's
/// values joined as `header()` joins them. A redacted value stays [`REDACTED`].
fn sent_headers(recorded: &[(String, String)]) -> Vec<(String, String)> {
    let mut headers: Vec<(String, String)> = Vec::new();
    for (name, value) in recorded {
        let lower = name.to_ascii_lowercase();
        if LEFT_OUT.contains(&lower.as_str()) {
            continue;
        }
        match headers.iter_mut().find(|(have, _)| *have == lower) {
            Some((_, joined)) if *joined != REDACTED && value != REDACTED => {
                joined.push_str(if lower == "cookie" { "; " } else { ", " });
                joined.push_str(value);
            }
            Some((_, joined)) => REDACTED.clone_into(joined),
            None => headers.push((lower, value.clone())),
        }
    }
    headers
}

/// The assertion on a whole response `body`.
fn body_assertion(body: &[u8], content_type: Option<&str>) -> String {
    let json = content_type.is_some_and(|value| {
        let media = value.split(';').next().unwrap_or_default().trim();
        media.ends_with("/json") || media.ends_with("+json")
    });
    if json
        && serde_json::from_slice::<serde::de::IgnoredAny>(body).is_ok()
        && let Some(text) = std::str::from_utf8(body)
            .ok()
            .and_then(|text| literal(text.trim()))
    {
        return format!("Test::assertEquals($response->json(), Core\\Json::decode({text}));\n");
    }
    match std::str::from_utf8(body).ok().and_then(literal) {
        Some(text) => format!("Test::assertEquals($response->body(), {text});\n"),
        None => format!(
            "Test::assertEquals($response->bytes(), {});\n",
            hex_of(body)
        ),
    }
}

/// A value for `body:`: a string literal, or bytes from hex when the body is not text a literal
/// can hold.
fn value(body: &[u8]) -> String {
    std::str::from_utf8(body)
        .ok()
        .and_then(literal)
        .unwrap_or_else(|| hex_of(body))
}

/// `Core\Encoding::fromHex('…')` of `bytes`.
fn hex_of(bytes: &[u8]) -> String {
    let mut out = String::from("Core\\Encoding::fromHex('");
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out.push_str("')");
    out
}

/// A Novis string literal whose value is `text`: single-quoted when it has no control character,
/// double-quoted with escapes when it has one, and `None` when it has `ESC`, which no literal may
/// hold (`rule:tooling/an-escape-in-source-text-is-a-compile-error`).
fn literal(text: &str) -> Option<String> {
    let escaped = |c: char| c.is_control() || is_format(c);
    if text.contains('\u{1b}') {
        return None;
    }
    if !text.chars().any(escaped) {
        return Some(format!(
            "'{}'",
            text.replace('\\', "\\\\").replace('\'', "\\'")
        ));
    }
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '$' => out.push_str("\\$"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if escaped(c) => {
                let _ = write!(out, "\\u{{{:x}}}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    Some(out)
}

/// A character that changes how text around it is shown, and is written as an escape so the test
/// reads as it runs.
fn is_format(c: char) -> bool {
    matches!(
        c,
        '\u{200b}'..='\u{200f}'
            | '\u{2028}'..='\u{202e}'
            | '\u{2060}'..='\u{206f}'
            | '\u{061c}'
            | '\u{feff}'
    )
}

/// The test class's name: the method, the path's words, and the trace id's first eight digits, so
/// two tests made from two requests to one route can sit in one program.
fn class_name(method: &str, path: &str, trace: &str) -> String {
    let mut name = method.to_owned();
    for word in path.split(|c: char| !c.is_ascii_alphanumeric()) {
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            name.push(first.to_ascii_uppercase());
            name.push_str(chars.as_str());
        }
    }
    name.push_str(trace.get(..8).unwrap_or(trace));
    name.push_str("Test");
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recorded(
        headers: &[(&str, &str)],
        status: i64,
        response: &[(&str, &str)],
        body: &str,
    ) -> Recorded {
        let pairs = |list: &[(&str, &str)]| {
            list.iter()
                .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
                .collect()
        };
        Recorded {
            trace: "4bf92f3577b34da6a3ce929d0e0e4736".to_owned(),
            method: "POST".to_owned(),
            path: "/orders".to_owned(),
            query: "page=2".to_owned(),
            headers: pairs(headers),
            body: Some(b"{\"item\":7}".to_vec()),
            status,
            response_headers: pairs(response),
            response_body: Ok(body.as_bytes().to_vec()),
        }
    }

    /// A redacted header is the placeholder under its comment, the trace context is left out, and
    /// a JSON body is compared through `json()`.
    #[test]
    fn a_recorded_post_prints_a_test_with_a_placeholder_and_a_json_assertion() {
        let text = print(&recorded(
            &[
                ("Host", "example.com"),
                ("Authorization", REDACTED),
                (
                    "traceparent",
                    "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01",
                ),
                ("Content-Type", "application/json"),
            ],
            201,
            &[("content-type", "application/json"), ("date", "today")],
            "{ \"id\": 7 }",
        ))
        .unwrap_or_else(|refused| panic!("{}", refused.message()));
        assert!(text.contains("class PostOrders4bf92f35Test\n"), "{text}");
        assert!(
            text.contains(
                "        // This value was removed when the request was recorded. Put a test \
                 credential here.\n            'authorization' => 'redacted',\n"
            ),
            "{text}"
        );
        assert!(!text.contains("traceparent"), "{text}");
        assert!(
            text.contains(
                "Test::request(Method::Post, '/orders?page=2', { headers: $headers, body: \
                 '{\"item\":7}' });"
            ),
            "{text}"
        );
        assert!(
            text.contains("Test::assertEquals($response->status(), 201);"),
            "{text}"
        );
        assert!(
            text.contains(
                "Test::assertEquals($response->json(), Core\\Json::decode('{ \"id\": 7 }'));"
            ),
            "{text}"
        );
        assert!(!text.contains("date"), "{text}");
    }

    /// A redirect asserts its `Location`, and a body a literal cannot hold is compared as bytes.
    #[test]
    fn a_redirect_asserts_its_location_and_a_binary_body_is_bytes() {
        let mut request = recorded(
            &[],
            302,
            &[("Location", "/orders/7"), ("Content-Type", "text/plain")],
            "",
        );
        request.response_body = Ok(vec![0x1b, 0xff]);
        let text = print(&request).unwrap_or_else(|refused| panic!("{}", refused.message()));
        assert!(
            text.contains("Test::assertEquals($response->header('location'), '/orders/7');"),
            "{text}"
        );
        assert!(
            text.contains(
                "Test::assertEquals($response->bytes(), Core\\Encoding::fromHex('1bff'));"
            ),
            "{text}"
        );
    }

    /// A cut response body is not asserted, and the comment says so.
    #[test]
    fn a_cut_response_body_asserts_the_status_and_headers_only() {
        let mut request = recorded(&[], 200, &[("Content-Type", "text/html")], "");
        request.response_body = Err(Unchecked::Cut);
        let text = print(&request).unwrap_or_else(|refused| panic!("{}", refused.message()));
        assert!(!text.contains("->body()"), "{text}");
        assert!(text.contains("is not complete"), "{text}");
    }

    #[test]
    fn a_literal_escapes_what_a_reader_cannot_see() {
        assert_eq!(literal("it's"), Some("'it\\'s'".to_owned()));
        assert_eq!(
            literal("a\n$b\u{202e}"),
            Some("\"a\\n\\$b\\u{202e}\"".to_owned())
        );
        assert_eq!(literal("\u{1b}[31m"), None);
    }
}
