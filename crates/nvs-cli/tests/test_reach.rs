//! What a test can set and read, driven through the built binary: `nvs test`
//! runs a fixture under `tests/fixtures/reach/`, and each Rust test names the
//! `#[Test]` methods it stands for with `--filter`.
//!
//! The peer tests are `rule:testing/in-process-request`'s *the peer is
//! stated*: `Core\Test::request`'s `clientIp`, `scheme` and `cookies`
//! options, the `host` header that picks the mount row, and a `traceparent`
//! header the request continues.
//!
//! The body tests are its *a body is written with `Core\Http\Client`'s
//! options*: `json`, `form`, `multipart` and a `body` with its `contentType`
//! reach the request readers, and two bodies do not compile.

use std::path::{Path, PathBuf};
use std::process::Command;

/// `tests/fixtures/reach/<name>`.
fn fixture(name: &str) -> PathBuf {
    [
        env!("CARGO_MANIFEST_DIR"),
        "tests",
        "fixtures",
        "reach",
        name,
    ]
    .iter()
    .collect()
}

/// `nvs test <file> --filter <filter>`, run from `dir`, as `(report, success)`.
fn run_in(dir: &Path, file: &str, filter: &str) -> (String, bool) {
    let data = nvs_repo::scratch_private("nvsdata");
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--data")
        .arg(&*data)
        .arg("--no-init")
        .arg("test")
        .arg(file)
        .args(["--filter", filter])
        .current_dir(dir)
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    let report = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (report, out.status.success())
}

/// Runs the one `#[Test]` method `method` of `peer.nvs` and asserts that it
/// alone ran, and passed.
fn passes_in_peer(method: &str) {
    let (report, passed) = run_in(&fixture(""), "peer.nvs", method);
    assert!(passed, "`{method}` failed:\n{report}");
    assert!(
        report.contains(method) && report.contains("0 failed, 1 passed"),
        "`{method}` was the one test that ran, and it passed:\n{report}"
    );
}

/// Runs the one `#[Test]` method `method` of `bodies.nvs` and asserts that it
/// alone ran, and passed.
fn passes_in_bodies(method: &str) {
    let (report, passed) = run_in(&fixture(""), "bodies.nvs", method);
    assert!(passed, "`{method}` failed:\n{report}");
    assert!(
        report.contains(method) && report.contains("0 failed, 1 passed"),
        "`{method}` was the one test that ran, and it passed:\n{report}"
    );
}

/// `json` reaches `Core\Request::json()` under `application/json`, and
/// `json: null` sends the document `null`.
// covers: Core\Test::request
#[test]
fn a_json_option_reaches_request_json() {
    passes_in_bodies("jsonReachesRequestJson");
}

/// `form` reaches `Core\Request::post()` under
/// `application/x-www-form-urlencoded`, with a space, a `+` and a `=` in it.
// covers: Core\Test::request
#[test]
fn a_form_option_reaches_request_post() {
    passes_in_bodies("formReachesRequestPost");
}

/// `multipart` sends a `Core\Http\Part` that `Core\Request::files()` reads
/// with its name, file name, media type and length, and a text field that
/// `Core\Request::post()` reads.
// covers: Core\Test::request
#[test]
fn a_multipart_option_reaches_request_files() {
    passes_in_bodies("multipartReachesRequestFiles");
}

/// A `body` with `contentType` is sent under that type, and one without it
/// under none.
// covers: Core\Test::request
#[test]
fn a_body_with_a_content_type_is_sent_as_written() {
    passes_in_bodies("bodyWithAContentTypeIsSentAsWritten");
}

/// Two body keys, and `contentType` without `body`, are the client's two
/// compile errors.
// covers: Core\Test::request
#[test]
fn two_body_options_do_not_compile() {
    let (report, passed) = run_in(&fixture(""), "two_bodies.nvs", "twoBodies");
    assert!(!passed, "two bodies compiled:\n{report}");
    assert!(
        report.contains("E0815") && report.contains("given two bodies, `json` and `form`"),
        "two bodies are E0815:\n{report}"
    );
    assert!(
        report.contains("E0817") && report.contains("`contentType` is written without `body`"),
        "a media type with no body is E0817:\n{report}"
    );
}

/// `clientIp` is what `Core\Request::clientIp()` returns, for an IPv4 and an
/// IPv6 address.
// covers: Core\Test::request
#[test]
fn a_test_request_has_the_client_address_it_states() {
    passes_in_peer("clientAddressIsStated");
}

/// `scheme: "https"` makes a `__Host-` cookie visible, and the same cookie
/// over the default `http` is not.
// covers: Core\Test::request
#[test]
fn a_test_request_over_https_reads_a_host_prefixed_cookie() {
    passes_in_peer("httpsReadsAHostCookie");
}

/// A request that states no peer has no client address and is `http`.
// covers: Core\Test::request
#[test]
fn a_test_request_with_no_peer_has_a_null_client_address() {
    passes_in_peer("noPeerHasNoAddress");
}

/// A host name, an address with a port and a scheme that is not `http` or
/// `https` are each a `LogicError`.
// covers: Core\Test::request
#[test]
fn a_bad_client_address_is_a_logic_error() {
    passes_in_peer("badPeerIsALogicError");
}

/// `cookies` reaches `Core\Request::cookie()` and travels as one `cookie`
/// field line, and a call with none sends none.
// covers: Core\Test::request
#[test]
fn the_cookies_option_reaches_the_cookie_readers() {
    passes_in_peer("cookiesReachTheReaders");
}

/// A `host` header picks the row the server picks for it, after the port and
/// the case are folded away, so a link to another module is the server's link
/// on that host. `links_between_modules.rs` pins the same answer under `nvs
/// serve`. With no host, the request takes the row that has none.
// covers: Core\Test::request
#[test]
fn the_host_header_picks_the_mount_row() {
    let (report, passed) = run_in(
        &fixture("hosts"),
        "Shop/public/index.nvs",
        "theHostPicksTheRow",
    );
    assert!(passed, "the host test failed:\n{report}");
    assert!(
        report.contains("0 failed, 1 passed"),
        "the one test ran and passed:\n{report}"
    );
}

/// A `traceparent` header is continued, so `Core\Server::traceId()` returns
/// the trace id it names.
// covers: Core\Test::request
#[test]
fn a_traceparent_header_is_continued() {
    passes_in_peer("traceparentIsContinued");
}
