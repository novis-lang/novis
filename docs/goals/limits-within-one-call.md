---
looper: 1
title: "a native call stops at the wall-time, CPU and stack limits as it does at the memory ceiling"
summary: "every wait a member makes ends at the request's deadline, native recursion checks the stack before it goes deeper, and the overshoot scan judges CPU and wall-time readings"
context: [crates/nvs-runtime/src/ctx/limits.rs, crates/nvs-runtime/src/ctx/safepoint.rs, crates/nvs-host/src/watchdog.rs, tools/nv/proofs/run.ts]
stages:
  - id: 1
    title: "a survey lists every wait, every native recursion and every member whose cost its caller picks"
    status: done
  - id: 2
    title: "native recursion checks the stack before it goes deeper"
    status: done
    context: [crates/nvs-runtime/src/ctx/mod.rs, crates/nvs-host/src/stack.rs]
  - id: 3
    title: "every wait a member makes ends at the request's deadline"
    status: open
  - id: 4
    title: "a CPU or wall-time FATAL names what the request used, and the overshoot scan judges it"
    status: open
    context: [crates/nvs-runtime/src/abi.rs, tools/nv/cmd/proofs.ts]
  - id: 5
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: deep-native-recursion
    stage: 2
    name: native recursion past the stack ends the request with a FATAL and the process goes on
    run: [cargo, test, --lib, native_recursion_past_the_stack]
    tests:
      - native_recursion_past_the_stack_ends_the_request_and_not_the_process
  - id: time-overshoot
    stage: 4
    name: every attack that ends at its CPU or wall-time limit stopped within its bound
    run: [bun, nv, proofs, --overshoot]
    timeout: 1800
    reach: true
  - id: owners-closes
    stage: 5
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, limits-within-one-call]
    reach: true
    floor: false
---

# a native call stops at the wall-time, CPU and stack limits as it does at the memory ceiling

## Why

A request's limits are noticed anywhere and acted on only at a poll: before and after each `Core`
member, and at the JIT's safepoints. `memory-ceiling-holes` closed that gap for memory. The other
limits have the same shape, and nobody has looked at them the same way:

- **Wall time.** The deadline is checked at a poll. `Core\Time::sleep` is the one member that ends a
  wait at the deadline (`crates/nvs-runtime/src/ctx/limits.rs:987`). Any other blocking wait inside a
  member runs to its own timeout.
- **CPU time.** `nvs_host::watchdog` samples the thread's CPU clock and raises `CPU_LIMIT`
  (`crates/nvs-host/src/watchdog.rs`). Work proportional to its input is bounded by the memory ceiling.
  Work that grows faster than its input, or whose cost the caller picks, is not.
- **Stack.** Compiled code checks the stack at function entry (`crates/nvs-runtime/src/ctx/safepoint.rs:1216`,
  `STACK_CEILING` at `crates/nvs-runtime/src/ctx/mod.rs:1905`). Native Rust recursion has depth caps
  written per site (`json.rs`, `graph.rs`, `signature.rs` and others). A stack overflow in native code
  aborts the process, which ends every request. This is the most serious of the three.

The user's call (2026-10-31): the work starts with a survey, and a later stage does only what the
survey shows is needed.

## Standing decisions

**The survey sets the scope.** Stage 1 writes the item lists of stages 2, 3 and 4. A stage whose
survey list is empty is closed with one line saying so. Nothing is changed in a site the survey did not
list.

**One stack guard, from the existing pair.** Native recursion asks one helper in `nvs-runtime` whether
the stack has room before it goes deeper. The helper compares the stack pointer with the floor the
context already arms (`stack_floor`, `crates/nvs-runtime/src/ctx/safepoint.rs:27`). A refusal is the
same `FATAL` the compiled code's check gives. No new dependency: `stacker` and `psm` were considered,
and the existing pair already knows each stack's bounds, task stacks included
(`crates/nvs-host/src/stack.rs`). A per-site depth cap that gives a catchable error at a sensible depth,
such as JSON's nesting limit, stays: it is a format limit, not a stack guard. Cost: one load and one
compare per recursive step.

**A wait takes the deadline it is given, never a new setting.** A blocking wait inside a member reads
`Ctx::wall_time_left` and waits no longer than the shorter of that and its own timeout, as
`Core\Time::sleep` does. Reaching the deadline gives `wall_time_breach`'s `FATAL`. No lint is added in
this goal: the survey's list is the set, and the decision on a lint waits for its size.

**CPU work whose cost the caller picks is checked before it starts.** Each such member the survey
lists checks its cost against the CPU time left before the work, the way `affordable` checks a count
against memory. Where the cost cannot be known first, the member polls every so many steps, and its
doc comment says how many. The regex backtracking setting (`[regex] backtracking`) already covers
backtracking patterns and is not changed.

**A time limit's overshoot bound is the CPU or wall-time limit plus 100 ms.** The `FATAL` names the
time the request used, the way a memory `FATAL` names its peak. The overshoot judge reads it, and a
reading past the bound is red. The 100 ms is the watchdog's sampling interval plus scheduling noise.
Stage 4 measures the legitimate worst over the existing attacks first, and sets the slack to twice
that if it is larger. The bound is never raised to make an attack pass.

**The output limit counts the response body, wherever it is written.** A streamed chunk, an SSE event
and a file body are output, and each is charged when it is written; a file body by its size. Headers,
diagnostics, and log or dump records sent to a named target are not the response body and stay
uncharged: memory bounds the headers, and each sink has its own bound. Cost: one add and one compare
per streamed write.

**Where the stack helper cannot be asked.** `Ctx::native_stack_step` reads the bounds of the task
stack the context was armed for. A walk that runs on another stack, such as JSON's spare stack, or
that has no `Ctx` and no error type for a fatal, such as the value-graph walk, keeps its format cap.
That cap must be shown to fit by a test or by its size. Threading a `Ctx` through the graph's carriers
to guard a cap of 256 was declined: it spends simplicity for no depth a request can reach. The helper
refuses at `NATIVE_STEP_ROOM` (64 KiB) above the floor, inside the reserve, with the fatal compiled
code gives at the floor. A native walk has no catch point of its own, so it is never a catchable
`Recursion`.

**A wait at the deadline is the FATAL where the member has the context, and the flag elsewhere.** A
member that holds `&mut Ctx` where its wait fails maps the failure through `Ctx::wall_time_fault`, so
the request stops there with the wall-time `FATAL` (`Core\Net`, `Core\Http`, `Core\Mail`). A door
that reaches the wait without the context, such as Redis's `on_shared`, LDAP's operations and the
database's statements, only clamps the wait. Its failure is the member's own error, and the request's
safepoint flag, raised at the deadline by the watchdog sweep or the run's clock, stops the request at
its next poll. Threading `&mut Ctx` into each of those doors was declined: the clamp already ends
the wait, which is the security property, and the flag gives the same `FATAL` one poll later.

**A CLI run's standard input stops at its wall time, when it has one.** `nvs run` arms a wall
deadline whenever `[limits] wall_time` is set, so `Core\IO::stdin` reads chunks of 64 KiB on the
blocking pool with `run_until` and gives up at the deadline with the `FATAL`. No new setting is added.
A run with no wall time reads inline and pays nothing. A read given up leaves its pool thread waiting
on the descriptor until input arrives, which is acceptable because the run is ending. Reading on the
core with a nonblocking descriptor was declined: standard input is not always pollable (a file or a
Windows console), and the pool already has the bounded wait.

**Lanes.** Stage 1 runs first. Stages 2, 3 and 4 touch disjoint files once the survey is in, and each
may run as a side goal.

**Trade-offs.**
- Security: a request cannot crash the process through native recursion, and cannot hold a core past
  its time limits inside one call.
- Latency: one compare per recursive step, and one clock read per wait.
- Memory: nothing is added.
- Simplicity: one stack helper replaces the reasoning each site does today.

## Stages

### 1. a survey lists every wait, every native recursion and every member whose cost its caller picks

**Split:** three read-only `reader` subagents, side by side, over `crates/nvs-stdlib`,
`crates/nvs-db`, `crates/nvs-ldap`, `crates/nvs-host` and `crates/nvs-runtime`. Each returns
`file.rs:NN`, the member that reaches the site and one line each.

- [x] Every blocking wait a member can reach: a socket read or write, a database reply, a child
  process, a lock or a channel. For each: whether it reads the request's deadline today. Written as
  stage 3's items.
- [x] Every native recursion a request's input can drive: a parser, a walk, an encoder, a matcher. For
  each: its depth cap if it has one, and the depth at which the stack would end first. Written as
  stage 2's items.
- [x] Every member whose CPU cost grows faster than its input or is picked by an argument. Examples to
  check: an edit distance, a password hash's cost, decompression, sorting with a callback. Written as
  stage 4's items.
- [x] Whether the output limit is written through one path. If it is not, each other path is a stage 4
  item.

**What the survey found.** A task stack is 1 MiB (`crates/nvs-host/src/stack.rs:202`), so about
768 KiB is usable above the 256 KiB reserve. No native site consults `stack_floor`: only compiled code
calls `nvs_stack_check` (`crates/nvs-codegen/src/emit.rs:1396`). Drop, release, equality and hash are
already iterative. `wall_time_left` has two readers, `Core\Time::sleep` (`crates/nvs-stdlib/src/time.rs:3738`)
and SQLite's busy wait (`crates/nvs-stdlib/src/db/execute.rs:86`). No `Ctx` accessor gives the CPU time
left. `bounded_loop` (`crates/nvs-runtime/src/abi.rs:386`) has two callers, `Core\Arr::map` and
`Core\Task::map`. There is no edit-distance member in `Core`. The output limit is charged only in
`Ctx::write_output` (`crates/nvs-runtime/src/ctx/output.rs:275`).

### 2. native recursion checks the stack before it goes deeper

- [x] The stack helper in `nvs-runtime`, by the standing decision: `Ctx::native_stack_step` and
  `NATIVE_STEP_ROOM` (`crates/nvs-runtime/src/ctx/safepoint.rs:54`).
- [x] The test `native_recursion_past_the_stack_ends_the_request_and_not_the_process`, which drives
  one listed recursion past the stack. Run once by name.
- [x] Each recursion stage 1 listed asks the helper, or is closed by the standing decision *Where the
  stack helper cannot be asked*:
  - [x] `Core\Router`'s `$params` walk, `written_form` (`crates/nvs-stdlib/src/router.rs`). It asks
    at every level. The test above drives it.
  - [x] `Core\Json::decode` and `decodeAs` (`json.rs:1966`, `json.rs:2885`): closed. A text of at most
    `IN_PLACE_BYTES` nests at most 64 deep, and two tests show that fits the reserve. A longer text
    runs on the spare stack, whose bounds the context does not hold.
  - [x] The value-graph walk and read (`crates/nvs-runtime/src/graph.rs:251`, `graph.rs:983`): closed.
    Cap 256, no `Ctx` in scope, and `GraphError` cannot carry a fatal.
  - [x] `Core\Debug`'s record walk (`crates/nvs-runtime/src/record.rs:182`): closed. The survey was
    wrong: its depth is `Caps::default()`'s constant 8, never a setting.
- [x] The third-party recursions take an explicit limit, written beside the call: `regex`'s
  `NEST_LIMIT` 250 (`crates/nvs-stdlib/src/regex.rs`, regex-syntax 0.8's default) and `CBOR_DEPTH` 16
  for the passkey reader's three CBOR reads (ciborium 0.2.2's default is 256).
- [x] An attack step per listed recursion that a `.nvs` file can reach, ending at `FATAL`. Only the
  router walk is left: a `$params` nested from `Core\Uri::parseQuery` and passed to
  `Core\Router::urlSigned`.
  - [x] Find `parseQuery`'s own nesting cap. It has none: the memory limit bounds the nesting
    (`crates/nvs-stdlib/src/uri.rs`, the iterative path builder), so 100,000 levels parse.
  - [x] Write the step under `tests/hostile/`, ending at `FATAL`, and run it once with
    `bun nv proofs --run --only 'Core\Router::urlSigned'`:
    `tests/hostile/core/Router/urlSigned/02-a-value-nested-past-the-stack.nvs`, green.

The other recursions have a format cap that fits the stack and stay as they are: XML 1024 iterative,
HTML 1024 and html5ever 512, signature 32, `assertEqualsDeep` 64, the database schema 16, the LDAP
filter 100, Postgres arrays 6, the redaction walk 128 on the spare stack.

### 3. every wait a member makes ends at the request's deadline

Most waits go through four doors, so most items are one change at a door. Each item reads
`wall_time_left`, by the standing decision.

- [x] The socket door: `NvsStream`'s deadline (`crates/nvs-host/src/net.rs:250`, `net.rs:727`,
  `net.rs:1055`, waits at `net.rs:1414` and `net.rs:1689`). The stream has no `Ctx`, so each member that
  sets a stream's deadline clamps it to the wall deadline. That covers `Core\Net` (`crates/nvs-stdlib/src/net.rs:1419`,
  `:1631`, `:1662`, `:1712`, `:1834`, `:1885`), `Core\Http` (`crates/nvs-stdlib/src/http/transport.rs:1673`,
  `:1723`, `:2093`), Redis (`crates/nvs-stdlib/src/cache/redis.rs:463`), `Core\Mail` (`crates/nvs-stdlib/src/mail.rs:817`,
  `:1054`) and `Core\Ldap` (`crates/nvs-ldap/src/conn.rs:276`).
  - [x] `Ctx::within_wall_time`, `Ctx::wall_bound` and `Ctx::wall_time_fault` (`crates/nvs-runtime/src/ctx/limits.rs`).
  - [x] `Core\Net`: `within` clamps every bound, and `waited` gives the `FATAL`.
  - [x] `Core\Http`: `call_of` and the socket open clamp `deadline` and `maxDuration`, and `wall_or` gives the `FATAL`.
  - [x] Redis: `Connection::set_wall`, filed by `on_shared` for each command.
  - [x] `Core\Mail`: one deadline over the whole exchange. Before, only the connect was bounded.
  - [x] `Core\Ldap`: `held` files the wall on `Held`, and `dial` takes it.
  - [x] Attack `tests/hostile/core/Net-Listener/accept/02-a-wait-past-the-time-limit.nvs`, green.
- [x] The database door: `bound_connection` (`crates/nvs-stdlib/src/db/execute.rs:77`) clamps every
  statement's deadline. That covers the four wire drivers' replies (`crates/nvs-db/src/pg.rs:461`,
  `mysql.rs:449`, `maria.rs:462`, `conn.rs:1236`) and their connects (`pg.rs:1099`, `mysql.rs:1819`,
  `maria.rs:381`, `tds/mod.rs:401`). SQLite's busy wait with a statement `timeout` takes the shorter of
  the two (`execute.rs:86`), and `into_wal`'s retry (`crates/nvs-db/src/sqlite.rs:808`) stops at it.
  - [x] `bound_connection` files the earlier of the statement's deadline and the wall deadline.
  - [x] The four connects take the wall deadline: `open_named` and both `open` arms pass
    `connect_by`, from `execute::within_wall`. The pool's slot wait is the park door's.
  - [x] `into_wal`'s retry stops at the wall deadline: `SqliteTarget::deadline`.
  - [x] A Rust test: `a_connect_to_a_peer_that_never_replies_ends_at_the_wall_deadline`, green.
- [x] The blocking-pool door: `nvs_host::blocking::run` (`crates/nvs-host/src/blocking.rs:442`). Its
  callers wait no longer than the deadline. That covers `Core\Process::run` (`crates/nvs-stdlib/src/process.rs:964`,
  `:1187`, `:1311`), the handle's read, write and kill (`process.rs:1523`) and `Handle::wait` (`process.rs:1661`),
  and the resolver `resolve_off_core` (`crates/nvs-host/src/lib.rs:210`).
  - [x] `Ctx::wall_clamp`, the earlier of a member's own deadline and the wall deadline.
  - [x] `Core\Process`: `run` and `spawn` clamp the child's deadline, and `stopped` gives the
    `FATAL` once the wall time is out. Test `run_stops_a_child_at_the_request_wall_time`, green.
  - [x] The resolver: `blocking::run` gets a bounded twin that stops waiting at a deadline and
    leaves the job to finish on its pool thread, its answer dropped. `resolve_off_core` takes it.
    `blocking::run_until`, test `a_bounded_blocking_call_stops_waiting_at_its_deadline`, green.
    The `Resolver` hook takes a deadline, and `pinned_addresses` passes the wall deadline.
- [x] The three inline `getaddrinfo` calls move to the shared resolver: `crates/nvs-stdlib/src/db/open.rs:152`,
  `mail.rs:428`, `ldap/mod.rs:493`. Today each blocks the core with no bound.
- [x] The park door: each `host.park` a member makes takes the shorter of its own bound and the
  deadline. `crates/nvs-stdlib/src/db/pool.rs:168`, `cache.rs:1507`, `socket.rs:1236`, `socket.rs:1506`,
  `sse.rs:884`, `channel.rs:327`, `crates/nvs-runtime/src/stream.rs:228`, `crates/nvs-runtime/src/ctx/late_init.rs:208`.
  - [x] `db/pool.rs`, `cache.rs`, `sse.rs`, `channel.rs` and `late_init.rs` park with the wall
    deadline and give the `FATAL` on a wake past it.
  - [x] `stream.rs`: `Emit::send_by` takes the wall deadline; `response.rs` and `sse.rs`'s
    `onto_the_wire` pass it and give the `FATAL`.
  - [x] `socket.rs:1236`, `wait_on_both`: `peer.receive_or` waits on the WebSocket peer. It was
    bounded only by the idle and lifetime windows. `PeerSocket::set_wall` files the wall deadline,
    `Framed::arm` caps every wait by it, and a timeout past it is an error the member turns into
    the `FATAL`. `send` takes it too.
  - [x] `socket.rs:1506` is a test double, `Parking`, with nothing to change.
- [x] The backoff sleeps stop at the deadline: `Core\Http`'s retry (`http/transport.rs:1501`) and
  `Core\Db`'s transaction retry (`db/transaction.rs:352`). `Core\Http`'s was already bounded by
  `call.deadline`, which `call_of` clamps.
- [x] `Core\IO::stdin` (`crates/nvs-stdlib/src/io.rs:2263`) reaches only `nvs run`. Decide in the stage
  whether a CLI run has a wall deadline; if it has, the read stops at it. It has one whenever
  `[limits] wall_time` is set, since `refresh_limits` publishes it for every run. Under one, the
  read goes through `StdinUntil` on the blocking pool; with none it stays inline.
- [ ] An attack step per listed wait a `.nvs` file can reach, ending at the wall-time `FATAL`. A wait
  only a remote peer can hold open is a Rust test with a scripted peer.
  - [x] `Core\Task\Channel`: `tests/hostile/core/Task-Channel/close/02-a-wait-past-the-time-limit.nvs`, green.
  - [x] `Core\Cache\Store::getSecret`'s fill wait: `tests/hostile/core/Cache-Store/getSecret/02-a-wait-past-the-time-limit.nvs`, green.
  - [x] The db pool slot wait: `tests/hostile/core/Db/open/03-a-wait-past-the-time-limit.nvs`, green.
  - [ ] The WebSocket `receive`: a Rust test in `crates/nvs-server/src/socket.rs`'s tests, a `Framed`
    over a peer that never sends, `set_wall` a short instant, and `receive` returns `Err`, not `Closed`.
  - [ ] `Core\IO::stdin` under a wall time: a Rust test of `StdinUntil` is not possible over the
    process's own standard input, so a `nvs run` integration test with a held-open pipe, in
    `crates/nvs-cli/tests/`, ends at the `FATAL`.

### 4. a CPU or wall-time FATAL names what the request used, and the overshoot scan judges it

- [ ] The CPU and wall-time `FATAL`s name the time the request used and its limit. `crates/nvs-runtime/src/abi.rs:381`,
  `crates/nvs-runtime/src/ctx/safepoint.rs:1046`, `crates/nvs-runtime/src/ctx/limits.rs:997`
- [ ] `judgeHostile` reads that time and applies the time bound. `bun nv proofs --overshoot` includes
  attacks that end at a time limit. `tools/nv/proofs/run.ts:888`, `tools/nv/proofs/run.ts:976`
- [ ] Measure the legitimate worst over the existing time attacks, and set the slack by the standing
  decision.
- [ ] `Ctx::cpu_time_left`, beside `wall_time_left` (`crates/nvs-runtime/src/ctx/limits.rs:1002`). It
  reads the thread's CPU clock against `cpu_limit` (`limits.rs:1047`). Today only the watchdog reads
  that clock (`crates/nvs-host/src/watchdog.rs:275`).
- [ ] Each member stage 1 listed checks its cost first or polls:
  - `Core\Password::verify` and `hash` (`crates/nvs-stdlib/src/password.rs:469`, `:436`). The stored
    hash picks the cost, up to bcrypt 17 or argon2 4 Mi work. Checked first, from the cost.
  - `Core\Crypto::deriveKey` (`crates/nvs-stdlib/src/crypto.rs:3165`), up to 2M PBKDF2 iterations.
    Checked first. RSA `sign` and `verify` (`crypto.rs:3286`, `:3314`): read whether the key size is
    capped, and cap it if not.
  - `Core\BigInt::powMod` and the wide `mul`, `div`, `mod`, `sqrt`, `gcd`, `toString`, `parse`
    (`crates/nvs-stdlib/src/bigint.rs:1154`, `:1197`, `:1088`). Operand width is bounded by memory alone.
    `powMod` polls per exponent bit; the others are checked first from the widths.
  - `Core\Compress::decompress` and the stream's `add` and `finish` (`crates/nvs-stdlib/src/compress.rs:1097`,
    `:1306`, `:1350`). Up to 64 MiB of output by default. Polls per output chunk.
  - `Core\Regex`'s linear tier, `matchAll`, `replace`, `replaceWith` and `split` (`crates/nvs-stdlib/src/regex.rs:1811`).
    It has no step budget, and the cost is pattern size times input. Polls per match.
  - `Core\Arr::sort`, `sortByKey`, `diff` and `intersect` with a callback (`crates/nvs-stdlib/src/arr.rs:4610`,
    `:4811`, `:6056`). Read whether a callback's entry poll stops the native loop; if not, poll per call.
  - Postgres SCRAM login (`crates/nvs-db/src/pg.rs:1298`): the server picks the PBKDF2 iteration count.
    Cap it, and check it first.
- [ ] The output limit charges every byte of a response body. These paths do not today:
  `Core\Response\Stream::write` to a connection (`crates/nvs-stdlib/src/response.rs:2426`), `Core\Sse`
  events (`crates/nvs-stdlib/src/sse.rs:956`), both through `Emit::send` and `send_all`
  (`crates/nvs-runtime/src/stream.rs:184`, `:254`), and the file body `sendFile` declares
  (`crates/nvs-runtime/src/ctx/output.rs:868`), charged by the file's size.
- [ ] `bun nv proofs --overshoot` exits 0.

### 5. the goal is verified once, and every failure is fixed

**Does:** the stages before this one checked with a build alone, so this stage verifies the
whole goal once and fixes every failure in one pass.

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stage 3 is nearly done. Every door, park and backoff is done, and so is `Core\IO::stdin`. Two
Rust tests of the attack item are left.

The WebSocket peer takes the wall deadline through `PeerSocket::set_wall`
(`crates/nvs-runtime/src/peer.rs`). `Framed` (`crates/nvs-server/src/socket.rs`) caps every wait in
`arm` by it, and `expiry` reads a timeout past it as an error, not a close. `Core\Socket`'s
`receive` and `send_frame` (`crates/nvs-stdlib/src/socket.rs`) give `wall_time_fault` on that error.
`Core\IO::stdin` under a wall time reads through `StdinUntil` (`crates/nvs-stdlib/src/io.rs`) on the
blocking pool. Three attacks end at the wall-time `FATAL`: a channel, a `getSecret` fill wait and a
full db pool, each green under `bun nv proofs --run --id`.

The db pool's own `acquire` bound (5 s) caps an open's `timeout`, and `Core\Config::set('db.pool.acquire')`
does not move it at run time. The attack is still right, since 1 s comes first.

The acceptance sweep's reds are stale: nightly run 41187571578 ran on `857b39cad`, before the
fixes `39795c9c4` and `8ce92dda5`. `env-secret-proofs` has no definition in the tree.

**Tests expected to go red:** a `Core\Ldap` test that expects more than eight addresses of a name.
A cache, channel or SSE test under a `wall_time` shorter than its wait now gets a `FATAL`. A test
that counts parks with a fake host whose `park` asserts on a `None` deadline. A `Core\Socket` test
that expects a timeout under a wall time to read as an idle close, `null`. None is known.

### Next

The two open checkpoints under stage 3's attack item. First, a Rust test in
`crates/nvs-server/src/socket.rs`'s test module: a `Framed` over a loopback peer that never sends,
`set_wall(Some(now + 50ms))`, and `receive_or` returns `Err` and not `Received::Closed`. Look for
an existing `Framed` test there to copy its setup. Second, a `nvs run` integration test in
`crates/nvs-cli/tests/`: a script sets `limits.wall_time` to `1s` and calls `Core\IO::stdin()`, with
the child's stdin a pipe held open, and the run ends at the wall-time `FATAL`. Then set stage 3's
status to done and take stage 4, starting with the `FATAL` text (`limits.rs:1051`,
`wall_time_message`).

### Backlog
