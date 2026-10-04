//! `rule:core-classes/html-later`'s slotted delivery, read off the wire of a
//! served request.

use std::io::Read as _;
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use nvs_runtime::{Ctx, NativeBody, Value};

use super::{POLYFILL, POLYFILL_HASH, TRIGGER, TRIGGER_HASH, allow_scripts};
use crate::serve::tests::served_page;

/// A slot that waits `MS` milliseconds, then writes one paragraph.
struct After<const MS: u64>;
impl<const MS: u64> NativeBody for After<MS> {
    fn run(ctx: &mut Ctx) {
        let _ = nvs_host::sleep(Duration::from_millis(MS));
        ctx.write_output(format!("<p>after {MS}</p>").as_bytes())
            .expect("a slot's body refused a write");
    }
}

/// A slot that writes one byte at once.
struct Writes;
impl NativeBody for Writes {
    fn run(ctx: &mut Ctx) {
        ctx.write_output(b"X")
            .expect("a slot's body refused a write");
    }
}

/// A slot that meets a limit of the request tree, once a quicker slot has
/// finished.
struct Breaches;
impl NativeBody for Breaches {
    fn run(ctx: &mut Ctx) {
        let _ = nvs_host::sleep(Duration::from_millis(60));
        ctx.set_pending_fatal("the request reached its memory limit");
    }
}

const TOP: &str = "<html><body><h1>Blog</h1>";
const END: &str = "</body></html>";
const LOADING: &[u8] = b"<p>loading</p>";
const FAILED: &[u8] = b"<p>failed</p>";

/// Writes a page with one placeholder per slot in `slots`, between [`TOP`]
/// and [`END`].
fn page(ctx: &mut Ctx, slots: &[Value]) {
    write(ctx, TOP.as_bytes());
    for &closure in slots {
        let marker = ctx.register_later(closure, LOADING, FAILED, None);
        write(ctx, &marker);
    }
    write(ctx, END.as_bytes());
}

fn write(ctx: &mut Ctx, bytes: &[u8]) {
    ctx.write_output(bytes)
        .expect("a captured body refused a write");
}

/// The whole response, read to its end.
fn whole(mut socket: TcpStream, _: std::sync::Arc<crate::admit::Admission>) -> String {
    let mut answer = String::new();
    socket
        .read_to_string(&mut answer)
        .expect("the response could not be read");
    answer
}

/// A slotted page with `slots`, made slotted by the call, read whole.
fn slotted(slots: fn() -> Vec<Value>) -> String {
    served_page(
        move |ctx| {
            ctx.make_slotted()
                .expect("the main script could not make its response slotted");
            page(ctx, &slots());
        },
        None,
        whole,
    )
}

/// The head, lower-cased, and the body with its chunk framing taken off.
fn split(answer: &str) -> (String, String) {
    let (head, framed) = answer
        .split_once("\r\n\r\n")
        .expect("the response had no head");
    let head = head.to_ascii_lowercase();
    if !head.contains("transfer-encoding: chunked") {
        return (head, framed.to_owned());
    }
    let mut body = String::new();
    let mut rest = framed;
    while let Some((size, after)) = rest.split_once("\r\n") {
        let size = usize::from_str_radix(size.trim(), 16).expect("a chunk had no size");
        if size == 0 {
            break;
        }
        body.push_str(&after[..size]);
        rest = &after[size + 2..];
    }
    (head, body)
}

/// Where each fill's `<template for>` opens, in the order they arrived.
fn fills(body: &str) -> Vec<usize> {
    body.match_indices("<template for=\"")
        .map(|(at, _)| at)
        .collect()
}

#[test]
fn a_slotted_route_sends_its_shell_while_a_later_is_still_parked() {
    static FINISHED: AtomicBool = AtomicBool::new(false);
    struct Parks;
    impl NativeBody for Parks {
        fn run(ctx: &mut Ctx) {
            let _ = nvs_host::sleep(Duration::from_millis(300));
            FINISHED.store(true, Ordering::Relaxed);
            ctx.write_output(b"<p>late</p>")
                .expect("a slot's body refused a write");
        }
    }
    let answer = served_page(
        |ctx| {
            ctx.make_slotted()
                .expect("the main script could not make its response slotted");
            page(ctx, &[nvs_runtime::native_closure::<Parks>()]);
        },
        None,
        |mut socket, _| {
            let mut seen = Vec::new();
            let mut buffer = [0u8; 4096];
            while !String::from_utf8_lossy(&seen).contains("<h1>Blog</h1>") {
                let read = socket
                    .read(&mut buffer)
                    .expect("the response could not be read");
                assert!(read > 0, "the response ended before its shell arrived");
                seen.extend_from_slice(&buffer[..read]);
            }
            let finished = FINISHED.load(Ordering::Relaxed);
            socket
                .read_to_end(&mut seen)
                .expect("the response could not be read");
            format!("{finished}|{}", String::from_utf8_lossy(&seen))
        },
    );
    let (finished, answer) = answer.split_once('|').expect("the client wrote no verdict");
    assert_eq!(finished, "false", "the shell waited for the slot: {answer}");
    let (_, body) = split(answer);
    assert!(
        body.contains("<p>late</p></template>"),
        "the slot's fill never arrived: {body}"
    );
}

#[test]
fn fills_arrive_in_the_order_their_later_finished() {
    let answer = slotted(|| {
        vec![
            nvs_runtime::native_closure::<After<300>>(),
            nvs_runtime::native_closure::<After<30>>(),
        ]
    });
    let (_, body) = split(&answer);
    let fast = body
        .find("<p>after 30</p>")
        .expect("the fast slot's fill is missing");
    let slow = body
        .find("<p>after 300</p>")
        .expect("the slow slot's fill is missing");
    assert!(
        fast < slow,
        "the slot that finished first was sent second: {body}"
    );
}

#[test]
fn the_polyfill_is_sent_once_and_only_with_a_fill() {
    let answer = slotted(|| {
        vec![
            nvs_runtime::native_closure::<After<30>>(),
            nvs_runtime::native_closure::<After<60>>(),
        ]
    });
    let (_, body) = split(&answer);
    assert_eq!(
        body.matches(POLYFILL).count(),
        1,
        "the polyfill was not sent once: {body}"
    );
    let polyfill = body.find(POLYFILL).expect("the polyfill is missing");
    assert!(
        polyfill < fills(&body)[0],
        "the polyfill came after a fill: {body}"
    );

    // A placeholder the page never writes: the slot is cancelled, nothing
    // is filled, and the page goes out whole without the polyfill.
    let answer = served_page(
        |ctx| {
            ctx.make_slotted()
                .expect("the main script could not make its response slotted");
            write(ctx, TOP.as_bytes());
            let _ = ctx.register_later(nvs_runtime::native_closure::<Writes>(), b"", b"", None);
            write(ctx, END.as_bytes());
        },
        None,
        whole,
    );
    let (_, body) = split(&answer);
    assert!(
        !body.contains("_nvs"),
        "a page with no fill carried a script: {body}"
    );
}

#[test]
fn the_end_of_the_document_is_held_back_until_the_last_fill() {
    let answer = slotted(|| {
        vec![
            nvs_runtime::native_closure::<After<30>>(),
            nvs_runtime::native_closure::<After<60>>(),
        ]
    });
    let (_, body) = split(&answer);
    assert!(
        body.ends_with(END),
        "the page did not end with its end: {body}"
    );
    let last = *fills(&body).last().expect("no fill arrived");
    assert!(
        last < body.len() - END.len(),
        "a fill came after the end: {body}"
    );
    assert_eq!(
        body.matches("</body>").count(),
        1,
        "the end was sent twice: {body}"
    );
}

#[test]
fn the_polyfill_and_trigger_hashes_are_added_only_to_a_policy_that_limits_scripts() {
    let amended = |policy: &str| {
        let mut headers = hyper::HeaderMap::new();
        headers.insert(
            hyper::header::CONTENT_SECURITY_POLICY,
            hyper::header::HeaderValue::from_str(policy).expect("a policy the wire can carry"),
        );
        allow_scripts(&mut headers);
        headers[hyper::header::CONTENT_SECURITY_POLICY]
            .to_str()
            .expect("an amended policy is text")
            .to_owned()
    };
    let both = format!("{POLYFILL_HASH} {TRIGGER_HASH}");
    for unchanged in [
        "frame-ancestors 'none'",
        "img-src 'self'",
        "script-src 'self' 'unsafe-inline'",
    ] {
        assert_eq!(
            amended(unchanged),
            unchanged,
            "a policy that allows the scripts changed"
        );
    }
    assert_eq!(
        amended("default-src 'self'; frame-ancestors 'none'"),
        format!("default-src 'self' {both}; frame-ancestors 'none'")
    );
    assert_eq!(
        amended("frame-ancestors 'none'; script-src 'none'"),
        format!("frame-ancestors 'none'; script-src {both}")
    );
    assert_eq!(
        amended("script-src 'nonce-abc' 'unsafe-inline'"),
        format!("script-src 'nonce-abc' 'unsafe-inline' {both}")
    );
    assert_eq!(
        amended("script-src 'self'; script-src-elem 'self'"),
        format!("script-src 'self'; script-src-elem 'self' {both}")
    );

    // On the wire: a policy the page declared, and the shipped one.
    let answer = served_page(
        |ctx| {
            ctx.make_slotted()
                .expect("the main script could not make its response slotted");
            ctx.declare_header("Content-Security-Policy", "default-src 'self'");
            page(ctx, &[nvs_runtime::native_closure::<Writes>()]);
        },
        None,
        whole,
    );
    let (head, _) = split(&answer);
    assert!(
        head.contains(&format!("default-src 'self' {both}").to_ascii_lowercase()),
        "a declared policy did not learn the hashes: {head}"
    );
    let (head, _) = split(&slotted(|| vec![nvs_runtime::native_closure::<Writes>()]));
    assert!(
        !head.contains("sha256-"),
        "the shipped policy learned hashes: {head}"
    );
}

#[test]
fn every_fill_ends_with_the_trigger() {
    let answer = slotted(|| {
        vec![
            nvs_runtime::native_closure::<After<30>>(),
            nvs_runtime::native_closure::<Writes>(),
            nvs_runtime::native_closure::<After<60>>(),
        ]
    });
    let (_, body) = split(&answer);
    let trigger = format!("</template><script>{TRIGGER}</script>");
    assert_eq!(fills(&body).len(), 3, "not every slot was filled: {body}");
    assert_eq!(
        body.matches(&trigger).count(),
        3,
        "a fill ended without the trigger: {body}"
    );
}

#[test]
fn the_polyfill_is_at_most_one_kilobyte() {
    assert!(
        POLYFILL.len() <= 1024,
        "the polyfill is {} bytes",
        POLYFILL.len()
    );
}

#[test]
fn a_slot_adds_at_most_120_bytes_beyond_its_content() {
    // Twelve slots, so the slot numbers reach two digits. Each writes one byte
    // and has an empty placeholder.
    let answer = served_page(
        |ctx| {
            ctx.make_slotted()
                .expect("the main script could not make its response slotted");
            write(ctx, TOP.as_bytes());
            for _ in 0..12 {
                let marker =
                    ctx.register_later(nvs_runtime::native_closure::<Writes>(), b"", b"", None);
                write(ctx, &marker);
            }
            write(ctx, END.as_bytes());
        },
        None,
        whole,
    );
    let (_, body) = split(&answer);
    let polyfill = "<script></script>".len() + POLYFILL.len();
    let overhead = body.len() - TOP.len() - END.len() - polyfill - 12;
    assert!(
        overhead <= 12 * 120,
        "twelve slots added {overhead} bytes: {body}"
    );
}

#[test]
fn a_slot_name_carries_a_16_character_token() {
    let (_, body) = split(&slotted(|| vec![nvs_runtime::native_closure::<Writes>()]));
    let at = fills(&body)[0] + "<template for=\"".len();
    let name = &body[at..at + body[at..].find('"').expect("an unterminated name")];
    let (prefix, rest) = name.split_once('-').expect("a name with no prefix");
    let (token, number) = rest.rsplit_once('-').expect("a name with no number");
    assert_eq!(prefix, "nvs");
    assert_eq!(number, "0");
    assert_eq!(token.len(), 16, "the token in {name} is not 16 characters");
    assert!(
        token
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'),
        "the token in {name} is not URL-safe base64"
    );
    assert!(
        body.contains(&format!("<?start name=\"{name}\">")),
        "no placeholder has that name"
    );
}

#[test]
fn the_polyfill_and_trigger_hashes_match_their_files() {
    use base64::Engine as _;
    use sha2::Digest as _;
    let hash = |text: &str| {
        let digest = sha2::Sha256::digest(text.as_bytes());
        format!(
            "'sha256-{}'",
            base64::engine::general_purpose::STANDARD.encode(digest)
        )
    };
    assert_eq!(
        hash(POLYFILL),
        POLYFILL_HASH,
        "polyfill.js changed without its hash"
    );
    assert_eq!(
        hash(TRIGGER),
        TRIGGER_HASH,
        "trigger.js changed without its hash"
    );
}

#[test]
fn a_slotted_response_carries_no_content_length_and_turns_off_proxy_buffering() {
    let (head, _) = split(&slotted(|| vec![nvs_runtime::native_closure::<Writes>()]));
    assert!(
        !head.contains("content-length"),
        "a slotted response has a length: {head}"
    );
    assert!(
        head.contains("x-accel-buffering: no"),
        "a proxy may buffer it: {head}"
    );
    assert!(
        head.contains("content-type: text/html"),
        "the page lost its type: {head}"
    );
}

#[test]
fn slotted_from_a_route_attribute_and_from_the_call_are_the_same() {
    let by_route = served_page(
        |ctx| page(ctx, &[nvs_runtime::native_closure::<Writes>()]),
        Some(
            nvs_runtime::routes::Route::new("GET", "/page", None, "Blog::show", None, vec![])
                .with_slotted(),
        ),
        whole,
    );
    let by_call = slotted(|| vec![nvs_runtime::native_closure::<Writes>()]);
    let (head, body) = split(&by_route);
    assert!(
        head.contains("x-accel-buffering: no"),
        "the route's page was not slotted: {head}"
    );
    // The token differs per request, so the two are compared without it.
    let shape = |body: &str| {
        let at = body.find("nvs-").expect("no slot name");
        body.replace(&body[at + 4..at + 20], "")
    };
    let (_, called) = split(&by_call);
    assert_eq!(shape(&body), shape(&called));
}

#[test]
fn a_slotted_route_without_later_is_sent_whole() {
    let answer = served_page(
        |ctx| {
            ctx.make_slotted()
                .expect("the main script could not make its response slotted");
            page(ctx, &[]);
        },
        None,
        whole,
    );
    let (head, body) = split(&answer);
    assert!(
        head.contains("content-length"),
        "a page with no slot was streamed: {head}"
    );
    assert_eq!(body, format!("{TOP}{END}"));
}

#[test]
fn a_limit_breach_on_a_slotted_route_fills_every_unfilled_slot_with_its_error_and_ends() {
    // No slot here is parked in its own body when the breach stops the
    // others: a test slot is a native function, and a parked one cannot be
    // torn down. The two quick slots finish before the breach.
    let answer = slotted(|| {
        vec![
            nvs_runtime::native_closure::<Writes>(),
            nvs_runtime::native_closure::<Breaches>(),
            nvs_runtime::native_closure::<Writes>(),
        ]
    });
    let (head, body) = split(&answer);
    assert!(
        head.starts_with("http/1.1 200"),
        "the status changed after the head: {head}"
    );
    let failed = String::from_utf8_lossy(FAILED);
    assert_eq!(
        fills(&body).len(),
        3,
        "a slot was left without a fill: {body}"
    );
    assert_eq!(
        body.matches(&*failed).count(),
        1,
        "the breached slot did not fail: {body}"
    );
    let breached = body.rfind("-1\">").expect("the breached slot has no fill");
    assert!(
        body[breached..].starts_with(&format!("-1\">{failed}</template>")),
        "the breached slot did not show its error: {body}"
    );
    assert!(body.ends_with(END), "the page did not end: {body}");
}

#[test]
fn a_slotted_response_holds_its_admission_place_until_its_last_fill() {
    let answer = served_page(
        |ctx| {
            ctx.make_slotted()
                .expect("the main script could not make its response slotted");
            page(ctx, &[nvs_runtime::native_closure::<After<300>>()]);
        },
        None,
        |mut socket, admission| {
            let mut seen = Vec::new();
            let mut buffer = [0u8; 4096];
            while !String::from_utf8_lossy(&seen).contains("<h1>Blog</h1>") {
                let read = socket
                    .read(&mut buffer)
                    .expect("the response could not be read");
                assert!(read > 0, "the response ended before its shell arrived");
                seen.extend_from_slice(&buffer[..read]);
            }
            admission.in_flight().to_string()
        },
    );
    assert_eq!(
        answer, "1",
        "the place was given back once the shell was sent"
    );
}
