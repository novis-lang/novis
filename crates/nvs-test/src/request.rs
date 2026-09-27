//! The request a case answers, frozen into the file `nvs run --request` reads.
//!
//! A case's `--GET--`, `--POST--`, `--POST_RAW--`, `--COOKIE--`,
//! `--HEADERS--`, `--CLIENT_IP--` and `--SCHEME--` describe a request, and the
//! program answering it is a
//! separate process ([the `run` module](mod@crate::run)'s doc owns why), so
//! the description has to cross a process boundary. It crosses as a **file
//! named on the command line** and never as an environment variable: whether
//! a program is answering a request changes what every `Core\Request` member
//! does, and a variable would make that a semantic change nothing at the call
//! site shows.
//!
//! The file is written in the same `--NAME--` shape a case is, so there is no
//! second parser here — [`crate::section`] is the one that reads both:
//!
//! ```text
//! --METHOD--
//! POST
//! --PATH--
//! /
//! --QUERY--
//! page=2&q=novis
//! --CLIENT_IP--
//! 203.0.113.7
//! --SCHEME--
//! https
//! --HEADERS--
//! accept: application/json
//! cookie: session=abc123
//! content-type: application/x-www-form-urlencoded
//! content-length: 20
//! --BODY--
//! name=ada&role=author
//! ```
//!
//! `--BODY--` comes last and is read **verbatim** — everything past its header
//! line to the end of the file, never lexed. A body is arbitrary octets and a
//! multipart one is made of `--` lines, so a body that happened to hold a line
//! reading as a header would otherwise cut the file in half. Taking the
//! remainder rather than a section makes that impossible rather than unlikely.
//!
//! `--BODY_CRLF--` is the same remainder with every line ending sent as CRLF.
//! A multipart body's delimiters are CRLF by RFC 2046, and this repository
//! checks every text file out with LF line endings, so a body that needs CRLF
//! says so in its header line instead of in bytes that checkout rewrites.
//!
//! ## What a case does not write, and where it comes from
//!
//! Three facts a request has that no `.nvst` section spells, derived here and
//! nowhere else: the method is `POST` where the case gave a body and `GET`
//! where it did not, the path is `/`, and the query is `--GET--`'s line or
//! empty. A file written by hand says whatever it likes for all three, which
//! is what makes `nvs run --request` worth having on its own — a request is
//! reproduced without standing a listener up in front of it.
//!
//! **The peer's two facts are sections rather than field lines**, because
//! nothing a peer sends states either one: with `[server] trusted_proxies`
//! empty the forwarded headers are not read at all
//! (`rule:http-server/trusted-proxies-is-empty-and-empty-reads-nothing`), so a
//! file writing `x-forwarded-for` would be describing a claim rather than an
//! answer. `--CLIENT_IP--` is the address that walk settled on and `--SCHEME--`
//! the scheme it decided, each said outright — and a file naming neither
//! describes a request from nobody in particular over plaintext, which is
//! `nvs_runtime::InboundSpec`'s own fail-closed pair rather than a hole for
//! anything downstream to fill.
//!
//! `--COOKIE--`'s pairs are joined into the one `cookie` field a peer would
//! have sent, because the wire has no cookie of its own — only a header — and
//! `content-type` and `content-length` are written for a body the case did not
//! describe itself. Every name is lower-cased, which is what a served request
//! carries (`nvs_runtime::Inbound`'s names come off `hyper`, already
//! normalised), so a case cannot pin one shape here and meet the other in
//! production.
//!
//! Those three derivations are `nvs_runtime::InboundSpec`'s as well, and the
//! duplication is this crate's price for having no dependencies (the manifest
//! says why): a request that never crosses a file is built by that type
//! directly, and one that crosses this one is written out here and read back
//! into it. The two agree on the rule that keeps them from fighting — a field
//! written by hand is never written a second time — so a file that already
//! carries one of the three is carried through the spec untouched.

use std::net::IpAddr;

use crate::case::{Body, Request, Scheme};
use crate::section;

/// The name a runner writes this file under, in the case's own directory.
pub const FILE_NAME: &str = "request.nvsr";

/// A request as the file describes it, ready to be built into a carrier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wire {
    /// The verb as it would arrive on the wire.
    pub method: String,
    /// The path, already without any query string.
    pub path: String,
    /// Everything past the `?`, undecoded, and empty where there was none.
    pub query: String,
    /// One entry per field line, lower-cased, in the order written.
    pub headers: Vec<(String, String)>,
    /// The body, or `None` where the request carries none — which is not the
    /// same state as an empty one (RFC 9110 § 8.6), and the two reach a
    /// program differently.
    pub body: Option<String>,
    /// `--CLIENT_IP--`, the address the request's peer resolved to, and `None`
    /// for a peer with no address at all — the `null`
    /// `Core\Request::clientIp` answers, which is a fact rather than a value
    /// still to be filled in.
    pub client_ip: Option<IpAddr>,
    /// `--SCHEME--`, and [`Scheme::Http`] for a file naming none.
    pub scheme: Scheme,
}

/// Renders the request `case` describes as the file `nvs run --request` reads.
#[must_use]
pub fn render(case: &Request) -> String {
    let body = case.body.as_ref().map(|body| match body {
        Body::Form(pairs) => pairs.clone(),
        Body::Raw(raw) => raw.clone(),
    });
    let mut text = String::from("--METHOD--\n");
    text.push_str(if body.is_some() { "POST\n" } else { "GET\n" });
    text.push_str("--PATH--\n/\n--QUERY--\n");
    text.push_str(case.query.as_deref().unwrap_or(""));
    text.push('\n');
    // Each written only where the case named it, so a rendered file carries no
    // claim about the peer that the case did not make: an absent
    // `--CLIENT_IP--` is a peer with no address and an absent `--SCHEME--` is
    // plaintext, which is what reading one back answers anyway.
    if let Some(client_ip) = case.client_ip {
        text.push_str("--CLIENT_IP--\n");
        text.push_str(&client_ip.to_string());
        text.push('\n');
    }
    if let Some(scheme) = case.scheme {
        text.push_str("--SCHEME--\n");
        text.push_str(scheme.name());
        text.push('\n');
    }
    text.push_str("--HEADERS--\n");
    for (name, value) in &case.headers {
        field(&mut text, name, value);
    }
    if !case.cookies.is_empty() {
        let pairs: Vec<String> = case
            .cookies
            .iter()
            .map(|(name, value)| format!("{name}={value}"))
            .collect();
        field(&mut text, "cookie", &pairs.join("; "));
    }
    if let Some(body) = &body {
        // Only where the case did not describe the body itself: `--POST--`
        // says what its content type is by being urlencoded pairs, and a
        // length is a fact about the octets rather than a choice — but a case
        // that wrote either line meant it, and a field written twice is a
        // field that can disagree with itself.
        if matches!(case.body, Some(Body::Form(_))) && !carries(&case.headers, "content-type") {
            field(
                &mut text,
                "content-type",
                "application/x-www-form-urlencoded",
            );
        }
        if !carries(&case.headers, "content-length") {
            field(&mut text, "content-length", &body.len().to_string());
        }
        text.push_str("--BODY--\n");
        text.push_str(body);
    }
    text
}

/// Reads a rendered request back.
///
/// # Errors
///
/// Returns the one-line reason, already phrased for a terminal: an unknown or
/// repeated section, a missing `--METHOD--` or `--PATH--`, a header line that
/// is not a field, a `--CLIENT_IP--` that is not an address, or a `--SCHEME--`
/// that is neither of the two a request can arrive over.
pub fn read(text: &str) -> Result<Wire, String> {
    // The body is the rest of the file rather than a section, which is this
    // format's one deviation from the shape and the module doc's reason for
    // it. Split first, so nothing in a body is ever handed to the lexer.
    let mut head = String::new();
    let mut body = None;
    let mut lines = text.split_inclusive('\n');
    for line in &mut lines {
        match line.trim_end() {
            "--BODY--" => {
                body = Some(lines.collect());
                break;
            }
            "--BODY_CRLF--" => {
                // Normalised to LF first, so a working copy that checked the
                // file out with CRLF line endings sends the same octets.
                let rest: String = lines.collect();
                body = Some(rest.replace("\r\n", "\n").replace('\n', "\r\n"));
                break;
            }
            _ => head.push_str(line),
        }
    }

    let sections = section::lex(&head)
        .map_err(|stray| format!("line {}: text before `--METHOD--`", stray.line))?;
    let (mut method, mut path, mut query) = (None, None, None);
    let (mut client_ip, mut scheme) = (None, None);
    let mut headers = Vec::new();
    for seen in &sections {
        let (name, at) = (seen.name.as_str(), seen.line);
        if seen.arg.is_some() {
            return Err(format!("line {at}: `--{name}--` takes no argument"));
        }
        let slot = match name {
            "METHOD" => &mut method,
            "PATH" => &mut path,
            "QUERY" => &mut query,
            "CLIENT_IP" => &mut client_ip,
            "SCHEME" => &mut scheme,
            "HEADERS" => {
                for (offset, line) in seen.body.lines().enumerate() {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    let at = at + 1 + offset;
                    let Some((field, value)) = line.split_once(':') else {
                        return Err(format!("line {at}: `{line}` is not a header field"));
                    };
                    let field = field.trim();
                    if field.is_empty() {
                        return Err(format!("line {at}: a header field with no name"));
                    }
                    headers.push((field.to_ascii_lowercase(), value.trim_start().to_owned()));
                }
                continue;
            }
            other => return Err(format!("line {at}: unknown section `--{other}--`")),
        };
        if slot.is_some() {
            return Err(format!("line {at}: `--{name}--` appears twice"));
        }
        *slot = Some(seen.body.trim().to_owned());
    }

    let (Some(method), Some(path)) = (method, path) else {
        return Err("a request states its `--METHOD--` and its `--PATH--`".to_owned());
    };
    if method.is_empty() {
        return Err("`--METHOD--` is empty".to_owned());
    }
    if !path.starts_with('/') {
        return Err(format!(
            "`--PATH--` is a path and starts with `/`, not `{path}`"
        ));
    }
    // A file that names no peer describes one with no address, over plaintext:
    // the pair `nvs_runtime::InboundSpec` starts every spec with, for the same
    // fail-closed reason. A section written empty is refused instead, since a
    // file that opened one meant to say something in it.
    let client_ip = match client_ip {
        None => None,
        Some(text) if text.is_empty() => return Err("`--CLIENT_IP--` is empty".to_owned()),
        Some(text) => Some(
            text.parse::<IpAddr>()
                .map_err(|_| format!("`--CLIENT_IP--` is one client address, not `{text}`"))?,
        ),
    };
    let scheme = match scheme.as_deref() {
        None | Some("http") => Scheme::Http,
        Some("https") => Scheme::Https,
        Some(other) => return Err(format!("`--SCHEME--` is `http` or `https`, not `{other}`")),
    };
    Ok(Wire {
        method,
        path,
        query: query.unwrap_or_default(),
        headers,
        body,
        client_ip,
        scheme,
    })
}

/// Writes one header field line, with the name in the shape a served request
/// carries it in.
fn field(text: &mut String, name: &str, value: &str) {
    text.push_str(&name.to_ascii_lowercase());
    text.push_str(": ");
    text.push_str(value);
    text.push('\n');
}

/// Whether a field of this name is already written, compared the way RFC 9110
/// § 5.1 compares one.
fn carries(headers: &[(String, String)], name: &str) -> bool {
    headers
        .iter()
        .any(|(field, _)| field.eq_ignore_ascii_case(name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// The `Wire` a case writing exactly `sections` is answered with, taken
    /// the way the runner takes it: parsed as a case, rendered as the file,
    /// and read back out of it.
    ///
    /// Every assertion below goes through this rather than building a
    /// [`Request`] by hand, because what a section is worth is what survives
    /// the file — a section the parser reads and the render drops describes a
    /// request the program never answers.
    fn wire(sections: &str) -> Wire {
        let text =
            format!("--TEST--\nthe title\n--FILE--\n<?nvs\necho 1;\n{sections}--EXPECT--\n1\n");
        let case = crate::case::parse(Path::new("t.nvst"), &text).expect("the case parses");
        let request = case.request.expect("the sections describe a request");
        read(&render(&request)).expect("the rendered request reads back")
    }

    /// The first field of this name on the wire, or `None` where the request
    /// carries none.
    fn value_of<'wire>(wire: &'wire Wire, name: &str) -> Option<&'wire str> {
        wire.headers
            .iter()
            .find(|(field, _)| field == name)
            .map(|(_, value)| value.as_str())
    }

    #[test]
    fn a_get_section_becomes_the_requests_query_string() {
        let wire = wire("--GET--\npage=2&q=novis\n");
        assert_eq!(wire.query, "page=2&q=novis");
        // The three facts no section spells: a case with no body is a `GET`,
        // its path is `/`, and its query is `--GET--`'s line undecoded.
        assert_eq!(wire.method, "GET");
        assert_eq!(wire.path, "/");
        assert_eq!(wire.body, None);
    }

    #[test]
    fn a_post_section_becomes_a_urlencoded_body() {
        let wire = wire("--POST--\nname=ada&role=author\n");
        assert_eq!(wire.method, "POST");
        assert_eq!(wire.body.as_deref(), Some("name=ada&role=author"));
        // `--POST--` says what its content type is by being urlencoded pairs,
        // so the case does not write one and the render does.
        assert_eq!(
            value_of(&wire, "content-type"),
            Some("application/x-www-form-urlencoded")
        );
        assert_eq!(value_of(&wire, "content-length"), Some("20"));
    }

    #[test]
    fn a_post_raw_section_is_the_body_verbatim() {
        let wire = wire("--POST_RAW--\n{\"name\":\"ada\"}\n");
        // Verbatim to the last octet, the trailing newline included: a case
        // pinning what a program read out of a body is pinning the octets.
        assert_eq!(wire.body.as_deref(), Some("{\"name\":\"ada\"}\n"));
        assert_eq!(wire.method, "POST");
        // Nothing is assumed about a raw body, so it arrives unlabelled
        // unless the case wrote the label itself.
        assert_eq!(value_of(&wire, "content-type"), None);
        assert_eq!(value_of(&wire, "content-length"), Some("15"));
    }

    #[test]
    fn a_headers_section_pushes_one_line_per_field() {
        let wire = wire("--HEADERS--\nAccept: application/json\nX-Trace: 7\nAccept: text/html\n");
        // One entry per line, in the order written, lower-cased the way a
        // served request carries them — and a repeated name is kept rather
        // than merged, because a message may legitimately carry one twice.
        assert_eq!(
            wire.headers,
            [
                ("accept".to_owned(), "application/json".to_owned()),
                ("x-trace".to_owned(), "7".to_owned()),
                ("accept".to_owned(), "text/html".to_owned()),
            ]
        );
    }

    #[test]
    fn a_cookie_section_becomes_one_cookie_header() {
        let wire = wire("--COOKIE--\nsession=abc123\ntheme=dark\n");
        // The wire has no cookie of its own, only the one field a peer would
        // have sent, so the pairs a case wrote a line each arrive joined.
        assert_eq!(
            wire.headers,
            [("cookie".to_owned(), "session=abc123; theme=dark".to_owned())]
        );
        assert_eq!(wire.method, "GET");
        assert_eq!(wire.body, None);
    }

    #[test]
    fn the_peer_sections_cross_the_file_as_the_case_wrote_them() {
        let wire = wire("--CLIENT_IP--\n2001:db8::1\n--SCHEME--\nhttps\n");
        // Read as an address at both ends rather than carried as text: the one
        // spelling `IpAddr` writes back is what the carrier is given, so a case
        // and the program it runs cannot disagree about which address two
        // spellings of one were.
        assert_eq!(
            wire.client_ip,
            Some("2001:db8::1".parse::<IpAddr>().expect("a v6 address"))
        );
        assert_eq!(wire.scheme, Scheme::Https);
        // Describing a peer is describing a request, so the three facts no
        // section spells are derived for it as for any other.
        assert_eq!(wire.method, "GET");
        assert_eq!(wire.path, "/");
    }

    #[test]
    fn a_peer_the_file_cannot_read_is_refused_rather_than_repaired() {
        // A file written by hand reaches `read` without a case parser ahead of
        // it, so the refusals are here as well — and they are refusals rather
        // than a fallback to the socket peer, which would answer a question the
        // file got wrong instead of saying so.
        assert_eq!(
            read("--METHOD--\nGET\n--PATH--\n/\n--CLIENT_IP--\n203.0.113\n")
                .expect_err("three octets are not an address"),
            "`--CLIENT_IP--` is one client address, not `203.0.113`"
        );
        assert_eq!(
            read("--METHOD--\nGET\n--PATH--\n/\n--CLIENT_IP--\n\n").expect_err("nothing in it"),
            "`--CLIENT_IP--` is empty"
        );
        assert_eq!(
            read("--METHOD--\nGET\n--PATH--\n/\n--SCHEME--\nftp\n")
                .expect_err("a request arrives over one of two schemes"),
            "`--SCHEME--` is `http` or `https`, not `ftp`"
        );
    }

    #[test]
    fn a_case_describing_a_request_renders_what_a_peer_would_have_sent() {
        // The derivations are the point of the assertion: a body makes the
        // verb `POST`, `--COOKIE--`'s pairs are one joined field, and the two
        // facts about the octets are written because the case did not.
        let rendered = render(&Request {
            query: Some("page=2&q=novis".to_owned()),
            body: Some(Body::Form("name=ada".to_owned())),
            cookies: vec![
                ("session".to_owned(), "abc123".to_owned()),
                ("theme".to_owned(), "dark".to_owned()),
            ],
            headers: vec![("Accept".to_owned(), "application/json".to_owned())],
            client_ip: Some(IpAddr::from([203, 0, 113, 7])),
            scheme: Some(Scheme::Https),
        });
        let wire = read(&rendered).expect("it reads back");
        assert_eq!(wire.client_ip, Some(IpAddr::from([203, 0, 113, 7])));
        assert_eq!(wire.scheme, Scheme::Https);
        assert_eq!(wire.method, "POST");
        assert_eq!(wire.path, "/");
        assert_eq!(wire.query, "page=2&q=novis");
        assert_eq!(
            wire.headers,
            [
                ("accept".to_owned(), "application/json".to_owned()),
                ("cookie".to_owned(), "session=abc123; theme=dark".to_owned()),
                (
                    "content-type".to_owned(),
                    "application/x-www-form-urlencoded".to_owned()
                ),
                ("content-length".to_owned(), "8".to_owned()),
            ]
        );
        assert_eq!(wire.body.as_deref(), Some("name=ada"));
    }

    #[test]
    fn a_request_with_no_body_carries_none_rather_than_an_empty_one() {
        let wire = read(&render(&Request {
            query: None,
            body: None,
            cookies: Vec::new(),
            headers: Vec::new(),
            client_ip: None,
            scheme: None,
        }))
        .expect("it reads back");
        assert_eq!(wire.method, "GET");
        assert_eq!(wire.query, "");
        assert!(wire.headers.is_empty());
        assert_eq!(wire.body, None);
        // The peer a case did not describe: no address at all, over plaintext,
        // which is an answer rather than a pair still to be decided.
        assert_eq!(wire.client_ip, None);
        assert_eq!(wire.scheme, Scheme::Http);
    }

    #[test]
    fn a_crlf_body_sends_every_line_ending_as_crlf_whatever_the_checkout_wrote() {
        // A multipart body needs CRLF delimiters, and a working copy may hold
        // the file with either line ending: both read back as the same octets.
        let lf = "--METHOD--\nPOST\n--PATH--\n/\n--BODY_CRLF--\n--b\n\nx\n--b--\n";
        let crlf = lf.replace('\n', "\r\n");
        for text in [lf, crlf.as_str()] {
            let wire = read(text).expect("it reads");
            assert_eq!(wire.body.as_deref(), Some("--b\r\n\r\nx\r\n--b--\r\n"));
            assert_eq!(wire.method, "POST");
        }
    }

    #[test]
    fn a_body_holding_a_line_that_reads_as_a_header_survives_it() {
        // The whole reason `--BODY--` is the rest of the file: a multipart
        // body is made of `--` lines, and one of them reading as a section
        // would cut the request in half and lose the rest of the body.
        let raw = "--PATH--\nnot a section at all\n--METHOD--\n";
        let wire = read(&render(&Request {
            query: None,
            body: Some(Body::Raw(raw.to_owned())),
            cookies: Vec::new(),
            headers: vec![("Content-Type".to_owned(), "text/plain".to_owned())],
            client_ip: None,
            scheme: None,
        }))
        .expect("it reads back");
        assert_eq!(wire.body.as_deref(), Some(raw));
        assert_eq!(wire.path, "/");
        assert_eq!(
            wire.headers,
            [
                ("content-type".to_owned(), "text/plain".to_owned()),
                ("content-length".to_owned(), raw.len().to_string()),
            ]
        );
    }

    #[test]
    fn a_request_that_does_not_describe_one_is_refused() {
        assert_eq!(
            read("--PATH--\n/\n").expect_err("no method"),
            "a request states its `--METHOD--` and its `--PATH--`"
        );
        assert_eq!(
            read("--METHOD--\nGET\n--PATH--\n/\n--WHAT--\n?\n").expect_err("an unknown section"),
            "line 5: unknown section `--WHAT--`"
        );
        assert_eq!(
            read("--METHOD--\nGET\n--PATH--\n/\n--HEADERS--\naccept\n").expect_err("not a field"),
            "line 6: `accept` is not a header field"
        );
        assert_eq!(
            read("--METHOD--\nGET\n--PATH--\nusers\n").expect_err("not a path"),
            "`--PATH--` is a path and starts with `/`, not `users`"
        );
    }
}
