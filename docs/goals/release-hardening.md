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
    status: open
    context: [crates/nvs-cli/src/runner.rs, crates/nvs-runtime/tests/server_memory.rs, .github/workflows/ci.yml]
    extra:
      context:
        rules: [testing/the-deep-lane, security/path-scope-canonicalise-then-prefix, security/net-address-policy]
  - id: 9
    title: "a security fix gets an advisory, and a release proves what it ships"
    status: open
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
and the address table. If a session finds it needs `proptest`, it asks first.

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

- [ ] A runner mode sends many cases through one worker and compares each output with its fresh run. `crates/nvs-cli/src/runner.rs:1`
- [ ] The Miri job runs the `nvs-runtime` and `nvs-stdlib` unit tests that need no JIT. `.github/workflows/ci.yml:1104`
  The user decided how: one pass that lists every failure, never a rerun per failure, and a job that
  finishes inside its budget. Measure with one background run and hand off, never round after round
  inside a session.
  - [ ] Miri runs under nextest, `cargo +nightly miri nextest run`, in CI and locally. Each test gets its
    own process, so a test Miri cannot run fails alone and the run goes on. TSAN covers the cross-test
    races this gives up.
  - [ ] A test that does what Miri can never do gets `#[cfg_attr(miri, ignore = "<the reason>")]` on
    itself, never a module skip and never a CI skip list. Found so far: a Unix socket
    (`ctx::output::tests::a_context_sends_both_writers_to_syslog_when_the_target_names_it`), `openat`
    (`beneath`'s tests and the runtime's `capability` tests that reach the file doors), a process
    (`child_watch`), `mmap` with a guard page (the `json` depth tests) and `ring`'s C and assembly.
    The rest of `beneath` and `capability` is what Miri is for, so it keeps running.
  - [ ] A test that is only slow under Miri runs a smaller input under `cfg!(miri)`, never a skip
    (the `uri` budget test and the large-text JSON tests).
  - [ ] `nvs-runtime` and `nvs-stdlib` run as separate jobs or `--partition` shards, each well under its
    `timeout-minutes`, so the job ends green or red and never cancelled.
  - [ ] Miri's default model, Stacked Borrows. Its first run reported undefined behaviour in the pool
    allocator: a retag through a reference over a block, reached through `counting_alloc::Counting`
    wrapping `alloc::Pooled` (`crates/nvs-runtime/src/alloc.rs:226`). Fix it at its cause if the fix is
    local. If it needs `-Zmiri-tree-borrows` instead, stop with `BLOCKED` and name the report: the
    model is the user's call.
- [ ] Seeded property tests: a grant on `a/secret1` never grants `a/secret`, and every address in a denied range is denied in each of its spellings.
  - [x] The tests are in `crates/nvs-config/tests/capability.rs`, committed as `ae40a9e58`; `cargo test --test capability` passes.
  - [ ] Verified green and kept.
- [ ] A nightly soak serves many requests and fails when live bytes grow with the request count. `crates/nvs-runtime/tests/server_memory.rs:1`

### 9. a security fix gets an advisory, and a release proves what it ships

**Does:** `SECURITY.md` says every security fix gets a GitHub advisory, and that one request
affecting another is a security bug. A release runs the full matrix, checks that its tag holds
the fixes it claims, and builds with `cargo auditable`.

- [ ] `SECURITY.md` states the advisory rule and that a cross-request effect is a security bug. `SECURITY.md:1`
- [ ] The release workflow checks that the tag contains each commit its notes name as a fix. `.github/workflows/release.yml:181`
- [ ] Release binaries are built with `cargo auditable`. `.github/workflows/release.yml:292`

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
   A fix repairs the cause: no loosened bound, skipped test or deny `ignore` entry without the user.
   A timing guard that fails with no code change behind it is noise, and is run again rather
   than fixed.
5. Stop when one run is green on every job.

- [ ] `main` is pushed and a full CI run is started on it.
- [ ] Every failure the runs showed is fixed at its cause and committed.
- [ ] One full CI run on the pushed `main` is green on every job, and the handoff names its id.

A push or `gh` call the loop's permissions refuse is not retried in another form: the status line
is `BLOCKED` and names the call.

## Handoff

### State

Stages 1 to 7 are done. Stage 8 is next.

- **The unoptimised twin.** `NOVIS_NO_OPTIMISE` in the environment makes `nvs` call
  `nvs_codegen::compile_unoptimised` before any command compiles, so every unit of that process is
  at `opt_level` `none` (`crates/nvs-codegen/src/lib.rs:842`), and `cache::unit_for` reads and writes
  no artifact (`crates/nvs-cli/src/cache.rs:1562`). A bundled app ignores it. The `.nvst` runner's
  children inherit it. Locally, all 2,948 conformance cases pass with it set, and `--dump-asm`
  differs at the two levels.
- **CI.** The `test` job runs the suite a second time with the variable set, on every leg, and the
  matrix has a `linux-aarch64` leg on `ubuntu-24.04-arm` (`.github/workflows/ci.yml:131`). The compose
  images all publish arm64. Neither has run on GitHub yet: stage 10's pushed run is their first.
- `rule:testing/probes-on-is-a-tested-configuration` now says the opt-level half is on disk; its
  probes half (a suite run with coverage on) is still not.
- The nightly the sweep listed is again `1a78b49ef`, the same stale remote `main` as before.

### Next

Stage 8, on `crates/nvs-cli/src/runner.rs`, `crates/nvs-test/src/run.rs` and `.github/workflows/ci.yml` (`rule:testing/the-deep-lane`):
- [ ] A runner mode sends many cases through one worker and compares each output with its fresh run. `crates/nvs-cli/src/runner.rs:1`, `crates/nvs-test/src/run.rs:488`
- [ ] The Miri job runs the `nvs-runtime` and `nvs-stdlib` unit tests that need no JIT. `.github/workflows/ci.yml:1104`
- [ ] Seeded property tests for path grants and the address table. `crates/nvs-runtime/src/capability.rs:552`, `crates/nvs-config/src/capability.rs:262`

### Backlog

- The `linux-aarch64` leg and the unoptimised suite step have never run on GitHub. Read both on stage 10's first pushed run before anything else on it (`.github/workflows/ci.yml:131`).
- The suite's probes-on run is still owed: the coverage flags are refused over a `.nvst` tree (`crates/nvs-cli/src/main.rs:2394`, `rule:testing/probes-on-is-a-tested-configuration`).

- `NvsStr::new` takes any `&[u8]`, so the UTF-8 invariant rests on every caller, and the HTTP Garden bug was one that forgot. A checked constructor for bytes from outside, with `new` kept for payloads already text, closes the class (`crates/nvs-runtime/src/string.rs:462`).
- Two more unchecked `string`s: a MySQL column name (`crates/nvs-stdlib/src/db/column.rs:67`, also `db/execute.rs:835`, `db/stream.rs:345`), and a signed cookie's plaintext, opened with empty AAD (`crates/nvs-stdlib/src/signed_cookie.rs:309`).
- `bun nv verify` stops at `crates/nvs-stdlib/tests/spec_registry_coverage.rs:433`: `3a833d02d` deleted the last milestone record, so `data/plan/milestones/` no longer exists, and two tests refuse an empty directory on purpose. The session that retired the milestones decides what replaces them (`tools/nv/cmd/owners.ts:127`).
- `Core\IO::within` does not refuse what `rule:errors/path-component-refusals` lists, beyond device names (`crates/nvs-stdlib/src/io.rs:3657`).
- The Font attack waits on the user's edit or permission: drop `// hostile: ends-early 4 FATAL`, the `echo "step 4\n"` line and the memory sentence (`tests/hostile/novis/Image-Font/fromBytes/01-bytes-that-pretend-to-be-a-font.nvs:3`).
- CI's Windows warm-start budget still fails: a warm `nvs run` is 8.1 ms of work on this host against 6. Resolving the extension tree once per `run`, not three times, is the next cut (`crates/nvs-ext/src/load.rs:567`, `tools/nv/cmd/bench.ts:132`).
- CI's `proofs` jobs fail on proof debt: stale perf records (`bun nv benches --owed`) and owed proofs (`bun nv proofs --owed`). Stage 10 owns it (`.github/workflows/ci.yml:1`).
- A forged payload can put an enum case with a backing value the enum does not declare into an enum-typed property (`rule:enums/representation`). Whether a typed read of it is safe is not checked (`crates/nvs-runtime/src/object.rs:5629`).
- `bun nv verify` fails on load: `crates/nvs-stdlib/tests/ldap.rs:277` and `:1485` hit their socket deadline, and `crates/nvs-cli/tests/live_edit.rs:1010` misses its 30 s swap, beside the other binaries. Each passes alone (`tools/nv/cmd/verify.ts`'s module doc).
- The nightly tests GitHub's `main`, so its `supply chain` failure says nothing about the current lock until the user pushes (`.github/workflows/ci.yml:838`).
- CI's Windows cost guard missed `serve_throughput_scales_from_one_core_to_four_by_the_margin_this_test_names` at 1.38× of 1.5× on `1a78b49ef`. Watch for a repeat before changing the margin (`crates/nvs-cli/src/serve.rs:4093`).
- `bun nv verify` does not run `nv render --website --check`, so the website data goes stale unseen until CI (`tools/nv/cmd/verify.ts:153`).
- `test_request_dispatches_in_process_through_the_compiled_route_table` was red once, also alone, because `Core\Env::context()` was not `Request` (`crates/nvs-cli/src/runner.rs:3410`).
- `Core\Ldap::connect` dials each address of a host in turn, so a `localhost` block pays 2 s on Windows (`crates/nvs-stdlib/src/ldap/mod.rs:489`, `crates/nvs-host/src/net.rs:438`).
- `fullRun` and `runBenches` still take a `deadline` that no caller passes, and `runBenches` an `order` and `onStart` that only the debt passed (`tools/nv/select/full.ts:63`, `tools/nv/proofs/select.ts:265`).
