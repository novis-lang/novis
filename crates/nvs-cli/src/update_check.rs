//! `nvs update-check` — `rule:tooling/update-check-states-the-whole-picture`.
//!
//! **One line per step, written as it happens.** The telemetry switch and the local version are
//! printed before the request goes out, and the request's URL is named before it is sent, so a
//! slow network shows where the command waits. `--json` prints one document at the end instead,
//! with `schema` first and its keys in a fixed order.
//!
//! **The exit status is `0` whatever happened.** An update, no update and a request that failed
//! all exit `0`: a script reads `update_available` and `error` from the JSON, and a cron job does
//! not fail because the network did.
//!
//! **The latest release is GitHub's.** The request is one `GET` of [`endpoints::update_url`]
//! through [`nvs_stdlib::tool_get`], the transport `Core\Http\Client` uses, and the version is the
//! document's `tag_name` with a leading `v` removed. The switch is [`telemetry::is_on`], printed
//! beside the upload URL `NVS_TELEMETRY_URL` names.
//!
//! **It carries the telemetry upload.** With telemetry on, [`telemetry::report`] sends the
//! finished week once the release is read, and its line and the upload history follow the
//! release lines. A failed upload changes neither the exit status nor the release result.
//!
//! Cost: one HTTPS request per run, its reply capped at 1 MiB, and only when a user runs the
//! command. With telemetry on and a week due, one more request of at most three seconds. Nothing
//! here runs in a server.

use std::io::Write;
use std::process::ExitCode;
use std::time::Duration;

use crate::{endpoints, telemetry};

/// How long the whole request may take, connecting included.
const WITHIN: Duration = Duration::from_secs(10);

/// The version of this binary, as `Cargo.toml` writes it.
const LOCAL: &str = env!("CARGO_PKG_VERSION");

/// Check whether a newer version of Novis is released.
///
/// This prints whether telemetry is on, the version you run and the latest
/// released version. The latest version is read from GitHub, and
/// `NVS_UPDATE_URL` replaces that address. When telemetry is on, it also
/// sends the last finished week if it is due, and prints the upload history.
/// The command always exits with status 0, so use `--json` in a script.
#[derive(clap::Args)]
pub(crate) struct Args {
    /// Print one JSON document at the end, in place of one line per step.
    #[arg(long)]
    json: bool,
}

impl Args {
    /// Runs `nvs update-check` against [`endpoints::update_url`].
    pub(crate) fn run(self) -> ExitCode {
        check(
            &endpoints::update_url(),
            telemetry::is_on(),
            endpoints::telemetry_url().as_deref(),
            self.json,
            telemetry::report,
            &mut std::io::stdout(),
        )
    }
}

/// What one check found.
struct Found {
    /// The latest released version, or `None` where the request failed.
    latest: Option<String>,
    /// Why the request failed, or `None` where it did not.
    error: Option<String>,
}

/// The whole command against `url`, with telemetry `on` or off and `upload` as its upload URL,
/// written to `out`. `report` carries the upload once the release is read. Always
/// [`ExitCode::SUCCESS`].
fn check(
    url: &str,
    on: bool,
    upload: Option<&str>,
    json: bool,
    report: impl FnOnce() -> Option<telemetry::Report>,
    out: &mut impl Write,
) -> ExitCode {
    let switch = if on { "on" } else { "off" };
    if !json {
        step(
            out,
            &format!("telemetry: {switch}, {}", upload_line(upload)),
        );
        step(out, &format!("this version: {LOCAL}"));
        step(out, &format!("reading the latest release from {url}"));
    }
    let found = match latest(url) {
        Ok(latest) => Found {
            latest: Some(latest),
            error: None,
        },
        Err(error) => Found {
            latest: None,
            error: Some(error),
        },
    };
    if json {
        let report = report();
        step(out, &document(&found, switch, upload, report.as_ref()));
    } else {
        match (&found.latest, &found.error) {
            (Some(latest), _) => {
                step(out, &format!("latest version: {latest}"));
                if newer(latest, LOCAL) {
                    step(out, &format!("an update is available: {LOCAL} to {latest}"));
                } else {
                    step(out, "you have the latest version");
                }
            }
            (None, error) => step(
                out,
                &format!(
                    "could not read the latest release: {}",
                    error.as_deref().unwrap_or_default()
                ),
            ),
        }
        if let Some(report) = report() {
            if let Some(outcome) = &report.outcome {
                step(out, &outcome.line());
            }
            for line in report.history.lines() {
                step(out, &line);
            }
        }
    }
    ExitCode::SUCCESS
}

/// The telemetry line's second half.
fn upload_line(upload: Option<&str>) -> String {
    upload.map_or_else(
        || {
            format!(
                "and nothing is uploaded because `{}` is not set",
                endpoints::TELEMETRY_VARIABLE
            )
        },
        |url| format!("and the upload URL is {url}"),
    )
}

/// Writes `line` and flushes it, so the user sees each step as it happens. A closed output is
/// not an error this command reports.
fn step(out: &mut impl Write, line: &str) {
    let _ = writeln!(out, "{line}");
    let _ = out.flush();
}

/// The latest version the Releases API document at `url` names.
///
/// # Errors
///
/// The text of a failed request, a status other than `200`, and a reply with no `tag_name`.
fn latest(url: &str) -> Result<String, String> {
    let headers = vec![
        ("User-Agent".to_owned(), format!("nvs/{LOCAL}")),
        (
            "Accept".to_owned(),
            "application/vnd.github+json".to_owned(),
        ),
    ];
    let reply = nvs_stdlib::tool_get("nvs update-check", url, headers, WITHIN)?;
    if reply.status == 404 {
        return Err(
            "the server returned status 404, so no release is published at this address".to_owned(),
        );
    }
    if reply.status != 200 {
        return Err(format!("the server returned status {}", reply.status));
    }
    let document: serde_json::Value = serde_json::from_slice(&reply.body)
        .map_err(|error| format!("the reply is not JSON: {error}"))?;
    let tag = document
        .get("tag_name")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "the reply has no `tag_name`".to_owned())?;
    Ok(tag.trim().trim_start_matches(['v', 'V']).to_owned())
}

/// Whether `latest` is a later version than `local`: the numbers compared in order, a missing
/// one read as `0`, and a release after a pre-release of the same numbers. Build metadata after
/// `+` is ignored.
fn newer(latest: &str, local: &str) -> bool {
    fn ordered(version: &str) -> (Vec<u64>, bool) {
        let version = version.trim().trim_start_matches(['v', 'V']);
        let version = version.split_once('+').map_or(version, |(core, _)| core);
        let (core, pre) = version
            .split_once('-')
            .map_or((version, None), |(core, pre)| (core, Some(pre)));
        let mut numbers: Vec<u64> = core
            .split('.')
            .map(|part| part.parse().unwrap_or(0))
            .collect();
        numbers.resize(numbers.len().max(3), 0);
        (numbers, pre.is_none())
    }
    ordered(latest) > ordered(local)
}

/// `--json`'s document, with `switch` as `"telemetry"` and `report`'s upload and history, which
/// are `null` while telemetry is off. Written by hand, as `nvs cache`'s is, so the keys keep the
/// order shown.
fn document(
    found: &Found,
    switch: &str,
    upload: Option<&str>,
    report: Option<&telemetry::Report>,
) -> String {
    let text = |value: Option<&str>| {
        value.map_or_else(
            || "null".to_owned(),
            |value| serde_json::Value::from(value).to_string(),
        )
    };
    let update = found.latest.as_deref().map_or("null", |latest| {
        if newer(latest, LOCAL) {
            "true"
        } else {
            "false"
        }
    });
    let sent = report
        .and_then(|report| report.outcome.as_ref())
        .map_or_else(
            || "null".to_owned(),
            |outcome| {
                format!(
                    r#"{{"result":"{}","message":{}}}"#,
                    outcome.result(),
                    text(Some(&outcome.line()))
                )
            },
        );
    let history = report.map(|report| &report.history);
    let number = |value: Option<u64>| value.map_or_else(|| "null".to_owned(), |n| n.to_string());
    format!(
        r#"{{"schema":1,"telemetry":"{switch}","telemetry_url":{},"local":{},"latest":{},"update_available":{update},"error":{},"upload":{sent},"last_upload":{},"next_upload":{},"uploads":{},"bytes_sent":{}}}"#,
        text(upload),
        text(Some(LOCAL)),
        text(found.latest.as_deref()),
        text(found.error.as_deref()),
        text(
            history
                .and_then(|history| history.last)
                .map(telemetry::stamp)
                .as_deref()
        ),
        text(
            history
                .map(|history| telemetry::stamp(history.next))
                .as_deref()
        ),
        number(history.map(|history| history.uploads)),
        number(history.map(|history| history.bytes)),
    )
}

#[cfg(test)]
mod tests {
    use std::io::Read;
    use std::net::TcpListener;
    use std::thread::JoinHandle;

    use super::*;

    /// A listener on loopback that answers one request with a Releases API document naming
    /// `tag`, and the URL to reach it. The thread gives back the request it read.
    fn releases(tag: &str) -> (String, JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
        let url = format!(
            "http://{}/repos/novis-lang/novis/releases/latest",
            listener.local_addr().expect("the bound address")
        );
        let body = format!(r#"{{"tag_name":"{tag}","name":"Novis {tag}","draft":false}}"#);
        let served = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("the update check connects");
            let mut request = Vec::new();
            let mut buffer = [0_u8; 1024];
            while !request.ends_with(b"\r\n\r\n") {
                let read = stream.read(&mut buffer).expect("the request");
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..read]);
            }
            let reply = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(reply.as_bytes()).expect("the reply");
            String::from_utf8_lossy(&request).into_owned()
        });
        (url, served)
    }

    /// What the command printed against `url`, with telemetry off.
    fn printed(url: &str, upload: Option<&str>, json: bool) -> String {
        let mut out = Vec::new();
        assert_eq!(
            check(url, false, upload, json, || None, &mut out),
            ExitCode::SUCCESS
        );
        String::from_utf8(out).expect("the command prints text")
    }

    #[test]
    fn a_newer_release_is_reported_as_an_update() {
        let (url, served) = releases("v99.0.0");
        let out = printed(&url, None, false);
        let request = served.join().expect("the listener thread");
        assert!(request.starts_with("GET /repos/novis-lang/novis/releases/latest "));
        assert!(
            request
                .to_ascii_lowercase()
                .contains(&format!("user-agent: nvs/{LOCAL}"))
        );
        assert_eq!(
            out,
            format!(
                "telemetry: off, and nothing is uploaded because `NVS_TELEMETRY_URL` is not set\n\
                 this version: {LOCAL}\n\
                 reading the latest release from {url}\n\
                 latest version: 99.0.0\n\
                 an update is available: {LOCAL} to 99.0.0\n"
            )
        );
    }

    #[test]
    fn the_same_release_is_reported_as_current_and_in_json() {
        let (url, served) = releases(&format!("v{LOCAL}"));
        let out = printed(&url, Some("http://collector.example/v1"), true);
        served.join().expect("the listener thread");
        assert_eq!(
            out,
            format!(
                "{{\"schema\":1,\"telemetry\":\"off\",\"telemetry_url\":\"http://collector.example/v1\",\
                 \"local\":\"{LOCAL}\",\"latest\":\"{LOCAL}\",\"update_available\":false,\"error\":null,\
                 \"upload\":null,\"last_upload\":null,\"next_upload\":null,\"uploads\":null,\"bytes_sent\":null}}\n"
            )
        );

        let (url, served) = releases(&format!("v{LOCAL}"));
        let out = printed(&url, None, false);
        served.join().expect("the listener thread");
        assert!(out.ends_with("you have the latest version\n"), "{out}");
    }

    #[test]
    fn an_unreachable_server_still_exits_zero_and_names_the_failure() {
        // A port that was bound and released, so nothing listens on it.
        let port = TcpListener::bind("127.0.0.1:0")
            .and_then(|listener| listener.local_addr())
            .expect("a loopback port")
            .port();
        let url = format!("http://127.0.0.1:{port}/latest");

        let out = printed(&url, None, false);
        let last = out.lines().last().unwrap_or_default();
        assert!(
            last.starts_with("could not read the latest release: "),
            "{out}"
        );

        let out = printed(&url, None, true);
        let document: serde_json::Value = serde_json::from_str(&out).expect("one JSON document");
        assert_eq!(document["schema"], 1);
        assert_eq!(document["latest"], serde_json::Value::Null);
        assert_eq!(document["update_available"], serde_json::Value::Null);
        assert!(document["error"].is_string(), "{out}");
    }

    #[test]
    fn with_telemetry_on_the_upload_and_its_history_follow_the_release() {
        let report = || {
            Some(telemetry::Report {
                outcome: Some(telemetry::Outcome::Kept("timeout after 3s".to_owned())),
                history: telemetry::History {
                    last: Some(1_761_436_800),
                    next: 1_762_041_600,
                    uploads: 2,
                    bytes: 400,
                },
            })
        };
        let (url, served) = releases(&format!("v{LOCAL}"));
        let mut out = Vec::new();
        let _ = check(
            &url,
            true,
            Some("http://collector.example/v1"),
            false,
            report,
            &mut out,
        );
        served.join().expect("the listener thread");
        let out = String::from_utf8(out).expect("text");
        assert!(
            out.ends_with(
                "you have the latest version\n\
                 telemetry: could not send (timeout after 3s); kept for next time\n\
                 last upload: 2025-10-26T00:00:00Z\n\
                 next upload: from 2025-11-02T00:00:00Z\n\
                 uploads accepted: 2, bytes sent: 400\n"
            ),
            "{out}"
        );

        let (url, served) = releases(&format!("v{LOCAL}"));
        let mut out = Vec::new();
        let _ = check(
            &url,
            true,
            Some("http://collector.example/v1"),
            true,
            report,
            &mut out,
        );
        served.join().expect("the listener thread");
        let document: serde_json::Value = serde_json::from_slice(&out).expect("one JSON document");
        assert_eq!(document["telemetry"], "on");
        assert_eq!(document["upload"]["result"], "kept");
        assert_eq!(document["last_upload"], "2025-10-26T00:00:00Z");
        assert_eq!(document["next_upload"], "2025-11-02T00:00:00Z");
        assert_eq!(document["uploads"], 2);
        assert_eq!(document["bytes_sent"], 400);
    }

    #[test]
    fn versions_compare_by_number_and_a_release_follows_its_pre_release() {
        assert!(newer("0.0.2", "0.0.1"));
        assert!(newer("0.10.0", "0.9.9"));
        assert!(newer("v1.0", "0.99.0"));
        assert!(newer("1.0.0", "1.0.0-rc.1"));
        assert!(!newer("1.0.0-rc.1", "1.0.0"));
        assert!(!newer("0.0.1", "0.0.1+abc"));
        assert!(!newer("0.0.1", "0.0.2"));
    }
}
