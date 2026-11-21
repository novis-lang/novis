---
looper: 1
title: "a test can set and read everything a served request, a command and the clock give a program"
summary: "Reached when Core\\Test::request can state the client address, the scheme, cookies and every body Core\\Http\\Client can send, answers a forged write, an oversized body, a HEAD and a failure the way the server does and waits for after-response work; Core\\Test::run runs a command program with its arguments, input and environment and returns its exit code and output; a sleep under at: follows the test clock; #[Test(env:)] sets environment variables; Core\\Test::logs and Core\\Test::sentMail read what a test caused; and each has its feature proofs and website entry."
context: [crates/nvs-stdlib/src/test.rs, crates/nvs-cli/src/runner.rs, docs/rules/testing/in-process-request.md, docs/rules/testing/task-tree-and-virtual-clock.md, docs/rules/testing/test-attribute.md]
stages:
  - id: 1
    title: the rules say what a test can set and read, and the virtual clock rule matches the code it will have
    status: done
    context: [docs/rules/testing/determinism-declared-on-the-test.md, docs/rules/testing/an-outbound-call-is-answered-from-a-table.md, docs/rules/core-classes/process-run.md]
  - id: 2
    title: a test request states its client address, scheme, cookies and host
    status: done
    context: [crates/nvs-runtime/src/ctx/inbound.rs, crates/nvs-server/src/trace.rs]
  - id: 3
    title: a test request sends every body Core\Http\Client can send
    status: done
    context: [crates/nvs-stdlib/src/http.rs, crates/nvs-runtime/src/ctx/inbound.rs]
  - id: 4
    title: a test request is answered the way the server answers it
    status: done
    context: [crates/nvs-cli/src/serve.rs, crates/nvs-host/src/isolate.rs, crates/nvs-runtime/src/host.rs]
  - id: 5
    title: a sleep and a timer under at follow the test clock
    status: done
    context: [crates/nvs-runtime/src/peer.rs, crates/nvs-host/src/timer.rs, crates/nvs-host/src/scheduler.rs, crates/nvs-stdlib/src/time.rs]
  - id: 6
    title: Core\Test::run runs a command program and returns its exit code and output
    status: done
    context: [crates/nvs-stdlib/src/command.rs, crates/nvs-stdlib/src/cli.rs, crates/nvs-stdlib/src/io.rs, crates/nvs-cli/src/main.rs]
  - id: 7
    title: "#[Test(env:)], Core\\Test::logs and Core\\Test::sentMail"
    status: done
    context: [crates/nvs-runtime/src/capability.rs, crates/nvs-stdlib/src/log.rs, crates/nvs-stdlib/src/mail.rs]
  - id: 8
    title: the feature proofs and the help of every new member and option
    status: done
    context: [docs/examples/core/Test/request/about.md]
  - id: 9
    title: the website shows what a test can reach, and lists the change
    status: done
  - id: 10
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: rules
    stage: 1
    name: "the rulebook validates with the amended testing rules"
    run: [bun, nv, rules, --check]
    timeout: 900
  - id: peer
    stage: 2
    name: "a test request carries the client address, the scheme, its cookies, the mount row its host picks, and the trace it names"
    run: [cargo, test, --test, test_reach, --, a_test_request_has_the_client_address_it_states, a_test_request_over_https_reads_a_host_prefixed_cookie, a_test_request_with_no_peer_has_a_null_client_address, a_bad_client_address_is_a_logic_error, the_cookies_option_reaches_the_cookie_readers, the_host_header_picks_the_mount_row, a_traceparent_header_is_continued]
    timeout: 1800
    tests:
      - a_test_request_has_the_client_address_it_states
      - a_test_request_over_https_reads_a_host_prefixed_cookie
      - a_test_request_with_no_peer_has_a_null_client_address
      - a_bad_client_address_is_a_logic_error
      - the_cookies_option_reaches_the_cookie_readers
      - the_host_header_picks_the_mount_row
      - a_traceparent_header_is_continued
  - id: bodies
    stage: 3
    name: "json, form and multipart reach the request readers with the content type the client would send, and two bodies do not compile"
    run: [cargo, test, --test, test_reach, --, a_json_option_reaches_request_json, a_form_option_reaches_request_post, a_multipart_option_reaches_request_files, a_body_with_a_content_type_is_sent_as_written, two_body_options_do_not_compile]
    timeout: 1800
    tests:
      - a_json_option_reaches_request_json
      - a_form_option_reaches_request_post
      - a_multipart_option_reaches_request_files
      - a_body_with_a_content_type_is_sent_as_written
      - two_body_options_do_not_compile
  - id: answers
    stage: 4
    name: "a forged write, an oversized body, a HEAD and a failure answer as the server does, and after-response work has ended when the call returns"
    run: [cargo, test, --test, test_reach, --, a_forged_write_to_a_program_without_dispatch_answers_403, an_oversized_body_answers_as_the_server_does, a_head_request_answers_with_no_body, a_failed_request_names_its_error, a_request_returns_after_its_after_response_work, a_failed_after_response_task_fails_the_test]
    timeout: 1800
    tests:
      - a_forged_write_to_a_program_without_dispatch_answers_403
      - an_oversized_body_answers_as_the_server_does
      - a_head_request_answers_with_no_body
      - a_failed_request_names_its_error
      - a_request_returns_after_its_after_response_work
      - a_failed_after_response_task_fails_the_test
  - id: clock
    stage: 5
    name: "under at:, a sleep ends at once and moves the clock, two sleeping tasks wake in deadline order, a timeout races a sleep on the test clock, and a cache entry expires with advance"
    run: [cargo, test, --test, test_reach, --, a_sleep_under_at_ends_at_once_and_moves_the_clock, two_sleeping_tasks_wake_in_deadline_order, a_timeout_races_a_sleep_on_the_test_clock, a_sleep_without_at_waits_in_real_time, a_cache_entry_expires_with_advance]
    timeout: 1800
    tests:
      - a_sleep_under_at_ends_at_once_and_moves_the_clock
      - two_sleeping_tasks_wake_in_deadline_order
      - a_timeout_races_a_sleep_on_the_test_clock
      - a_sleep_without_at_waits_in_real_time
      - a_cache_entry_expires_with_advance
  - id: run
    stage: 6
    name: "Core\\Test::run reaches a command with its words, reads stdin, gives exit, the usage page and an uncaught error as exit codes, and captures stdout and stderr apart"
    run: [cargo, test, --test, test_reach, --, test_run_reaches_the_command_its_words_name, test_run_feeds_stdin, test_run_gives_the_exit_code_of_exit, test_run_gives_the_usage_page_on_stderr_and_exit_2, test_run_gives_an_uncaught_error_as_its_exit_code, test_run_keeps_stdout_and_stderr_apart, test_run_without_a_command_entry_is_a_logic_error]
    timeout: 1800
    tests:
      - test_run_reaches_the_command_its_words_name
      - test_run_feeds_stdin
      - test_run_gives_the_exit_code_of_exit
      - test_run_gives_the_usage_page_on_stderr_and_exit_2
      - test_run_gives_an_uncaught_error_as_its_exit_code
      - test_run_keeps_stdout_and_stderr_apart
      - test_run_without_a_command_entry_is_a_logic_error
  - id: records
    stage: 7
    name: "env: reaches Core\\Env in the test, its requests and its runs and no other test; logs reads every record a test caused; mail is kept and never sent"
    run: [cargo, test, --test, test_reach, --, the_env_option_reaches_the_test_and_its_requests, the_env_option_does_not_reach_another_test, logs_reads_the_records_of_the_test_and_its_requests, logs_is_empty_in_a_new_test, sent_mail_reads_a_message_a_request_sent, mail_under_nvs_test_opens_no_connection]
    timeout: 1800
    tests:
      - the_env_option_reaches_the_test_and_its_requests
      - the_env_option_does_not_reach_another_test
      - logs_reads_the_records_of_the_test_and_its_requests
      - logs_is_empty_in_a_new_test
      - sent_mail_reads_a_message_a_request_sent
      - mail_under_nvs_test_opens_no_connection
  - id: proofs-request
    stage: 8
    name: "Core\\Test::request has every feature proof"
    run: [bun, nv, proofs, --only, "Core\\Test::request", --gate]
    timeout: 1800
  - id: proofs-run
    stage: 8
    name: "Core\\Test::run has every feature proof"
    run: [bun, nv, proofs, --only, "Core\\Test::run", --gate]
    timeout: 1800
  - id: proofs-logs
    stage: 8
    name: "Core\\Test::logs has every feature proof"
    run: [bun, nv, proofs, --only, "Core\\Test::logs", --gate]
    timeout: 1800
  - id: proofs-mail
    stage: 8
    name: "Core\\Test::sentMail has every feature proof"
    run: [bun, nv, proofs, --only, "Core\\Test::sentMail", --gate]
    timeout: 1800
  - id: site-stale
    stage: 9
    name: "every page this goal reaches is reread and stamped"
    run: [bun, nv, site, --check, stale]
    timeout: 900
  - id: site-changes
    stage: 9
    name: "every member and option this goal adds or changes has its changes.json entry"
    run: [bun, nv, site, --check, changes]
    timeout: 900
  - id: site-review
    stage: 10
    name: "every user page this goal changed carries its newcomer review stamp"
    run: [bun, nv, site, --check, review]
    timeout: 900
---

# a test can set and read everything a served request, a command and the clock give a program

## Why

A user reported that `Core\Request::clientIp()` is `null` in every request `Core\Test::request`
sends, so no test can check code that depends on the client address. A search of the test runner
for the same kind of gap found these, each read in the code:

- **The request a test sends.** The bag has `headers`, `body`, `mount` and `captures` alone
  (`crates/nvs-stdlib/src/test.rs:551`). `described` (`crates/nvs-stdlib/src/test.rs:2926`) never
  calls `InboundSpec::set_peer`, so the client address is `null` and the scheme is `http`. A `__Host-`
  or `__Secure-` cookie is then invisible (`crates/nvs-runtime/src/ctx/inbound.rs:1004`), so a test
  cannot send a prefixed session or CSRF cookie. A `.nvt` case can state both
  (`crates/nvs-cli/src/main.rs:2079`); a `#[Test]` method cannot. The host does not pick a mount row
  (`links_through`, `crates/nvs-stdlib/src/test.rs:2977`), and a `traceparent` header is not taken
  (`crates/nvs-server/src/trace.rs:125` is called on the wire only). A JSON, form or multipart body
  must be framed by hand, with its own `content-type`.
- **The answer a test reads.** A forged write to a program that does not call `Core\Router::dispatch`
  gets `403` from the server (`crates/nvs-cli/src/serve.rs:1355`) and reaches the handler in a test
  (`refuse_a_forged_write`, `crates/nvs-cli/src/runner.rs:4161`). No body limit is checked
  (`crates/nvs-cli/src/serve.rs:1381`). A failed request is a bare `500`
  (`crates/nvs-stdlib/src/test.rs:3090`), so a test cannot see which error ended it.
  `Core\Task::afterResponse` work runs after the call returns, detached
  (`crates/nvs-host/src/isolate.rs:1263`), so a test cannot wait for it.
- **Command programs.** Only the launcher sets the command line (`crates/nvs-cli/src/main.rs:2464`),
  and stdin is always empty in a test (`crates/nvs-stdlib/src/io.rs:1342`). `Core\Command::run()`
  in a test sees no words and prints its usage page. `exit(n)` ends the whole run
  (`crates/nvs-cli/src/runner.rs:580`). A test cannot run a command and check its exit code or
  output.
- **The clock.** `rule:testing/task-tree-and-virtual-clock` says a sleep under the test clock ends at
  once. The code waits in real time: `Core\Time::sleep` (`crates/nvs-stdlib/src/time.rs:4449`) calls
  `nvs_runtime::peer::sleep` (`crates/nvs-runtime/src/peer.rs:605`), which never reads the fixed
  clock. No runner fixture sleeps under `at:`, so nothing caught it.
- **What a test caused.** `Core\Env` reads the process environment
  (`crates/nvs-runtime/src/capability.rs:1826`) and no test can set it. No test can read a log
  record (`crates/nvs-stdlib/src/log.rs:229`). `Core\Mail::send`
  (`crates/nvs-stdlib/src/mail.rs:1173`) opens a real SMTP connection from a test, so a test either
  sends real mail or cannot test mail at all.

Security and request isolation is the first priority, and most of these gaps are on security paths:
rate limits and allowlists by client address, `__Host-` cookies, CSRF and body limits. A test that
cannot reach them leaves them untested.

## Standing decisions

The user asked for the gaps to be closed. Every design below is the authoring session's call, made by
the priority ordering. A session builds on it and does not reopen it.

**The peer is stated, never derived.** `Core\Test::request` gains `clientIp?: string` and
`scheme?: string`. They are what the `[server] trusted_proxies` walk would have settled on, as a
`.nvt` case's `--CLIENT_IP--` and `--SCHEME--` are. No test reads `X-Forwarded-For`. `clientIp` that
is not one IPv4 or IPv6 address, and `scheme` that is not `"http"` or `"https"`, is a `LogicError`.
Omitted, the request has no address and is `http`, as today. Cost: none per request.

**`cookies?: array<string>`** sends the pairs as one `cookie` field, through
`InboundSpec::push_cookie` (`crates/nvs-runtime/src/ctx/inbound.rs:1586`), as `--COOKIE--` does.
No cookie jar: each call is one request, and a test copies `Response::cookies()` into the next call
itself.

**The host picks the mount row.** A `host` header given in `headers` goes to `InboundSpec::set_host`
and to `links_through`, which picks the row the server would pick for that host. No new option.

**A `traceparent` header is taken.** `UnderTest::answer` (`crates/nvs-cli/src/runner.rs:1619`) calls
`nvs_server::trace::take` as the wire does.

**Bodies use `Core\Http\Client`'s options and its walk.** `json`, `form`, `multipart` and
`contentType` join `body`, read by the same code as `Core\Http\Client`'s options
(`crates/nvs-stdlib/src/http.rs:2371`), and built with `SpecBody::Json`, `Form` and `Files`. Only one
of `json`, `form`, `body` and `multipart` may be given, and `contentType` needs `body`. Both mistakes
are compile errors, as they are for the client. A test then writes a request the way a program writes
one.

**The answer is the server's answer.** Each refusal the wire makes before the program runs is made
from the same function in a test:
- A forged write to a program without `Core\Router::dispatch` answers `403` with no body, as
  `Reply::forbidden()` does. A dispatching program keeps its gate's `refused`.
- A body over the run's `[limits]` answers what the wire answers for it, read at
  `crates/nvs-cli/src/serve.rs:1381`.
- A `HEAD` request answers with no body, as the wire does. Stage 4 reads where the server strips it
  and calls the same code.

**`Core\Test\Response::error(): ?string`** is the class and message of the error that ended a failed
request, as `Class: message`. It is `null` for a request that did not fail. The status, the body and
the headers stay what the server sends, `500` with none. It reads `Completion.error`
(`crates/nvs-runtime/src/host.rs:674`). A string, because the error object belongs to the request's
isolate and does not cross into the test's.

**`Core\Test::request` returns after the request's after-response work has ended.** The response is
the same, and a test then reads what that work wrote. An after-response task that fails fails the
test, named as such, as a task left running does
(`rule:testing/task-tree-and-virtual-clock`). On the server, after-response work stays detached.

**The clock rule becomes true.** Under `#[Test(at:)]`, a sleep or a timer of the test's task tree
parks on a virtual deadline. When no task of the tree can run, the clock moves to the earliest
deadline and wakes its tasks. So a sleep ends at once, two tasks wake in deadline order, and a
timeout races a sleep the way it would in real time. A request or a run a test makes has its own
copy of the clock and its own deadlines. Without `at:`, a sleep waits in real time, as today. Expiries
that Novis computes itself read the same clock: the cache's TTL (`crates/nvs-stdlib/src/cache.rs:855`,
`:1036`) and a queue job's delay (`crates/nvs-stdlib/src/queue.rs:3013`). A TTL kept by Redis or a
database, a lock wait and an I/O timeout stay in real time: the other side of them is a real program.
Cost: one check of the fixed clock per sleep and per expiry read, nothing else.

**`Core\Test::run(array<string> $arguments, {stdin?: string|bytes}): Core\Test\RunResult`** runs
the program's entry, the file `nvs run` runs, in a child isolate as `Core\Test::request` runs a
request. `$arguments` is the command line after the program name. `Core\Test\RunResult` has
`exitCode(): int`, `stdout(): string`, `stderr(): string` and `bytes(): bytes` for stdout, the names
`ProcessResult` uses (`rule:core-classes/process-run`). `exit(n)` in the child ends the child, and
`n` is its exit code. An uncaught error is the exit code `nvs run` gives for one, with its message
on stderr. A unit with no entry `nvs run` would run is a `LogicError`. No `cwd` option: a program
reads no working directory but its grants. A run from inside a run or a request is refused, as a
request from inside a request is (`rule:testing/in-process-request`).

**`#[Test(env: array<string>)]`** sets environment variables for that test. `Core\Env` reads them
first, then the process environment. They reach the test, every request and run it makes, and no
other test. The `env` grant is still needed, as it is in production. One option on the test, not one
per call: a program reads its environment at any point, and two values in one test would test
nothing a real deployment has.

**`Core\Test::logs(): array<Core\Test\LogRecord>`** returns every record written since the test
started, by the test, its requests and its runs, in the order written. `Core\Test\LogRecord` has
`level(): string`, `message(): string` and `fields(): array<mixed>`. Records still reach their
`[log] target`. Cost: the records of one test, freed with it.

**Under `nvs test`, `Core\Mail` sends nothing.** `Core\Mail::send` keeps the message, and
`Core\Test::sentMail(): array<Core\Test\SentMail>` returns those sent since the test started, by the
test, its requests and its runs. `Core\Test\SentMail`'s members are the fields `Core\Mail::send`
takes, read back. This is the same reason outbound HTTP is answered from a table in a test: a test
never reaches a real person.

**Declined.**
- A cookie jar kept between `Core\Test::request` calls. Hidden state between two calls, and the copy
  is one line.
- Repeated header lines. Joining the values gives the same request, and a second `Host` line is a
  request the server refuses.
- Server-added headers (`[http.headers]`, CORS, the CSP nonce) in `Core\Test\Response`, WebSocket and
  SSE upgrades, chunk boundaries and `Range` in a test request. `#[Test(server: true)]` serves the
  program on a real listener for those, as `rule:testing/in-process-request` says.
- A per-test configuration file. Declined in `rule:testing` already: the configuration is per server.
- A fake for `Core\Process`. A program that runs a command wraps it in its own class, and
  `Core\Test::double` replaces that class.
- Running a `[[schedule]]` row from a test. `spawn script` runs the script it names.
- An expected-to-fail mark for a whole test, a file snapshot and an exact-name filter. `skip:` with a
  reason, inline snapshots and `--filter` cover them.

**The two new rules are `designed` until their stage ships them.** `a-test-runs-a-command` and
`a-test-reads-its-logs-and-mail` carry `"goal": "tests-reach-everything"`, so the rulebook marks them
as not yet built. Stage 6 and stage 7 set each to `shipped` and fill its `guardedBy`. The three
amended rules stay `shipped`, because most of each is built already.

**Lanes.** Stages 2, 3 and 4 share `crates/nvs-stdlib/src/test.rs` and run in order. Stage 5
(`peer.rs`, `nvs-host`) and stage 6 (`command.rs`, `cli.rs`, `io.rs`, `runner.rs`'s run path) touch
disjoint files and could run as side goals, but stage 6's `Core\Test::run` shares `test.rs` with
stage 7. One goal, run in order.

**The deadline queue is `nvs_runtime::test_clock`, and an idle core moves every clock.** The
clock is one `Rc<TestClock>` on `Ctx`, shared by `Ctx::child`, so a task's sleep moves the clock
its tree reads. A sleep files its waker under `(deadline, order filed)` and parks with no real bound.
`Scheduler::run` calls `test_clock::advance_idle` when its run queue is empty, which moves each clock
with a deadline to its earliest one. That is "no task of the tree can run" read per core: a clock
moves only when nothing on the core can run, which is the same answer for a tree and costs one
length read where no test runs. `Core\Task::all`'s deadline is filed on the same clock, and
`Core\Test::advance` wakes every deadline it passes. A cache entry a test writes ends on its test
clock, and a queue job's creation time and default due time read the request's wall clock.

**What stops it.** One attempt per item and two sessions per stage. A stage that does not close in
two sessions writes what holds it in the handoff, and the next session splits it.

## Stages

### 1. the rules say what a test can set and read, and the virtual clock rule matches the code it will have

- [x] `docs/rules/testing/in-process-request.md:14`: the signature with `clientIp`, `scheme`,
  `cookies`, `json`, `form`, `multipart` and `contentType`; the host picks the mount row; a
  `traceparent` is taken; the refusals the wire makes; `error()`; the call returns after
  after-response work. Declined: a cookie jar, repeated header lines, server-added headers and
  upgrades.
- [x] `docs/rules/testing/task-tree-and-virtual-clock.md:8`: the virtual deadlines, the order of
  wakes, a copy per request and run, the expiries that follow it and those that stay real.
- [x] `docs/rules/testing/a-test-runs-a-command.md:1` and its `data/rules/testing/` entry: the new
  rule for `Core\Test::run`, from § *Standing decisions*.
- [x] `docs/rules/testing/test-attribute.md:34`: `env:` in the option list, with its reach.
- [x] `docs/rules/testing/a-test-reads-its-logs-and-mail.md:1` and its `data/rules/testing/` entry:
  `Core\Test::logs` and `Core\Test::sentMail`, and that mail is never sent under `nvs test`.
  Declined: a `Core\Process` fake and running a schedule row.
- [x] `bun nv rules --render`, then the `rules` check.

### 2. a test request states its client address, scheme, cookies and host

- [x] `crates/nvs-stdlib/src/test.rs:551`: `clientIp`, `scheme` and `cookies` in `REQUEST_OPTIONS`,
  and the card's text (AGENTS.md § *Text an end user reads*).
- [x] `crates/nvs-stdlib/src/test.rs:2926`: `described` calls `set_peer`, `push_cookie` and
  `set_host`, and throws the two `LogicError`s.
- [x] `crates/nvs-stdlib/src/test.rs:2977`: `links_through` takes the host.
- [x] `crates/nvs-cli/src/runner.rs:1619`: `UnderTest::answer` calls `nvs_server::trace::take`.
- [x] `crates/nvs-cli/tests/test_reach.rs:1` with fixtures under
  `crates/nvs-cli/tests/fixtures/reach/`: the seven `peer` tests.

### 3. a test request sends every body Core\Http\Client can send

- [x] `crates/nvs-stdlib/src/http.rs:2371`: the body options' walk callable from `test.rs`, and the
  compile error for two bodies or `contentType` alone applied to `Core\Test::request`'s bag.
- [x] `crates/nvs-stdlib/src/test.rs:2940`: `json`, `form` and `multipart` built as `SpecBody::Json`,
  `Form` and `Files`, and `contentType` with `body`.
- [x] `crates/nvs-cli/tests/test_reach.rs`: the five `bodies` tests.

### 4. a test request is answered the way the server answers it

- [x] `crates/nvs-cli/src/runner.rs:4156`: `refuse_a_forged_write` answers `403` for a program that
  does not dispatch, and `UnderTest::answer` returns that answer.
- [x] `crates/nvs-cli/src/runner.rs:1619`: the body limit from `crates/nvs-cli/src/serve.rs:1381`,
  and the `HEAD` body removed where the server removes it.
- [x] `crates/nvs-stdlib/src/test.rs:3090`: `Core\Test\Response::error()`, its card and its
  `RESPONSE` field.
- [x] `crates/nvs-host/src/isolate.rs:1263`: a request `Core\Test::request` made is joined after its
  after-response work, and a failure there fails the test.
- [x] `crates/nvs-cli/tests/test_reach.rs`: the six `answers` tests.

### 5. a sleep and a timer under at follow the test clock

- [x] `crates/nvs-cli/tests/test_reach.rs`: `a_sleep_under_at_ends_at_once_and_moves_the_clock`
  first, run once by name, red, so the gap is pinned before the fix.
- [x] `crates/nvs-host/src/timer.rs:305` and `crates/nvs-host/src/scheduler.rs`: a virtual deadline
  queue per test tree, and the clock moves to the earliest one when no task of the tree can run.
- [x] `crates/nvs-runtime/src/peer.rs:605`: a sleep with a fixed clock parks on a virtual deadline.
- [x] `crates/nvs-stdlib/src/cache.rs:855`, `:1036` and `crates/nvs-stdlib/src/queue.rs:3013`: read
  the clock seam (`crates/nvs-stdlib/src/time.rs:4358`).
- [x] `crates/nvs-cli/tests/test_reach.rs`: the other four `clock` tests.

### 6. Core\Test::run runs a command program and returns its exit code and output

**Split:** the `Core\Test\RunResult` class and its cards go to one writing subagent once the child's
outcome type exists; the session writes the run path.

- [x] `crates/nvs-stdlib/src/test.rs:743`: `Core\Test::run` in the member list, and
  `Core\Test\RunResult` with its four members.
- [x] `crates/nvs-cli/src/runner.rs:1619`: a run path beside `UnderTest::answer` that sets the
  child's command line (`ctx.set_command_line`, as `crates/nvs-cli/src/main.rs:2464` does), its stdin
  and two capture buffers, and turns `exit(n)` (`crates/nvs-cli/src/runner.rs:580`) into the exit
  code.
- [x] `crates/nvs-stdlib/src/io.rs:1342`: stdin reads the run's input in a child of `Core\Test::run`,
  and end of input everywhere else in a test.
- [x] `crates/nvs-cli/tests/test_reach.rs`: the seven `run` tests.
  - [x] the fixtures `crates/nvs-cli/tests/fixtures/reach/run.nvs` and `run_folder/checks.nvs`
  - [x] `run.nvs` compiles: its last lines pass a `uint` to `exit`
  - [x] the seven Rust tests, each run once by name

### 7. #[Test(env:)], Core\Test::logs and Core\Test::sentMail

- [x] `crates/nvs-cli/src/runner.rs:4659`: `env:` read with the other options, checked as
  `array<string>`, and carried to the test's isolate and every child.
- [x] `crates/nvs-runtime/src/capability.rs:1826`: `env_var` and `env_vars` read the test's
  variables first.
- [x] `crates/nvs-stdlib/src/log.rs:229`: a record is kept for `Core\Test::logs` in a test and its
  children, and `Core\Test\LogRecord` with its three members.
- [x] `crates/nvs-stdlib/src/mail.rs:1173`: under `nvs test`, `send` keeps the message and opens no
  connection, and `Core\Test::sentMail` with `Core\Test\SentMail`.
- [x] `crates/nvs-cli/tests/test_reach.rs`: the six `records` tests.
  - [x] the two `env` tests, in `crates/nvs-cli/tests/fixtures/reach/records.nvs`
  - [x] the two `logs` tests
  - [x] the two `mail` tests
  - [x] `a-test-reads-its-logs-and-mail` set to `shipped`, and the `records` check run by name

### 8. the feature proofs and the help of every new member and option

**Split:** the examples and attacks for `Core\Test::run`, `Core\Test::logs` and
`Core\Test::sentMail` go to three writing subagents, side by side. The session writes the
`Core\Test::request` ones.

- [x] `docs/examples/core/Test/request/about.md:1`: the new options, the server's answers and
  `error()`, and examples `05` onward: a rate limit by client address, a `__Host-` cookie over
  `https`, a JSON post, a file upload, and a forged write answered `403`.
- [x] `docs/examples/core/Test/run/about.md:1` and the examples and attack
  `bun nv proofs --id 'Core\Test::run'` names: a command with a flag, a command that reads stdin, a
  usage error with exit code `2`.
- [x] `docs/examples/core/Test/logs/about.md:1` and `docs/examples/core/Test/sentMail/about.md:1`,
  with the examples, benches and attacks their `bun nv proofs --id` names. The `sentMail` attack: a
  test tries to send real mail, and no connection opens.
- [x] `docs/examples/core/Test/advance/about.md`: a retry with backoff tested in microseconds.
- [x] `bun nv proofs --comments` over every new `.nvs`, then the four `proofs` checks.

### 9. the website shows what a test can reach, and lists the change

- [x] The testing guide pages `bun nv brief --where "Core\\Test"` names under
  `website/src/content/docs/`: test a command program, the client address and scheme, the request
  body options, logs and mail, and the clock.
- [x] `website/src/data/changes.json`: `new` `Core\Test::run`, `Core\Test::logs`,
  `Core\Test::sentMail`, `Core\Test\Response::error`, `#[Test(env:)]`; `changed`
  `Core\Test::request` (options and answers), `Core\Mail::send` (sends nothing under `nvs test`),
  `Core\Time::sleep` (follows the test clock under `at:`).
- [x] Reread and stamp every page `bun nv site --check stale` names.

### 10. the goal is verified once, and every failure is fixed

- [ ] The newcomer review of every page this goal changed ([prose.md](../agent/prose.md) § *A
  newcomer reads it*).
- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Stages 1 to 9 are done, and every code check (`peer` to `records`), `site-stale` and
`site-changes` are green. What is left is the close (stage 10).

Stage 9 is `47c0f33d16` and `6c85af66be`: a new `guides/testing/test-a-command.mdx`, the other
testing guides extended, eight `changes.json` entries, 45 texts stamped and the moved pages' search
questions rewritten. `#[Test(env:)]` has no roster id, so its entry is `changed`
`lang:testing/test-options`, which the env tests in `test_reach.rs` now cover. Stage 8's texts have
their newcomer review; what stage 9 wrote does not yet.

At the gate, these may go red: a test whose `Core\Task::all` child relied on the real clock or
timed a sleep under `at:`; a test that lists `#[Test]`'s options (`env` is new); a test that lists
`Core\Test`'s members or the `Core` class roster (`logs`, `sentMail`, `LogRecord`, `SentMail` are
new; `run`, `logs` and `sentMail` joined `asserting_members`' exclusions in `test.rs`); and any
existing test that calls `Core\Mail::send` under `nvs test` expecting a connection or its error.
Nightly 44894790665's Miri timeout predates `156f11a7ca`, which fixed it.

### Next

Stage 10, the close. First the newcomer review of what stage 9 wrote: the four pages under
`website/src/content/docs/guides/testing/`, `syntax/testing/writing-tests.mdx`, the nine about.md
files and the two help cards (`crates/nvs-stdlib/src/time.rs` `TIME_SLEEP_DOC`,
`crates/nvs-stdlib/src/mail.rs` `SEND_DOC`) amended in `47c0f33d16`. Take the keys from
`bun nv site --review`, `--brief` per batch, one `reader` each, then `--stamp`, until
`bun nv site --check review` is green. Then the close check the pack names: a build, clippy and
this goal's `test_reach` tests by name, unless the pack names this goal as a full gate.

`bun nv site --check questions` still names six pages this goal did not touch, stale since
`8ec87a7122`: `/guides/production/{deploy,docker,run-as-a-service}/`, `/in-depth/compare/`,
`/in-depth/` and `/`. `ci --check` fails on them at the next full gate; reread each, rewrite its
questions and `bun nv site --questions --stamp` it (with `MSYS_NO_PATHCONV=1`).

### Backlog

- **A log level set after the first `Core\Log::write` of a run is ignored.** After one write,
  `Core\Config::set('log.level', 'Debug')` returns `true` and `Core\Config::get` reads it back, but
  later `debug` writes in the same run are still dropped, under `nvs run` and `nvs test`. Setting it
  before the first write works. The cause is not read yet; start at `Core\Log::write` in
  `crates/nvs-stdlib/src/log.rs`. Not this goal's: it needs an owner goal.
