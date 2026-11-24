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
//!
//! The answer tests are its *the answer is the server's answer*: a forged
//! write, a body over `[limits] upload_total`, a `HEAD` and a failed request
//! answer as `nvs serve` does, and the call returns after the request's
//! after-response work.
//!
//! The clock tests are `rule:testing/task-tree-and-virtual-clock`'s *a sleep
//! is a virtual deadline*: under `#[Test(at:)]` a sleep ends at once, two
//! sleeping tasks wake in deadline order, a `Core\Task::all` deadline races a
//! sleep, and a memory cache entry's `ttl` runs on the same clock.
//!
//! The run tests are `rule:testing/a-test-runs-a-command`: `Core\Test::run`
//! reaches the command its words name, feeds stdin, gives `exit`, the usage
//! page and an uncaught error as exit codes, keeps stdout and stderr apart, and
//! is a `LogicError` in a folder with no entry `nvs run` would run.
//!
//! The env tests are `rule:testing/test-attribute`'s `env:`: the test's
//! variables reach the test, its requests and its runs, and no other test.

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

/// `form` reaches `Core\Request::formField()` under
/// `application/x-www-form-urlencoded`, with a space, a `+` and a `=` in it.
// covers: Core\Test::request
#[test]
fn a_form_option_reaches_request_post() {
    passes_in_bodies("formReachesRequestPost");
}

/// `multipart` sends a `Core\Http\Part` that `Core\Request::files()` reads
/// with its name, file name, media type and length, and a text field that
/// `Core\Request::formField()` reads.
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

/// Runs the one `#[Test]` method `method` of `answers/answers.nvs`, from a
/// scratch copy of that folder, so the file the after-response work writes
/// lands outside the source tree. Returns `(report, success)`.
fn run_in_answers(method: &str) -> (String, bool) {
    let dir = nvs_repo::scratch_private("reach-answers");
    for name in ["answers.nvs", "nvs.toml"] {
        std::fs::copy(fixture("answers").join(name), dir.join(name))
            .unwrap_or_else(|err| panic!("copying {name}: {err}"));
    }
    run_in(&dir, "answers.nvs", method)
}

/// Runs `method` of `answers.nvs` and asserts that it alone ran, and passed.
fn passes_in_answers(method: &str) {
    let (report, passed) = run_in_answers(method);
    assert!(passed, "`{method}` failed:\n{report}");
    assert!(
        report.contains(method) && report.contains("0 failed, 1 passed"),
        "`{method}` was the one test that ran, and it passed:\n{report}"
    );
}

/// A cross-origin write to a program that does not dispatch is answered `403`
/// with no body and no header, before the program runs.
// covers: Core\Test::request
#[test]
fn a_forged_write_to_a_program_without_dispatch_answers_403() {
    passes_in_answers("forgedWriteAnswers403");
}

/// A body over `[limits] upload_total` is answered `413` with no body, and
/// one under it reaches the program.
// covers: Core\Test::request
#[test]
fn an_oversized_body_answers_as_the_server_does() {
    passes_in_answers("oversizedBodyAnswers413");
}

/// A `HEAD` request is answered with its status and headers and no body.
// covers: Core\Test::request
#[test]
fn a_head_request_answers_with_no_body() {
    passes_in_answers("headAnswersWithNoBody");
}

/// A request that throws is answered `500` with no body, and
/// `Core\Test\Response::error()` names the error; it is `null` otherwise.
// covers: Core\Test::request
#[test]
fn a_failed_request_names_its_error() {
    passes_in_answers("failedRequestNamesItsError");
}

/// The file an after-response task writes is there when the call returns.
// covers: Core\Test::request
#[test]
fn a_request_returns_after_its_after_response_work() {
    passes_in_answers("requestReturnsAfterItsAfterResponseWork");
}

/// An after-response task that throws fails the test that sent the request,
/// and the failure names it.
// covers: Core\Test::request
#[test]
fn a_failed_after_response_task_fails_the_test() {
    let method = "failedAfterResponseTaskFailsTheTest";
    let (report, passed) = run_in_answers(method);
    assert!(!passed, "a failed after-response task passed:\n{report}");
    assert!(
        report.contains("1 failed, 0 passed"),
        "the one test ran and failed:\n{report}"
    );
    assert!(
        report.contains("an after-response task of the request failed")
            && report.contains("RuntimeError: the receipt was not sent"),
        "the failure names the after-response task and its error:\n{report}"
    );
}

/// Runs `method` of `clock.nvs` and asserts that it alone ran, and passed.
fn passes_in_clock(method: &str) {
    let (report, passed) = run_in(&fixture(""), "clock.nvs", method);
    assert!(passed, "`{method}` failed:\n{report}");
    assert!(
        report.contains(method) && report.contains("0 failed, 1 passed"),
        "`{method}` was the one test that ran, and it passed:\n{report}"
    );
}

/// Under `at:`, an hour's sleep ends at once and `Core\Time::now()` reads an
/// hour later.
// covers: Core\Time::sleep
#[test]
fn a_sleep_under_at_ends_at_once_and_moves_the_clock() {
    passes_in_clock("sleepEndsAtOnceAndMovesTheClock");
}

/// Two tasks of one `Core\Task::all` that sleep under `at:` wake in the order
/// of their deadlines, each at its own instant.
// covers: Core\Time::sleep
#[test]
fn two_sleeping_tasks_wake_in_deadline_order() {
    passes_in_clock("twoSleepingTasksWakeInDeadlineOrder");
}

/// A `Core\Task::all` deadline under `at:` ends a longer sleep with a
/// `TimeoutError` at the deadline's instant, and lets a shorter one finish.
// covers: Core\Task::all
#[test]
fn a_timeout_races_a_sleep_on_the_test_clock() {
    passes_in_clock("aTimeoutRacesASleep");
}

/// Without `at:`, a sleep waits in real time.
// covers: Core\Time::sleep
#[test]
fn a_sleep_without_at_waits_in_real_time() {
    passes_in_clock("sleepWithoutAtWaits");
}

/// A memory cache entry's `ttl` runs on the test clock: `Core\Test::advance`
/// and a sleep under `at:` both end it.
// covers: Core\Cache::memory
#[test]
fn a_cache_entry_expires_with_advance() {
    passes_in_clock("cacheEntryExpiresWithAdvance");
}

/// Runs `method` of `run.nvs` and asserts that it alone ran, and passed.
fn passes_in_run(method: &str) {
    let (report, passed) = run_in(&fixture(""), "run.nvs", method);
    assert!(passed, "`{method}` failed:\n{report}");
    assert!(
        report.contains(method) && report.contains("0 failed, 1 passed"),
        "`{method}` was the one test that ran, and it passed:\n{report}"
    );
}

/// `Core\Test::run(['greet', 'Ada'])` reaches the `greet` command with `Ada`
/// as its argument, and exits `0` with its output on stdout.
// covers: Core\Test::run
#[test]
fn test_run_reaches_the_command_its_words_name() {
    passes_in_run("reachesTheCommandItsWordsName");
}

/// The `stdin` option is what `Core\IO::stdin()` reads in the run, and a run
/// without it reads an empty input.
// covers: Core\Test::run
#[test]
fn test_run_feeds_stdin() {
    passes_in_run("feedsStdin");
}

/// `exit(7)` in the run ends it with the exit code `7`, keeping what it wrote.
// covers: Core\Test::run
#[test]
fn test_run_gives_the_exit_code_of_exit() {
    passes_in_run("givesTheExitCodeOfExit");
}

/// A command line missing an argument exits `2` with the usage page on stderr.
// covers: Core\Test::run
#[test]
fn test_run_gives_the_usage_page_on_stderr_and_exit_2() {
    passes_in_run("givesTheUsagePageOnStderr");
}

/// An uncaught error exits `1` with its message on stderr.
// covers: Core\Test::run
#[test]
fn test_run_gives_an_uncaught_error_as_its_exit_code() {
    passes_in_run("givesAnUncaughtErrorAsItsExitCode");
}

/// What the run writes to stdout and to stderr is read back apart.
// covers: Core\Test::run
#[test]
fn test_run_keeps_stdout_and_stderr_apart() {
    passes_in_run("keepsStdoutAndStderrApart");
}

/// A folder of programs has no entry `nvs run` would run, so
/// `Core\Test::run` there throws a `LogicError`.
// covers: Core\Test::run
#[test]
fn test_run_without_a_command_entry_is_a_logic_error() {
    let method = "runIsALogicError";
    let (report, passed) = run_in(&fixture(""), "run_folder", method);
    assert!(passed, "`{method}` failed:\n{report}");
    assert!(
        report.contains(method) && report.contains("0 failed, 1 passed"),
        "`{method}` was the one test that ran, and it passed:\n{report}"
    );
}

/// `#[Test(env:)]`'s variable is what `Core\Env::get` reads in the test, in a
/// request it makes and in a run it makes.
// covers: Core\Test, lang:testing/test-options
#[test]
fn the_env_option_reaches_the_test_and_its_requests() {
    passes_in_records("envReachesTheTestAndItsRequests");
}

/// Runs the one `#[Test]` method `method` of `records/records.nvs` and asserts
/// that it alone ran, and passed.
fn passes_in_records(method: &str) {
    let (report, passed) = run_in(&fixture("records"), "records.nvs", method);
    assert!(passed, "`{method}` failed:\n{report}");
    assert!(
        report.contains(method) && report.contains("0 failed, 1 passed"),
        "`{method}` was the one test that ran, and it passed:\n{report}"
    );
}

/// A test without `env:`, run in the same suite as one with it, reads none of
/// its variables, and neither do its requests and runs.
// covers: Core\Test, lang:testing/test-options
#[test]
fn the_env_option_does_not_reach_another_test() {
    let (report, passed) = run_in(&fixture("records"), "records.nvs", "env");
    assert!(passed, "the env tests failed:\n{report}");
    assert!(
        report.contains("envReachesTheTestAndItsRequests")
            && report.contains("envStaysWithItsTest")
            && report.contains("0 failed, 2 passed"),
        "both env tests ran in one suite, and passed:\n{report}"
    );
}

/// `Core\Test::logs` returns the records the test, a request it made and a run
/// it made wrote, in that order, with their levels, messages and fields.
// covers: Core\Test::logs
#[test]
fn logs_reads_the_records_of_the_test_and_its_requests() {
    passes_in_records("logsReadsTheTestAndItsRequests");
}

/// A test run after one that wrote three records reads none of them.
// covers: Core\Test::logs
#[test]
fn logs_is_empty_in_a_new_test() {
    let (report, passed) = run_in(&fixture("records"), "records.nvs", "logs");
    assert!(passed, "the logs tests failed:\n{report}");
    assert!(
        report.contains("logsReadsTheTestAndItsRequests")
            && report.contains("logsStartsEmpty")
            && report.contains("0 failed, 2 passed"),
        "both logs tests ran in one suite, and passed:\n{report}"
    );
}

/// `Core\Test::sentMail` returns the message a request sent, with every field
/// `Core\Mail::send` was given.
// covers: Core\Test::sentMail
#[test]
fn sent_mail_reads_a_message_a_request_sent() {
    passes_in_records("mailReadsWhatARequestSent");
}

/// A send under `nvs test` returns without a connection: the relay's port has
/// no listener, so a real send would throw and fail the test.
// covers: Core\Test::sentMail, Core\Mail::send
#[test]
fn mail_under_nvs_test_opens_no_connection() {
    passes_in_records("mailOpensNoConnection");
}
