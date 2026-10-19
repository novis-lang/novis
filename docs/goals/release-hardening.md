---
looper: 1
title: The bug classes other runtimes shipped are closed before the first release
summary: "Every stage below is done: the decoders, the HTTP framing and the doors refuse what their rules say, the new fuzz, differential, JIT and isolation tests run in CI, the release has its gates, and a full CI run on the pushed `main` is green."
context: [docs/rules/programs/memory-priority.md, docs/rules/errors/ambiguous-input-refused.md]
stages:
  - id: 1
    title: "`Core\\Serialize::decode` checks every value it builds"
    status: done
    context: [crates/nvs-runtime/src/graph.rs, crates/nvs-runtime/src/object.rs, crates/nvs-stdlib/src/serialize.rs, docs/rules/classes/serialize-is-a-closed-format.md]
    extra:
      context:
        rules: [classes/serialize-is-a-closed-format, types/erased-member-access, types/string-is-utf8]
  - id: 2
    title: HTTP framing is refused both ways where the rule says so
    status: done
    context: [crates/nvs-stdlib/src/http/transport.rs, crates/nvs-server/src/io.rs, crates/nvs-server/src/serve.rs, crates/nvs-server/src/mount.rs, docs/rules/errors/http-message-defects.md]
    extra:
      context:
        rules: [errors/http-message-defects, errors/ambiguous-input-refused]
  - id: 3
    title: "the address table denies every non-global address, and an upgrade checks its `Origin`"
    status: done
    context: [crates/nvs-config/src/capability.rs, crates/nvs-server/src/route.rs, crates/nvs-server/src/socket.rs, docs/rules/security/net-address-policy.md, docs/rules/security/csrf-is-on-by-default.md]
    extra:
      context:
        rules: [security/net-address-policy, security/csrf-is-on-by-default, concurrency/an-upgrade-is-spawn-shaped]
  - id: 4
    title: the process and file doors hold on every spelling
    status: done
    context: [crates/nvs-runtime/src/capability.rs, crates/nvs-runtime/src/beneath.rs, docs/rules/core-classes/process-refuses-a-shell-target.md, docs/rules/core-classes/process-options.md, docs/rules/security/writes-open-beneath-a-handle.md]
    extra:
      context:
        rules: [core-classes/process-refuses-a-shell-target, core-classes/process-options, security/writes-open-beneath-a-handle, security/path-scope-canonicalise-then-prefix]
  - id: 5
    title: "JSON refuses a repeated key, and the conversion rule says what `parse` accepts"
    status: done
    context: [crates/nvs-stdlib/src/json.rs, crates/nvs-runtime/src/helpers.rs, docs/rules/types/conversion.md]
    extra:
      context:
        rules: [types/conversion, http-server/engine-depth-is-bounded-per-decoder]
  - id: 6
    title: "every parser that reads outside bytes has a fuzz target, and the smuggling payloads are tests"
    status: done
    context: [fuzz/Cargo.toml, .github/workflows/ci.yml, crates/nvs-server/src/serve.rs, docs/rules/testing/the-deep-lane.md]
    extra:
      context:
        rules: [testing/the-deep-lane]
  - id: 7
    title: "compiled code is checked against its unoptimised twin, and aarch64 Linux runs the tests"
    status: done
    context: [crates/nvs-codegen/src/lib.rs, .github/workflows/ci.yml]
    extra:
      context:
        rules: [testing/the-deep-lane]
  - id: 8
    title: "requests that share a worker share nothing, and memory stays flat"
    status: done
    context: [crates/nvs-cli/src/runner.rs, crates/nvs-runtime/tests/server_memory.rs, .github/workflows/ci.yml]
    extra:
      context:
        rules: [testing/the-deep-lane, security/path-scope-canonicalise-then-prefix, security/net-address-policy]
  - id: 9
    title: "a security fix gets an advisory, and a release proves what it ships"
    status: done
    context: [SECURITY.md, .github/workflows/release.yml]
  - id: 10
    title: "the pushed `main` has a green CI run"
    status: open
    context: [.github/workflows/ci.yml]
checks: []
---

# The bug classes other runtimes shipped are closed before the first release

## Why

Before the first release, the advisories, regressions and test setups of PHP, Go, Node and Bun over
the last two years were read and compared with Novis. Novis already avoids the largest families:
it serves HTTP/1.1 only, so the HTTP/2 floods do not reach it; a process takes argv only; maps are
seeded; arithmetic throws; XML refuses a DOCTYPE; regex backtracking has a step budget. What is
left is a set of holes at protocol and decoder edges, and a test suite that is broad but has few
fuzzers and no differential checks. Those two kinds of test found most of the other runtimes'
bugs. Each stage below closes one hole or adds one kind of test, in priority order.

## Standing decisions

**The user decided every item on this page.** Each stage names the choice it implements.

**A session decides and goes on. It does not ask the user.** The user takes the session's
recommendation, so a choice this goal meets is the session's to make. It picks the option that fits
Novis best by the priority ordering, writes the choice and its reason where the change lives (the
rule, the test, the CI comment or the commit message), and carries on. `BLOCKED` is only for a call
the loop's permissions refuse.

**Memory safety first.** `Core\Serialize::decode` builds values without checking them, which is a
memory-safety hole, so it is stage 1. It is the same family as the wider-shape-view gap that
`completeness-holes` holds open.

**Decided and not changed by this goal:** a JSON integer longer than `u64::MAX` keeps becoming a
`float`, as `crates/nvs-stdlib/src/json.rs:69` records. `float` to `string` keeps 14 significant
digits, as PHP's `echo` does (`crates/nvs-runtime/src/fmt.rs:19`). `string as int` keeps Rust's
`parse`, so `+5` and `007` convert, and the rule is rewritten to say so (stage 5). Context-aware
template escaping stays declined (`docs/rules/core-classes/html-template.md`).

**No new limits and no new configuration keys.** The address table grows rows, which is the table
the rule already names. The upgrade's `Origin` check reuses the per-route `csrf: false` escape.

**Property tests need no new crate.** A seeded loop over generated inputs is enough for path grants
and the address table. A session that finds it needs `proptest` adds it and says why in the commit.

**Trade-offs.**
- Stage 1 adds one tag test per field on a decode: latency on that path only, no memory.
- Stage 2 scans the request head once for a repeated `Content-Length` and a bare LF: a small
  latency per request, no memory. An upstream or client with sloppy framing now fails visibly.
- Stage 3 adds one header compare per upgrade. A browser page on another origin can no longer
  open a socket to a route that did not opt out.
- Stage 4 opens a read one path component at a time, a small latency per open. A child process
  no longer inherits variables it is not given, so a script that relied on that must name them.
- Stage 5 makes JSON with a repeated key throw. It costs nothing: the insert already looks the
  key up.
- Stages 6 to 9 change no program's behaviour. They cost CI time.

## Stages

### 1. `Core\Serialize::decode` checks every value it builds

**Does:** a payload whose field value does not fit the field's declared type, or whose `string`
node is not UTF-8, is refused with a decode error. No typed read can see a value of the wrong
kind, whatever the bytes are. The user chose this over signing payloads.

- [x] A `STR` node is validated as UTF-8, and a `BYTES` node is not. `crates/nvs-runtime/src/graph.rs:878`
- [x] Each field value is checked against the declared type before `set_field`, with the checks an erased write already uses. `crates/nvs-runtime/src/graph.rs:961`, `crates/nvs-runtime/src/object.rs:2343`
- [x] A case per shape: a wrong scalar, an object of the wrong class, an array with a wrong element, invalid UTF-8. Each throws.
- [x] The rule says the decode checks every value against the declared type. `docs/rules/classes/serialize-is-a-closed-format.md:1`

### 2. HTTP framing is refused both ways where the rule says so

**Does:** the outbound client refuses every row of the message-defects table in a reply, and the
server refuses a repeated `Content-Length`, a bare LF in the head, and a path with a dot-segment or
an encoded separator, each with a 400 that closes the connection.

- [x] `head_of` refuses a line with no colon, obs-fold, and CTL, NUL, CR or LF in a name or value. `crates/nvs-stdlib/src/http/transport.rs:2785`
- [x] `frame_of` takes only a final `chunked` as chunked, refuses a repeated or unreadable `Content-Length`, and refuses one beside `Transfer-Encoding`. `crates/nvs-stdlib/src/http/transport.rs:2812`
- [x] The server refuses a repeated `Content-Length`, even with equal values, which hyper merges. `crates/nvs-server/src/serve.rs:2308`, `crates/nvs-server/src/io.rs:309`
- [x] The server refuses a bare LF anywhere in the head, which httparse accepts. `crates/nvs-server/src/io.rs:309`
- [x] A dot-segment or encoded-separator path is a 400 at the door and never reaches the app. The module doc's fall-through paragraph is rewritten. `crates/nvs-server/src/mount.rs:47`
- [x] A test per row, inbound beside the smuggling suite and outbound beside the transport tests. `crates/nvs-server/src/serve.rs:9031`

### 3. the address table denies every non-global address, and an upgrade checks its `Origin`

**Does:** the default table denies every address that is not globally routable, and a WebSocket
upgrade from another origin is refused unless its route opted out.

- [x] The table denies the IANA special-purpose ranges, among them 100.64/10, 192.0.0/24, 198.18/15, multicast, 240/4 and broadcast. `crates/nvs-config/src/capability.rs:262`
- [x] 6to4 (2002::/16) and Teredo (2001::/32) are unwrapped to the IPv4 address inside them, as a mapped address already is. `crates/nvs-config/src/capability.rs:298`
- [x] An upgrade whose `Origin` is another origin is refused, and a request with no `Origin` passes, as for a CSRF-checked write. `crates/nvs-server/src/route.rs:214`, `crates/nvs-server/src/socket.rs:86`
- [x] The route's `csrf: false` turns the upgrade check off too. `crates/nvs-server/src/route.rs:122`
- [x] The two rules say what is denied and what an upgrade checks. `docs/rules/security/net-address-policy.md:1`, `docs/rules/security/csrf-is-on-by-default.md:1`

### 4. the process and file doors hold on every spelling

**Does:** a batch target is refused however it is spelled, a child gets only a small fixed
environment unless the program gives one, and a read is opened beneath a handle as a write is.

- [x] Trailing dots and spaces and a `:` stream suffix are cut before the extension check, on every platform. A test per spelling. `crates/nvs-runtime/src/capability.rs:1497`
  - [x] `shell_target` cuts the stream, then trailing dots and spaces.
  - [x] A test per spelling, and the rule says so.
- [x] A child with no `env` gets only PATH, HOME or USERPROFILE, TEMP and TMP, SYSTEMROOT on Windows, and LANG. The rule and the `Core\Process` docs say so. `crates/nvs-runtime/src/capability.rs:1384`, `docs/rules/core-classes/process-options.md:1`
  - [x] `exec_with` clears the environment and copies `BASE_ENV`.
  - [x] The options test proves a variable outside the set is not passed on.
  - [x] The rule, the `env` help text and the stdlib docs say so.
- [x] `open_read` opens beneath a handle, so the check and the open are one step. `crates/nvs-runtime/src/capability.rs:552`, `crates/nvs-runtime/src/beneath.rs:9`
  - [x] `beneath::Mode::Read` opens what is there and creates nothing.
  - [x] `open_read`, `open`'s reading modes and `copy`'s source check the grant on one resolution and open it beneath a handle.
  - [x] A test reads through a planted link and is refused, and the rule names the read doors.
- [x] A test proves `CON`, `NUL`, `COM1` and `CONIN$`, with an extension or a trailing dot, are refused by a file grant and by static serving.
  - [x] One `is_reserved_device` in `nvs_config::capability`, which the mount capture, static serving and the file doors share.
  - [x] Every file door refuses a device component before the grant is read.
  - [x] A test at the doors, one in static serving, and one on the function.

### 5. JSON refuses a repeated key, and the conversion rule says what `parse` accepts

**Does:** `Core\Json::decode` throws on a repeated key and names it, and the conversion rule says
that `string` to `int` accepts a leading `+` and leading zeros.

- [x] A repeated key throws, naming the key. The module doc's refusal list says so. `crates/nvs-stdlib/src/json.rs:1820`
  - [x] `NvsArray::replace` reports a displaced key, and `visit_map` throws on it.
  - [x] The module doc, both `ParseError` help texts and `rule:errors/ambiguous-input-refused`'s list say so.
- [x] A conformance case and an attack case for a repeated key.
- [x] The conversion rule's `string` row says what the code accepts. `docs/rules/types/conversion.md:13`, `crates/nvs-runtime/src/helpers.rs:1536`
  - [x] The row names the sign, leading zeros, fraction and exponent, and what throws.
  - [x] A conformance case pins each spelling.

### 6. every parser that reads outside bytes has a fuzz target, and the smuggling payloads are tests

**Does:** cargo-fuzz targets exist for the request head and body, the outbound reply parser,
multipart, JSON, XML, `Serialize::decode`, cookies, regex compile and decompression, and they run
in the nightly fuzz job. The published request-smuggling payloads are fixed server tests.

- [x] One target per parser above, registered in the fuzz crate and in the fuzz job's matrix. `fuzz/Cargo.toml:23`, `.github/workflows/ci.yml:1038`
  - [x] JSON and `Serialize::decode`.
  - [x] A fuzzing seam for the crate-private parsers.
  - [x] Multipart and cookies.
  - [x] The outbound reply parser, XML, regex compile and decompression.
  - [x] The request head and body.
- [x] The "funky chunks" payloads and the 0.CL and Expect desync shapes are tests in the smuggling suite. `crates/nvs-server/src/serve.rs:9031`
  - [x] An empty inbound `Transfer-Encoding` beside a `Content-Length` is a refused row.
  - [x] The chunk-line payloads are rows of the broken-body table.
  - [x] The 0.CL and Expect shapes are a table of their own, counting final responses.
- [x] HTTP Garden runs once against `nvs serve`, and each finding becomes a test. The handoff records the run.
  - [x] An `nvs` origin echoing the Garden's JSON, built from the WSL mirror.
  - [x] A mutation sweep against hyper and Node, keeping what `nvs` reads differently from both.
  - [x] The UTF-8 finding fixed with tests, and the bare-LF prefix a refused row.
  - [x] The run is a command, `bun nv garden`, and its recheck of 5,000 rounds against hyper and Node found no panic and only the kept `501` for a method outside the eight. `tools/nv/cmd/garden.ts:1`

### 7. compiled code is checked against its unoptimised twin, and aarch64 Linux runs the tests

**Does:** the conformance suite runs with Cranelift's optimiser on and off, and the outputs must
match. A Linux aarch64 leg runs the test job.

- [x] A way to compile with `opt_level` `none`, used only by this check. `crates/nvs-codegen/src/lib.rs:1973`
- [x] A CI job runs the conformance suite both ways and fails on any difference. `.github/workflows/ci.yml:115`
- [x] The test matrix gains a Linux aarch64 leg. `.github/workflows/ci.yml:129`

### 8. requests that share a worker share nothing, and memory stays flat

**Does:** the conformance suite also runs many cases through one worker and matches the fresh
runs. Miri runs the unit tests of the runtime and standard library that run no compiled code.
Seeded property tests cover path grants and the address table. A nightly serve soak fails when
live bytes per request grow.

- [x] A runner mode sends many cases through one worker and compares each output with its fresh run. `crates/nvs-cli/src/shared.rs:1`
- [x] The Miri job runs the `nvs-runtime` and `nvs-stdlib` unit tests that need no JIT. `.github/workflows/ci.yml:1104`
  The user decided how: one pass that lists every failure, never a rerun per failure, and a job that
  finishes inside its budget. Measure with one background run and hand off, never round after round
  inside a session.
  - [x] Miri runs under nextest, `cargo +nightly miri nextest run`, in CI and locally. Each test gets its
    own process, so a test Miri cannot run fails alone and the run goes on. TSAN covers the cross-test
    races this gives up.
  - [x] A test that does what Miri can never do gets `#[cfg_attr(miri, ignore = "<the reason>")]` on
    itself, never a module skip and never a CI skip list. 109 in `nvs-stdlib` and 18 in
    `nvs-runtime` carry one, from the first pass's log. Beyond the list below, they cover `kill`,
    `access`, `getrusage`, `getloadavg`, a UDP socket, and three Miri gaps: `mkdir` ignores its
    mode, `jiff` finds no time zone database, and `powi` gains a rounding error. Found so far: a Unix socket
    (`ctx::output::tests::a_context_sends_both_writers_to_syslog_when_the_target_names_it`), `openat`
    (`beneath`'s tests and the runtime's `capability` tests that reach the file doors), a process
    (`child_watch`), `mmap` with a guard page (the `json` depth tests), `ring`'s C and assembly, and
    SQLite's C (`sqlite3_threadsafe`, the `db` tests). The rest of `beneath` and `capability` is
    what Miri is for, so it keeps running. The first nightly found 27 more that only fail under
    Miri, and each carries its reason: 19 `time` tests (`jiff` reads no zone database under Miri,
    `jiff`'s `tz/db/mod.rs`), 4 coalescing-window tests in `floor` and `log`, 3 real-time deadlines
    in `http`, and `instance`'s dispatch-row test, which compares function addresses.
  - [x] A test that ends with its threads still running joins them, never an ignore. The eight
    `http::transport` cases are fixed: the `proxy` helper returns `Tunnels`, which closes and joins
    every tunnel when dropped, and the header-injection case joins its origin. The six `session`
    cases hold a `Serving` store, which closes the thread's cache connection and joins every
    thread when dropped.
  - [x] Leaks and float noise. The session decided both on the standing decision that it picks:
    `-Zmiri-ignore-leaks` is set for `nvs-stdlib` too, because its 193 reports are values its tests
    build by hand, and `-Zmiri-deterministic-floats` stops the exact-float `math` asserts failing on
    Miri's added error. The `miri-units` comment says why.
  - [x] A test that is only slow under Miri runs a smaller input under `cfg!(miri)`, never a skip
    (the `uri` budget test and the large-text JSON tests). The `uri` budget test is done: it passes
    under Miri in 27 s. The first full pass found 119 in
    `nvs-stdlib`, 57 passing over 60 s and 62 stopped at five minutes. By module: regex 12, cache 11,
    compress 10, crypto 10, cli 9, jwe 7, sort 6, xml 6, db 5, html 5, password 4, cldr 3, str 3,
    ast 2, http::transport 2, json 2, multipart 2, path 2, registry 2, request 2, topic 2, and one
    each in arr, bigint, bus, command, csv, hash, http, random, test, totp, uri and zip. The six
    deep-structure tests in `nvs-runtime` (`array`, `ctx::output`, `floor`, `identity`, `object`)
    are done. The first nightly timed out 71 in `nvs-stdlib`, 3 of them since ignored. The names
    come from the pass's log:
    `grep -E '^\s+(TIMEOUT|PASS) \[\s*>?([6-9][0-9]|[0-9]{3})' .agent-tmp/miri-nx-nvs-stdlib.log`.
    The regex, crypto and jwe tests are done: all 59 that run pass under Miri locally, the longest
    in 106 s. Two tests whose only work is PBKDF2 at the iteration floor carry an ignore, because
    a thousand iterations take 76 s under Miri and the floor is a hundred thousand: there is no
    smaller input. That is the same reason as an operation Miri lacks. The password, cache, str
    and html tests take a smaller input too. Three carry an ignore for that same reason: the two
    Argon2 tests at the module's own 19 MiB parameters, and `http`'s scan of its own source text.
    Two `cache` tests skip only their source-text scan under Miri. The `sort`, html, `str`,
    `http::socket`, streamed-reply, compress, cli, cldr, xml and db tests take a smaller input as
    well, each checked natively at its Miri size. The bcrypt verify runs one verify under Miri,
    because cost 4 is bcrypt's floor. The two `json` tests at the depth ceiling carry an ignore,
    because the ceiling is a constant. The bigint, csv, zip, multipart, request, random, test, arr,
    env and hash tests take a smaller input too. `multipart`'s part-count cap is a constant and
    carries an ignore for the json reason, and `registry`'s two card scans carry the source-scan
    ignore.
  - [x] `nvs-runtime` and `nvs-stdlib` run as separate jobs or `--partition` shards, each well under its
    `timeout-minutes`, so the job ends green or red and never cancelled. The user chose shards: job
    `miri-units` in `.github/workflows/ci.yml` runs 2 for `nvs-runtime` and 12 for `nvs-stdlib`.
    On the first nightly, the 8 `nvs-stdlib` shards reached 88 to 99 % of their tests in 45
    minutes, and 5 were cancelled. On run 39206993876 every shard ended in 8 to 33 minutes and
    none was cancelled. On run 39258294723 all 14 shards were green in 9 to 21 minutes of 45, and
    the slowest test took 218 s of its 300 s.
  - [x] Miri's default model, Stacked Borrows, stays. The undefined behaviour in the pool allocator
    is a block reused through a pointer that covers only its last owner's size. Tree Borrows accepts
    it, and a fix in the pool costs Miri its precision on every recycled block. The user chose the
    `sanitizer` feature for `nvs-runtime`'s run instead, for ASAN's reason, with
    `-Zmiri-ignore-leaks` because the cache keeps its blocks by design. The `miri-units` comment
    says why.
- [x] Seeded property tests: a grant on `a/secret1` never grants `a/secret`, and every address in a denied range is denied in each of its spellings.
  - [x] The tests are in `crates/nvs-config/tests/capability.rs`, committed as `ae40a9e58`; `cargo test --test capability` passes.
  - [x] Verified green and kept.
- [x] A nightly soak serves many requests and fails when live bytes grow with the request count. `crates/nvs-cli/src/serve.rs:4198`
  - [x] The test serves 4,550 requests over 91 loopback connections on one core and reads the
    serving thread's `live_bytes` 525 and 4,475 requests in; it allows 4 KiB of growth.
  - [x] Locally the readings are equal to the byte, and a planted 32-byte leak per request fails it.
  - [x] `cost-guards`' `Serve soak` step runs it under `--release` in the nightly only.

### 9. a security fix gets an advisory, and a release proves what it ships

**Does:** `SECURITY.md` says every security fix gets a GitHub advisory, and that one request
affecting another is a security bug. A release runs the full matrix, checks that its tag holds
the fixes it claims, and builds with `cargo auditable`.

- [x] `SECURITY.md` states the advisory rule and that a cross-request effect is a security bug. `SECURITY.md:1`
- [x] The release workflow checks that the tag contains each commit its notes name as a fix. `.github/workflows/release.yml:181`
  - [x] The tag is cut from the tip the notes were rendered from, so it holds every fix they list.
    The gap was the binaries: they are `github.sha`'s, and the tip can move while they build.
  - [x] `bun nv release --fixes-built <sha>` names each `fix` commit since the last tag that the
    built commit lacks, and fails (`tools/nv/cmd/release.ts`). `publish` runs it on `github.sha`
    before it writes anything.
- [x] Release binaries are built with `cargo auditable`. `.github/workflows/release.yml:292`
  - [x] `cargo-auditable` 0.7.7 is installed pinned and `--locked`. A local probe built with the
    release step's exact flags (`--release --locked --offline -p --target`) carries `.dep-v0`.

### 10. the pushed `main` has a green CI run

**Does:** the goal ends with a full CI run on the pushed `main` where every job is green. The user
asked for this loop, and it repeats until CI is green:

1. Run `cargo fmt --all --check` and the `docs` and `lint` jobs' steps locally, then push `main`.
2. Cancel every older CI run that is still going (`gh run cancel`), start a full run with
   `gh workflow run ci.yml --ref main`, and cancel the subset run the push started. Leave the
   website run alone.
3. Watch that run's jobs every 10 seconds through `gh api ... --jq`, at step level. Report a failed
   step the moment it fails, and print a `WATCH ERROR` line when `gh` itself fails. A failed job's
   log comes from `gh api repos/novis-lang/novis/actions/jobs/<id>/logs`, because
   `gh run view --log-failed` is empty while the run is going.
4. On the first failure, cancel the run, fix the cause, verify locally, commit, and go back to 1.
   A fix repairs the cause. A loosened bound, a skipped test or a deny `ignore` entry is a choice
   the session makes only when it judges that the right fix for Novis, with its reason in the
   commit message and beside the change.
   A timing guard that fails with no code change behind it is noise, and is run again rather
   than fixed.
5. Stop when one run is green on every job.

- [x] `main` is pushed and a full CI run is started on it. `f7eaecd7e` was pushed, and run
  39229718066 dispatched on it.
- [ ] Every failure the runs showed is fixed at its cause and committed.
- [ ] One full CI run on the pushed `main` is green on every job, and the handoff names its id.

A push or `gh` call the loop's permissions refuse is not retried in another form: the status line
is `BLOCKED` and names the call.

## Handoff

### State

Stages 1 to 9 are done. Stage 10 waits on full run 39278827562 on `2dfe330c6`, the first run
with the redis-windows step, and on a GitHub Actions outage.

- **GitHub Actions is down, and this goal waits behind `fewer-allocations`.** githubstatus.com
  opened "Degraded availability for GitHub Actions, GitHub Pages and Webhooks" at 15:49 UTC on
  2026-10-19. At 16:08 the run had jobs in progress and most queued. Its `A Redis server
  (Windows)` step had passed, and `test / windows-x86_64` was building. Push run 39278545350 shows
  `in_progress`, but every job in it is already cancelled. The user said to cancel and dispatch
  nothing more during the outage, and moved this goal behind `fewer-allocations`.
- **The Windows Redis step no longer uses Chocolatey or Memurai.** On run 39276660114 (`94ff16d2f`)
  the feed answered 503 again, and `choco` printed "installed 0/0 packages" but exited 0, so the
  retry loop never retried. At the user's request, the step now downloads Redis 8.10.2, the version
  `tests/db/compose.yaml` pins, from the redis-windows/redis-windows GitHub release. It checks a
  pinned SHA256, unzips it and starts `redis-server` on 16379 (`.github/workflows/ci.yml:187`).
  A retry loop that checked for `memurai.exe` was pushed first as `3d8e54c48`, and the commit
  after it replaces that loop. Runs 39276660114, 39278545350 and 39278568537 are cancelled.
- The user gave the push-and-watch loop to this loop alone.
- **Run 39272328614 (`067fae174`) failed two jobs in its first 15 minutes. It was cancelled, and
  both are fixed:**
  - `asan`: a stack-buffer-overflow in `nvs-stdlib`'s `html_later`, right after ASan warned it was
    ignoring `__asan_handle_no_return` on a stack it did not know. A panic unwinding on a task's
    coroutine stack left stale poison that a later task hit. The previous run was green on the same
    code. `corosensei`'s own `sanitizer` feature reports each switch and unpoisons a stack before
    reuse. `nvs-host/asan` forwards it, and the ASan step turns it on (`2ad211bb6`,
    `crates/nvs-host/Cargo.toml:31`, `.github/workflows/ci.yml:1290`). Under WSL, nine `html_later`
    runs and the whole step had no ASan report. Its four failures there check file modes and Unix
    sockets, which `/mnt/e` lacks.
  - `test / windows-x86_64`: its Redis step failed. The step is replaced, as the first bullet says.
  - So that run checked neither the Windows `test` leg's time nor the zip attack.
- **Run 39258294723 (`f115f6791`) was green on every job but two, and both are fixed:**
  - `proofs / windows-x86_64`: the `Core\Zip::extract` attack printed its last line and was still
    running at its 30 s limit. Step 6c created 19,200 folders (300 files, 64 deep), 6.7 s of the
    program's 8.7 s locally, all of it the file system. It now writes 100 files and runs in 3.5 s
    (`b99670cc9`). The exit sweep of its folders costs about 0.6 s.
  - `test / windows-x86_64` was cut off at 90 minutes in its last step: a cold 37-minute build, 31
    minutes of tests and 17 of the shared-worker conformance suite. macOS took 78. The `test` legs
    now have 120 minutes, with the reason beside the number (`067fae174`, `.github/workflows/ci.yml:127`).
- **Stage 8 is done on that run.** All 14 `miri units` shards were green in 9 to 21 minutes of 45.
  The slowest tests were `regex`'s flag test (218 s), `every_registered_member_has_an_implementation_address`
  (213 s) and the transport's compressed replies (203 s), each under the 300 s limit. The `Serve
  soak` step passed on all three `cost guards` legs, and the shared-worker `Conformance suite`
  step passed on all four `test` legs.
- **Its run ran to the end on purpose.** The proofs failure came early. Cancelling then would have
  lost the Miri, soak and shared-worker results stage 8 waited on, so the session fixed it while
  the run went on.
- **The installed `looper` has no `verify` command.** `bun tools/looper/main.ts verify` runs it.
- **What earlier sessions found that still holds:**
  - The `proofs` legs reach the examples on all three runners. Windows' Docker runs Windows
    containers, so every `requires:` program is skipped there with that reason.
  - Four LDAP examples print several entries in the server's order and pass on CI only by that
    order: `Ldap-Entry/dn/01`, `Ldap-Changes/cookie/01`, `Ldap-Entries/references/02` and
    `types/Ldap-Connection-Scope/01`. The user has not decided whether to sort them.
  - Stamp an example with an `nvs` built from the tree being stamped.
  - `cost guards` missed core scaling once on Linux (1.33× of 1.5×) and once on Windows. Each
    passed on the next run. A timing guard that fails with no change behind it is run again.
  - `bun nv verify` fails on load only in `nvs-ext` `spreadsheet_read`, `nvs-cli` `live_edit`,
    `init` and `examples` (`cache-shared-tls.nvs` missed its connect deadline), and `nvs-stdlib`
    `ldap`. Each passes alone.
- **Shard logs:** `gh api --allow-escape-sequences repos/novis-lang/novis/actions/jobs/<id>/logs`,
  strip `\x1b\[[0-9;]*m` with `perl -pe`, then grep `(TIMEOUT|PASS|FAIL) +\[ *>?([0-9]{3})\.`.
- **A watch script** that prints each failed job and step once, and the run's end, is a few lines
  of `gh api .../runs/<id>/jobs --jq` in a loop. Run it under Monitor and re-arm on expiry.

### Next

Stage 10's push-and-watch loop over `.github/workflows/ci.yml`:
- [x] Run 39258294723: every failure is fixed at its cause and committed (`b99670cc9`, `067fae174`), and stage 8 is done. `docs/goals/release-hardening.md:263`
- [x] `main` is pushed at `067fae174`, and full run 39272328614 is dispatched. `docs/goals/release-hardening.md:332`
- [x] Run 39272328614: the `asan` and Windows Redis failures are fixed at their causes and committed (`2ad211bb6`, `94ff16d2f`), and `main` is pushed with full run 39276660114 dispatched. `.github/workflows/ci.yml:1290`
- [x] Run 39276660114: the Windows Redis step is replaced by a pinned GitHub release download, and `main` is pushed with a new full run dispatched. `.github/workflows/ci.yml:187`
- [x] Run 39278827562 on `2dfe330c6`: `A Redis server (Windows)` passed. `.github/workflows/ci.yml:187`
- [ ] When this goal is live again, check run 39278827562 at step level. If it is gone, cancelled by the outage or red, fix any failure at its cause, push `main` and dispatch a new full run, and watch it with the watch script the State describes, under Monitor. `docs/goals/release-hardening.md:336`
- [ ] Check `test / windows-x86_64`'s total time against its new 120 minutes, and the zip attack on `proofs / windows-x86_64`. `.github/workflows/ci.yml:127`
- [ ] When one run is green on every job, tick stage 10's two open items with its id, set stage 10 done, run the goal-end gates (`bun nv verify --doc`, `bun nv owners --closes release-hardening`), and write `DONE`. `docs/goals/release-hardening.md:351`

### Backlog

- A warm baseline run is 30.6 M instructions, and 61% is the front end. The largest parts are
  `build_signatures` (10 M), `seed_natives` (6.5 M), `builtin_manifests` (4.5 M, which parses every
  built-in extension's manifest on every start) and the TOML parse of `default.toml` (4.4 M).
  Precomputing the manifests at build time is the plainest cut (`crates/nvs-ext/src/load.rs:704`).
- `nvs-runtime`'s pool hands a block back through a pointer narrowed to its last owner's size. Exposing each block's provenance and rebuilding the pointer in `push` and the same-class `realloc` removed the report, but no unit test reproduced it, and the user chose the `sanitizer` feature instead (`crates/nvs-runtime/src/alloc.rs:176`).
- `test / linux-aarch64` and every `test` and `cost guards` leg passed on run 39206993876, macOS included. Check the unoptimised suite step's log on that leg before stage 10 relies on it (`.github/workflows/ci.yml:131`).
- The suite's probes-on run is still owed: the coverage flags are refused over a `.nvst` tree (`crates/nvs-cli/src/main.rs:2394`, `rule:testing/probes-on-is-a-tested-configuration`).
- `NvsStr::new` takes any `&[u8]`, so the UTF-8 invariant rests on every caller, and the HTTP Garden bug was one that forgot. A checked constructor for bytes from outside, with `new` kept for payloads already text, closes the class (`crates/nvs-runtime/src/string.rs:462`).
- Two more unchecked `string`s: a MySQL column name (`crates/nvs-stdlib/src/db/column.rs:67`, also `db/execute.rs:835`, `db/stream.rs:345`), and a signed cookie's plaintext, opened with empty AAD (`crates/nvs-stdlib/src/signed_cookie.rs:309`).
- `bun nv verify` stops at `crates/nvs-stdlib/tests/spec_registry_coverage.rs:433`: `3a833d02d` deleted the last milestone record, so `data/plan/milestones/` no longer exists, and two tests refuse an empty directory on purpose. The session that retired the milestones decides what replaces them (`tools/nv/cmd/owners.ts:127`).
- `Core\IO::within` does not refuse what `rule:errors/path-component-refusals` lists, beyond device names (`crates/nvs-stdlib/src/io.rs:3657`).
- `test / macos-aarch64` on `be3e8a605`: the `nvs-cli` `nvs` test binary died of SIGSEGV with no stack-overflow line, after 116 of 247 tests. The test is not named. It needs a run with `--test-threads=1` on macOS (`.github/workflows/ci.yml:131`).
- A forged payload can put an enum case with a backing value the enum does not declare into an enum-typed property (`rule:enums/representation`). Whether a typed read of it is safe is not checked (`crates/nvs-runtime/src/object.rs:5629`).
- `bun nv verify` fails on load: `crates/nvs-stdlib/tests/ldap.rs:277` and `:1485` hit their socket deadline, and `crates/nvs-cli/tests/live_edit.rs:1010` misses its 30 s swap, beside the other binaries. Each passes alone (`tools/nv/cmd/verify.ts`'s module doc).
- The nightly tests GitHub's `main`, so its `supply chain` failure says nothing about the current lock until the user pushes (`.github/workflows/ci.yml:838`).
- CI's Windows cost guard missed `serve_throughput_scales_from_one_core_to_four_by_the_margin_this_test_names` at 1.38× of 1.5× on `1a78b49ef`, and the Linux one at 1.33× on `58df55643`. Each passed on the next run. It is a single-bound timing, which `rule:testing/perf-two-mechanisms` declines, as the warm-start figure was. Ask the user before giving it a soft and a hard bound (`crates/nvs-cli/src/serve.rs:4120`).
- `bun nv verify` does not run `nv render --website --check`, so the website data goes stale unseen until CI (`tools/nv/cmd/verify.ts:153`).
- Nor does it run `bun nv site --check structure stale questions prose`, which needs no binary. That is how 788 pages' questions went owed or stale with only the nightly `proofs` legs failing (`tools/nv/cmd/verify.ts:153`).
- `test_request_dispatches_in_process_through_the_compiled_route_table` was red once, also alone, because `Core\Env::context()` was not `Request`, and red again on CI's Windows leg on `be3e8a605` with 200 for 201 (`crates/nvs-cli/src/runner.rs:3410`).
- `Core\Ldap::connect` dials each address of a host in turn, so a `localhost` block pays 2 s on Windows (`crates/nvs-stdlib/src/ldap/mod.rs:489`, `crates/nvs-host/src/net.rs:438`).
- The Cranelift panic on the shared worker (`cranelift-codegen` 0.136.3 `machinst/vcode.rs:1901`) has not repeated. Rerun `RUST_BACKTRACE=1 target/debug/nvs test --shared-worker tests/conformance` and read the backtrace if it does: one thread compiling many units, after other cases ran on it, is where it happened (`crates/nvs-cli/src/shared.rs:171`).
- `fullRun` and `runBenches` still take a `deadline` that no caller passes, and `runBenches` an `order` and `onStart` that only the debt passed (`tools/nv/select/full.ts:63`, `tools/nv/proofs/select.ts:265`).
- Three Miri tests ran 203 to 218 s on run 39258294723, against a 300 s limit: `regex`'s flag test (32 s locally), `every_registered_member_has_an_implementation_address` and the transport's compressed replies. A slower runner day could reach the limit (`crates/nvs-stdlib/src/regex.rs`, `crates/nvs-stdlib/src/lib.rs`).
- The shared-worker run added about 6 minutes to each `test` leg's conformance step (11 to 17 minutes on Windows). It runs every eligible case a second time (`crates/nvs-cli/src/shared.rs`).
- This goal owns the gap record `bigint-gcd-and-modular-power-grow-quadratically`, handed over by `fewer-allocations`. `Core\BigInt`'s `gcd`, `lcm` and odd-modulus `powMod` are quadratic in an operand's width, and only the memory ceiling bounds that width, so request text reaches the quadratic cost. `nv scaling` never accepts `complexity quadratic`. Close it with a half-gcd and a Montgomery step over Karatsuba, or with a width cap on the three members, before this goal's `owners --closes` gate (`crates/nvs-stdlib/src/bigint.rs:68`, `data/gaps/nvs-stdlib/bigint-gcd-and-modular-power-grow-quadratically.json`).
