//! `nvs run --request <file>`: the program answers the request that file
//! describes, and refuses to invent one where no file was named.
//!
//! Through the built binary rather than against `inbound_from` directly, for
//! the reason the `bundle` suite already writes down — `nvs-cli` has no
//! library target — and because what is asserted here is the whole crossing
//! and not one link of it: `nvs_test::request`'s file is read into an
//! `Inbound`, the `Ctx` carries it, and `Core\Request` answers off it. A test
//! that stopped at the carrier would pass while the program still saw
//! nothing.
//!
//! The format itself is `nvs_test::request`'s and is asserted there;
//! `rule:security/request-state-throws-in-an-isolate` is what the second test
//! pins.

use std::path::PathBuf;
use std::process::{Command, Output};

/// The directory every fixture in this file lives in.
fn fixtures() -> PathBuf {
    [env!("CARGO_MANIFEST_DIR"), "tests", "fixtures", "request"]
        .iter()
        .collect()
}

/// Runs the fixture program, answering the named request file where there is
/// one, and hands back the whole result: one test wants the output and the
/// other wants the refusal.
fn run(request: Option<&str>) -> Output {
    let data = nvs_repo::scratch_private("nvsdata");
    let mut command = Command::new(env!("CARGO_BIN_EXE_nvs"));
    command
        .arg("--data")
        .arg(&*data)
        .arg("--no-init")
        .arg("run");
    if let Some(file) = request {
        command.arg("--request").arg(fixtures().join(file));
    }
    command
        .arg(fixtures().join("reads-the-request.nvs"))
        .output()
        .expect("the `nvs` binary this test was built beside runs")
}

#[test]
fn a_request_file_becomes_the_inbound_the_program_answers() {
    let out = run(Some("reads-the-request.nvsr"));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "the program answers it: {stderr}");
    // Every line is a section of the file: the verb, the path, one field of
    // the query, one header, one cookie off the `cookie` field, and the body
    // verbatim to its trailing newline.
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "method: POST\npath: /users\nquery: novis\nheader: 7\ncookie: abc123\nbody: name=ada\n"
    );
}

#[test]
fn a_run_without_a_request_file_answers_no_request() {
    let out = run(None);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "there is no request to answer");
    // Refused rather than answered empty, and refused as the program's own
    // mistake: "no request arrived" and "the request sent nothing" are
    // different facts, and a peer must never be able to raise this.
    assert!(
        stderr.contains("LogicError"),
        "the program's mistake: {stderr}"
    );
    assert!(
        stderr.contains("there is no request here"),
        "it says which of the two states this is: {stderr}"
    );
    assert!(
        out.stdout.is_empty(),
        "nothing is answered before the refusal"
    );
}

/// Runs the event-stream fixture as the answer to the fixture request, with
/// the events file beside it where `events` says so.
fn stream(events: bool) -> Output {
    let data = nvs_repo::scratch_private("nvsdata");
    let mut command = Command::new(env!("CARGO_BIN_EXE_nvs"));
    command
        .arg("--data")
        .arg(&*data)
        .arg("--no-init")
        .arg("run")
        .arg("--request")
        .arg(fixtures().join("reads-the-request.nvsr"));
    if events {
        command
            .arg("--events")
            .arg(fixtures().join("opens-an-event-stream.nvse"));
    }
    command
        .arg(fixtures().join("opens-an-event-stream.nvs"))
        .output()
        .expect("the `nvs` binary this test was built beside runs")
}

/// `rule:concurrency/two-doors-one-isolate`: every request is offered the SSE
/// cell, so the stream its `Core\Sse::upgrade` prepared starts once it has
/// ended, and `--events` feeds that stream rather than the request. Only the
/// topic the stream subscribed to reaches it, and the connection's own `echo`
/// follows the stream.
// covers: Core\Sse::upgrade
#[test]
fn a_request_that_upgrades_to_an_event_stream_runs_it_fed_from_the_events_file() {
    let out = stream(true);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "the request ended normally: {stderr}");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "data: hello ada\n\nevent: news\ndata: first\n\nevent: news\ndata: second\n\nclient left\n"
    );
}

/// With no `--events` the feed is empty, so the stream's first wait ends it
/// as a client that left does, and the run still ends.
// covers: Core\Sse::upgrade
#[test]
fn an_upgraded_event_stream_with_no_events_file_ends_at_its_first_wait() {
    let out = stream(false);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "the request ended normally: {stderr}");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "data: hello ada\n\nclient left\n"
    );
}
