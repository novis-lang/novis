---
looper: 1
title: "A request cannot crash the server, slip past the tainted check, or outgrow its caps"
summary: "Every finding of the 2026-10-21 audit that no other goal owns is fixed or written into its rule as declined: no request crashes the process through an uninitialised property, a rewritten readonly property or a string that is not UTF-8; no Core member or derived row returns a tainted input without its mark; parsers, connections, spans, sessions and extension instances stay under a cap that is per request or per in-flight request; pooled database connections carry no session state into the next request; Windows network paths, development static serving and the CSRF cookie read are closed; nvs serve and every reload refuse a configuration file another account can write, and a read-only mount passes; every fix has its attack; every rule this goal touched says what is true; and the checks below pass."
context: [docs/rules/testing/hostile-case-contract.md, docs/rules/http-server/an-unsafe-or-unbounded-default-is-a-defect.md]
stages:
  - id: 1
    title: "every repro runs again on main, and what is already fixed is struck"
    status: done
    context: [tests/hostile/README.md]
  - id: 2
    title: "a constructor cannot leak `$this` early or run twice, so the JIT's slot reads hold"
    status: done
    context: [crates/nvs-types/src/ctor_init.rs, crates/nvs-types/src/expr/assign.rs, crates/nvs-types/src/expr/calls.rs, crates/nvs-codegen/src/emit.rs, docs/rules/classes/definite-property-initialization.md, docs/rules/classes/readonly-is-written-once.md, docs/rules/classes/constructor-is-a-method-named-constructor.md]
  - id: 3
    title: every string made from outside bytes is checked UTF-8
    status: done
    context: [crates/nvs-stdlib/src/csv.rs, crates/nvs-stdlib/src/jwe.rs, crates/nvs-runtime/src/string.rs, tools/nv/cmd/lints.ts, docs/rules/types/string-is-utf8.md]
  - id: 4
    title: "one test walks every Core signature for a lost mark, and `Core\\Arr` and `Core\\Str` keep it"
    status: done
    context: [crates/nvs-stdlib/src/arr.rs, crates/nvs-stdlib/src/str.rs, crates/nvs-types/src/derive.rs, crates/nvs-types/src/expr/args.rs, docs/rules/security/taint-propagation.md, docs/rules/security/launderers-are-sink-named.md]
  - id: 5
    title: "signed cookies, streams, database rows and extension calls keep the mark"
    status: done
    context: [crates/nvs-stdlib/src/signed_cookie.rs, crates/nvs-stdlib/src/response.rs, crates/nvs-types/src/derive.rs, crates/nvs-types/src/ext_lib.rs, crates/nvs-stdlib/src/out.rs, crates/nvs-stdlib/src/script.rs, docs/rules/security/tainted-sources.md, docs/rules/security/secret-does-not-cross-an-extension.md, docs/rules/core-classes/db-column-types.md, docs/rules/security/capture-answers-the-carrier.md, docs/rules/security/isolate-output-is-captured.md]
  - id: 6
    title: "an array a Rust parser builds stops at the request's memory ceiling"
    status: done
    context: [crates/nvs-runtime/src/array.rs, crates/nvs-runtime/src/budget.rs, crates/nvs-stdlib/src/csv.rs, crates/nvs-stdlib/src/json.rs, crates/nvs-stdlib/src/xml.rs]
  - id: 7
    title: "connections, spans, metrics and blocking work grow with requests in flight, never with requests served"
    status: done
    context: [crates/nvs-server/src/serve.rs, crates/nvs-server/src/admit.rs, crates/nvs-server/src/trace.rs, crates/nvs-server/src/otlp.rs, crates/nvs-host/src/blocking.rs, crates/nvs-host/src/ladder.rs, docs/rules/http-server/admission-is-arithmetic-not-a-number.md, docs/rules/observability/sampling-is-head-based.md]
  - id: 8
    title: an extension instance ends with its request
    status: done
    context: [crates/nvs-cli/src/extensions.rs, crates/nvs-ext/src/call.rs, docs/rules/packaging/a-fresh-instance-per-request.md, docs/rules/security/extension-grants-are-an-intersection.md]
  - id: 9
    title: "sessions, the CSRF door and signed cookies read and bind one way"
    status: done
    context: [crates/nvs-server/src/route.rs, crates/nvs-stdlib/src/request.rs, crates/nvs-stdlib/src/session.rs, crates/nvs-stdlib/src/signed_cookie.rs, docs/rules/security/csrf-is-on-by-default.md, docs/rules/core-classes/session-is-started-explicitly.md]
  - id: 10
    title: "a pooled database connection starts clean, and SQLite opens only what the grant names"
    status: done
    context: [crates/nvs-db/src/pg.rs, crates/nvs-db/src/sqlite.rs, crates/nvs-stdlib/src/db/open.rs, crates/nvs-db/src/tds/rows.rs, docs/rules/security/db-pool-reset-is-a-boundary.md]
  - id: 11
    title: a Windows path is refused by its text before the file system sees it
    status: done
    context: [crates/nvs-runtime/src/capability.rs, crates/nvs-stdlib/src/io.rs, docs/rules/security/path-scope-canonicalise-then-prefix.md, docs/rules/security/writes-open-beneath-a-handle.md]
  - id: 12
    title: "development serving answers only loopback and serves no project file, and the HTTP front's small gaps are closed"
    status: done
    context: [crates/nvs-server/src/mount.rs, crates/nvs-server/src/statics.rs, crates/nvs-server/src/body.rs, crates/nvs-server/src/forwarded.rs, crates/nvs-stdlib/src/request.rs, crates/nvs-stdlib/src/router.rs, docs/rules/http-server/static-serving-is-one-policy.md, docs/rules/http-server/host-matching-is-on-the-host-part-only.md, docs/rules/http-server/the-body-is-read-on-demand-under-two-caps.md]
  - id: 13
    title: "the outbound client's redirects, the leftovers, and the audit's scratch is gone"
    status: done
    context: [crates/nvs-stdlib/src/http/transport.rs, crates/nvs-config/src/capability.rs, crates/nvs-stdlib/src/html.rs, docs/rules/http-server/a-cross-origin-redirect-drops-credentials.md, docs/rules/core-classes/html-sanitize.md]
  - id: 15
    title: "`nvs serve` and every reload check who owns the configuration, and a read-only mount passes"
    status: done
    context: [crates/nvs-cli/src/config.rs, crates/nvs-cli/src/reload.rs, crates/nvs-config/src/trust.rs, crates/nvs-config/src/resolve.rs, docs/rules/config/ownership-is-the-trust-boundary.md, docs/rules/config/the-ownership-check-runs-where-it-can-be-answered.md, tests/docker/smoke.sh]
  - id: 14
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: types-tests
    stage: 2
    name: "the checker refuses an early `$this` and a second constructor call"
    run: [bun, nv, verify, --fast, -p, nvs-types]
  - id: stdlib-tests
    stage: 5
    name: "every Core signature keeps a tainted input's mark"
    run: [bun, nv, verify, --fast, -p, nvs-stdlib]
  - id: server-tests
    stage: 12
    name: "the server's admission, trace, mount and static tests pass"
    run: [bun, nv, verify, --fast, -p, nvs-server]
  - id: cli-tests
    stage: 8
    name: extension instances end with their request
    run: [bun, nv, verify, --fast, -p, nvs-cli]
  - id: db-tests
    stage: 10
    name: a pooled connection starts clean
    run: [bun, nv, verify, --fast, -p, nvs-db]
  - id: config-tests
    stage: 15
    name: "the served configuration is ownership-checked, and a read-only mount passes"
    run: [bun, nv, verify, --fast, -p, nvs-config]
  - id: rules-check
    stage: 13
    name: every rule loads and every citation resolves
    run: [bun, nv, rules, --check]
  - id: owners-closes
    stage: 13
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, security-hardening]
    reach: true
    floor: false
---

# A request cannot crash the server, slip past the tainted check, or outgrow its caps

## Why

A security audit of `nvs serve` on 2026-10-21 (commit a2b4854a4) found holes no goal owned. One request
can segfault the whole server: through a property read before the constructor wrote it, and through a
string that is not UTF-8 from an uploaded CSV. Visitor data reaches a SQL query as a plain `string`
through `Core\Str::format`, `Core\Arr::keys`, `Core\Arr::shapeAs`, `Core\SignedCookie::open` and a
derived database row, so the injection compiles and runs. Parsers, idle connections, sampled spans and
new sessions grow past every cap. Extension calls fail for the whole process from the 64th request on.
On Windows, `Core\IO::within` sends the server account's credentials to any host named in a request.

A finding from 2026-10-22, outside the audit, is stage 15. `nvs serve` and its reloads resolve the
tree through `Unowned`, the reader without the ownership check (`crates/nvs-cli/src/config.rs:143`).
They always have. Both rules that say serve checks are `shipped`, so any account that can write a served
`nvs.toml` grants itself every capability. The Docker smoke test serves a bind mount the runner's
account owns, and it passes only because of this hole.

Security and request isolation is the first priority, so this goal comes right after the live one.

**The audit's evidence is deleted.** Its reports and repro programs were under `.agent-tmp/sec-audit/`,
and stage 13 removed them once every finding was fixed or declined. Each item below still names its
finding ID, and the attack each fix landed with is the finding's lasting form.

## Standing decisions

**The goal runs in the main tree, in one lane.** The evidence is under `.agent-tmp/`, which git does
not carry into a worktree. Stages share crates (`csv.rs` in stages 3 and 6, `derive.rs` in 4 and 5,
`signed_cookie.rs` in 5 and 9, `serve.rs` in 7 and 12), so they run in order.

**Every fix has its attack, and the attack is written first.** It goes under `tests/hostile/` by
`rule:testing/hostile-case-contract`. Where the harness cannot hold the shape (two requests at once, a
crash of the process, a network listener, a Postgres server), it is a Rust test beside the code, named
in the rule's `guardedBy`. A crash attack proves the server answers the next request. Its comments follow
AGENTS.md § *Text an end user reads*. A fix changes no syntax and adds no `Core` member, so it owes no
feature proofs, unless an item says otherwise.

**A finding already fixed on main is struck, not redone.** `defaults-that-work` and `failure-paths-hold`
landed after the audit. Stage 1 runs every repro against a debug build of main and strikes each one
that no longer reproduces, naming the commit.

**No new configuration key and no new limit.** Each bound below is derived from one that exists, or is
a constant with its reason in the code. This is the user's standing direction.

**Stage 2: two-phase construction, as Swift has it.** A constructor assigns every property of its own
class that has no default before it calls `parent::constructor(...)`, and before `$this` escapes in any
way: a method call on `$this`, passing or storing `$this`, a closure that captures it, or a `throw`
while `$this` is reachable. `rule:classes/definite-property-initialization` says so, and the diagnostic
names the first property not yet assigned. `constructor` is called only through `new` and as one
`parent::constructor(...)` statement on every path of a constructor. `$x->constructor(...)`, a second
`parent::constructor(...)` and a dynamic call named `constructor` through `mixed` do not compile, and
the dynamic form throws `Error` at run time where the checker cannot see it. The JIT's untested slot
reads (`crates/nvs-codegen/src/emit.rs:4248`) stay as they are, because the checker now proves what they
assume. Safe default if the corpus has a pattern the rule cannot express: a tagged read at that slot
for classes whose constructor lets `$this` escape, one compare and branch per read on those classes
only, with the cost written in the rule.

**Stage 3: bytes that are not UTF-8 throw where they enter.** `Core\Csv::rows` throws `ParseError`
naming the line, and `Core\Jwe::decrypt` throws `ParseError` once its tag holds. `ParseError` is what
`Core\Json::decode` already throws for input that is not its format, and a new `Core\Csv\Error` class
would be a `Core` member this goal does not add (safe default, taken 2026-10-22). `Core\Csv::parse`
needs no check: its input is a `string`, and it cuts and removes ASCII bytes only. No string is made
from unchecked bytes outside audited sites. `clippy.toml`'s `disallowed-methods` is the list, not a
`bun nv lints` text rule: clippy resolves paths, so a re-export cannot hide a call, and each audited
site says why in `#[expect(clippy::disallowed_methods, reason)]`. Replacing bad bytes with U+FFFD is
declined in `rule:types/string-is-utf8`.

**Stage 3's second half: `Value::str` is the door.** A `string` value is made from an `NvsStr` only
through `Value::str`, which takes bytes and checks nothing. It joins `clippy.toml`'s list.
`Value::text(&str)` is added beside it, typed UTF-8 by Rust, and every site that starts from a `&str`
or a byte literal moves to it mechanically. Every other site either checks with `std::str::from_utf8`
and throws, or keeps `Value::str` under an `expect` that says why its bytes are UTF-8.
`dispatch.rs:850`'s `Ctx::str(&str)` already has the safe shape.

**Captured output is checked where the capture ends (the user's call on d1, 2026-10-22).** Output
captured by `Core\Out::capture`, `Core\Html::later` or a later slot can hold raw bytes from
`Core\Response::bytes` or `::sendFile`. Each capture throws `RuntimeError` naming the member when the
output it ends with is not UTF-8. `Core\Test::request` keeps the body as bytes, and its `body`, `json`
and `jsonAs` readers throw that `RuntimeError` (the user's call on d6, option 1).

**Stage 4: a keystone test reads every `Core` signature.** It fails naming each member whose return
contains `string`, or an array or shape of it, while a parameter is a qualified container
(`Text(Contagious)`, `Array` or `Mixed`) and the return does not carry the contagion. The allow list is
the launderers the rules name (`rule:security/launderers-are-sink-named`), each with its rule id.
Fixing members one at a time is how this class of bug got in. The test is the list.

**Stage 5: a database row is tainted, as the rule already says.** `rule:security/tainted-sources` says
values read back from a database are `tainted`, which closes stored injection, and the code disagrees
(`crates/nvs-types/src/derive.rs:1466`). The code changes. `queryAs<T>`, `streamAs<T>` and a
`#[Derive]` class fill a text column only into a `tainted string` property, and the diagnostic names
that fix. Every example, page and app in the repository is updated to compile. Safe default if that
breaks more than the examples and `apps/`: keep the compile error and add the property form to the
error's suggestions, which the editor applies as a quick fix.

**Stage 6: an array built in Rust asks the budget before it grows.** `NvsArray::try_new` exists for
`uri.rs` from `failure-paths-hold`. Every helper that builds an array or a string from input
(`csv.rs`, `json.rs`, the HTML and XML parsers) uses the fallible form and throws the memory-limit
error at the ceiling, inside the parse.

**Stage 7: connection memory has its own bound, and admission reads requests only.** hyper's buffer
size is set to the head limit `MAX_HEADER_LINES` already pairs with (`crates/nvs-server/src/serve.rs:2373`).
A buffered response keeps its in-flight place until its body is written. Admission's memory check reads
the memory charged to requests, never the core thread's total. No cap on open connections is added.
No test plays a client a proxy stops first: a slow head, body or reader, an idle connection, or a
flood from one address (`rule:http-server/two-deployments-and-nothing-a-proxy-owns`). The attack is
overload a proxy passes: valid requests past capacity, many ending at a limit at once, and every
place and byte back once the flood ends. Pieces of that are tested one mechanism at a time
(`crates/nvs-server/src/admit.rs:478`, `crates/nvs-server/src/serve.rs:7306`); the real binary under
all of them together is not. With no exporter configured, no span is queued, and a client's sampled flag is read only when one is.
The `method` metrics label is the method for the nine standard methods and `other` for anything else.
A 501 for an unknown method is logged at `info`, since the client caused it. The log's `request_id` is
always the server's own, and the trace id stays a separate field. CPU a request spends on the blocking
pool is added to its own CPU time.

**Admission counts a burst before it allocates (the user's call on d2, 2026-10-22).** The `flood`
row showed a burst admitted whole, because no request of it had allocated yet, and the process limit
then stopped those past the room with `500`. An admitted request now holds a provisional share of the
room from its admission until its first publish, so a burst past the room is refused with `503` and
`Retry-After` before it allocates. It stays O(in-flight) and adds no setting. Shedding by CPU wait
was declined: it is a new admission condition.

**The share is 1 MiB, and a started request runs next (the user's call on d3, 2026-10-22).** The
scheduler's ready queue is FIFO (`crates/nvs-host/src/scheduler.rs:930`), so in a cold burst every
ready connection passed `Admission::admit` before any request ran, and a hold looked like a light
request. The share is 1 MiB, the publish step. A request `Isolate::start` queues runs before the next
ready connection, so it allocates and publishes before the next admission. Holds are refused with
`503` past about 32 in the `flood` row's room, and a light burst is never refused. This changes the
run order on a core: a new request goes ahead of tasks already ready.

**Stage 8: an extension instance is released when its request tree ends.** That is what
`rule:packaging/a-fresh-instance-per-request` says. wasmtime's instance limit is set from `max_in_flight`,
never the constant 64. Redirects an extension follows are checked against the extension's own grant
intersection, and a narrowed `grants:` of an isolate reaches an instance its tree already holds.

**Stage 9: one cookie reader.** The CSRF door uses the reader `Core\Session` uses
(`crates/nvs-stdlib/src/request.rs:2184`): every `Cookie` line, value by value. `Core\Session::start`
on a visitor with no cookie writes no row and sends no cookie until the session is first written.
Signed cookies bind the cookie's name and a purpose string into the associated data, so a value from
one cookie, a CSRF token or `Core\Crypto::seal` does not open as another. Expiry inside the seal is
declined: the cookie's own lifetime and the program's data cover it, and it would change the API. A
program with no `#[Route]` table gets the Origin check on unsafe methods too, the same check a routed
program gets.

**`Core\Config::get` types a secret directive by its name (the user's call on d4, 2026-10-22).** The
checker types `Core\Config::get('<literal>')` as `?secret string` when the literal is a secret
directive (`nvs_config::secret::is_secret_key`), such as `db.main.password` or `http.csrf_key`. A
computed name that turns out to be a secret directive throws `LogicError` at run time. This is a new
checker hook beside the intrinsic roster, one session's work.

**Seeded output is tainted, and a key refuses it (the user's call on d5, 2026-10-22).**
`Core\Random\Seeded::bytes` returns `tainted bytes` and `::token` returns `tainted string`. Every key
parameter refuses a `tainted` argument, so seeded output cannot become a key or a token. No syntax
is added. Tests and examples that seal with a seeded key change to a key from `Core\Random` or a
file.

**Stage 10: the reset undoes everything a request can set.** Postgres adds `SET SESSION AUTHORIZATION
DEFAULT` and `RESET ROLE` to the reset batch; `RESET ALL` does not cover either. `DISCARD ALL` stays
declined, because it drops the statement cache. SQLite gets an authorizer that refuses `ATTACH` and
`DETACH`: an attached path is a file the `fs` grant never checked. Its reset drops every object in
the `temp` schema and applies the opening pragmas again. A `file:` URI in `path` is refused at load,
and options go in their own keys. SQL Server runs a batch as one request and no client can refuse a
second statement reliably, so `rule:security/db-pool-reset-is-a-boundary`'s neighbour rule that says
multi-statement queries are refused is corrected to say the tainted check is the guard on SQL Server.
`quoteIdentifier`'s help says it checks syntax only, and that a name a visitor chooses needs an allow
list.

**Stage 11: a Windows path is judged by its text first.** Before any call that touches the file system,
`within` and every capability check refuse a path starting `\\` or `//` (UNC, `\\?\`, `\\.\`), a
component that ends in a dot or a space, a `:` after the drive letter, and a component with `~` and a
digit (8.3 short names). Then the existing canonical check runs. `remove_file`, `remove_dir`,
`read_dir`, `metadata` and `exists` open beneath a handle the way writes do
(`rule:security/writes-open-beneath-a-handle`).

**Stage 12: development mode is for this machine only.** Static serving in every mode skips dot-files,
`nvs.toml`, `*.nvst` and the database files the `[db]` blocks name, the same set program reads skip.
A development server on a loopback address answers only a `Host` that is `localhost`, a loopback
address, or a name `[[server.mount]]` lists. A static `.svg` or `.html` file gets `Content-Security-Policy:
sandbox`. A hard link out of the root is declined into `rule:http-server/static-serving-is-one-policy`:
a containment check cannot see one, and making the link needs write access to the root.

**Stage 15: serve checks every file it reads, every time, and a read-only mount passes (the user's
calls, 2026-10-22).** `nvs serve`'s boot and every reload resolve through `nvs_config::resolve::Disk`.
`nvs run`, `config check`, `config dump` and the language server keep `Unowned`, as
`rule:config/the-ownership-check-runs-where-it-can-be-answered` says. Development mode is checked too:
the mode comes from the same file, so skipping the check there lets whoever writes the file skip it. On
Unix, a file whose file system is mounted read-only (`statvfs` reports `ST_RDONLY`) passes the owner
test, and so does its directory. No account on this host can change it through that mount, and a
container's `-v ...:/app:ro` is exactly that. A group- or world-writable mode on a read-only mount is
still refused. If Docker Desktop on Windows reports `0777` for a bind mount, the guide tells a Windows
user to copy the configuration into the image, and the rule records why. Windows has no read-only mount
to waive. A checkout on a non-system drive fails there until its inherited `Authenticated Users` write
grant is removed, which the rule already accepts. The repository's own servers must pass on Linux CI and
on a Windows checkout. Safe default: a `bun nv` command that removes that inherited grant from the
checkout once. `E0607`'s fix line names it for a contributor, and CI's Windows jobs run it before the
tests. `benches/compare`'s site run mounts `/app/current` read-write (`tools/nv/cmd/bench-compare.ts:155`),
so its files must belong to the container's account or root.

**Out of scope, each decided by its rule or owned elsewhere:** the slow body wait (HTTP-3, the four idle
waits rule), a disconnect that does not cancel (LC-10), JWE's PBKDF2 ceiling (CRYPTO-4,
`rule:security/jwe-compact-subset`), context-blind template escaping (`rule:core-classes/html-template`),
the multipart header scan (MP-1), extension source authority (`namespace-grants`), and memory and time
isolation between requests on one core (`core-time-slice`).

**Trade-offs.** Isolation gains everywhere above. Latency: stage 3 adds a UTF-8 check where bytes
enter, linear and SIMD-fast; stage 6 adds a budget check per array growth, one compare; stage 10 adds
two commands to a reset already batched in one round trip. Memory: stage 7 and 9 lower it, and nothing
here adds per-request memory. Simplicity: stages 2 and 5 make programs stricter, and every diagnostic
names its fix.

## Stages

### 1. every repro runs again on main, and what is already fixed is struck

**Does:** each finding below is re-run on a fresh debug build of main, and the stages carry only what
still reproduces.

**Split:** five writing-free subagents, one per report (`wb-runtime-memory`, `wb-stdlib-request`,
`wb-sinks`, `wb-http-front` with `wb-routing-static`, `wb-lifecycle`), each re-running its CONFIRMED
repros with `target/debug/nvs.exe` on ports 18100-18199 and answering, per ID, reproduces or not.

- [x] Build debug once (`cargo build`), and run every CONFIRMED repro in `.agent-tmp/sec-audit/` against it. `.agent-tmp/sec-audit/reports/wb-runtime-memory.md:1`
- [x] Strike each item below whose finding no longer reproduces, with the commit that fixed it. Run at fdbedb805: every CONFIRMED finding an item names still reproduces, so nothing is struck. LC-1 no longer reproduces (`040873f47`), and no item names it. `docs/goals/security-hardening.md:1`
- [x] Correct each item's `path:line` anchor that moved since a2b4854a4: `budget.rs:1153`, `serve.rs:1706`, `serve.rs:2373`, `ladder.rs:122` and `body.rs:198`. `docs/goals/security-hardening.md:1`

### 2. a constructor cannot leak `$this` early or run twice, so the JIT's slot reads hold

**Does:** RT-1 and RT-2 no longer compile, and no program reaches a slot the constructor has not
written.

- [x] RT-2: two-phase construction in the init check: every own property is assigned before `$this` escapes or `parent::constructor` runs (`rule:classes/definite-property-initialization`). `crates/nvs-types/src/ctor_init.rs:73`
  - [x] `InitState::escape` refuses `$this`, a closure that captures it, `parent::`/`self::`/`static::` instance calls and a `throw` that carries it, while an own property or an owed parent call is missing
  - [x] `parent::constructor(...)` is refused while an own required property is unassigned
  - [x] unit tests `this_used_*` and `a_subclass_assigns_its_own_properties_before_the_parent_constructor` pass
  - [x] the rule and E0409's card say it, and the four programs the sweep found are ported
- [x] RT-2 through the property pipeline: a constructor's write runs `onPropertySet`, a guard or a hook on `$this` while later slots are zero, and that still panics (`.agent-tmp/sec-audit/wb-runtime-memory/rt2-observer.nvs`). Refusing these accesses at compile time would refuse every observer class that assigns two properties, so it takes the standing safe default: a tagged read of a required slot in a class that implements `PropertyObserver`, `PropertyGuard` or `LateInit`, or declares a hook, and a promoted parameter's pipeline store counts too. `crates/nvs-types/src/expr_table.rs:1513`
  - [x] `record_pipeline_slots` in `crates/nvs-types/src/check.rs` records the armed slots per class and the guarded reads under the class and each ancestor
  - [x] `property_defaults` in `crates/nvs-ir/src/lower/mod.rs` arms them `Unset`, and the property read and the delegation forward test them through `read_may_find_unset`
  - [x] the repro, a promoted pair, an inherited property, a hook and a guard each throw a catchable `LogicError`
  - [x] `rule:classes/definite-property-initialization` and `rule:classes/an-unwritten-property-read-throws` say it, with the cost
- [x] RT-1: `constructor` is callable only through `new` and as one `parent::constructor(...)` statement per path (`rule:classes/constructor-is-a-method-named-constructor`). `crates/nvs-types/src/expr/calls.rs:1040`
  - [x] `$x->constructor(...)`, a class-side call, a reference outside `Core\Attributes`' target, and `parent::constructor(...)` outside a constructor's own body are `E0885`
  - [x] unit test `a_constructor_called_by_name_outside_parent_is_refused` passes, and `t5.nvs` no longer compiles
- [x] RT-1: a second `parent::constructor(...)` on a path does not compile. `crates/nvs-types/src/expr/assign.rs:1333`
  - [x] tracked in `InitState`'s may-have-run set under `PARENT_CALL`, a loop counting too; `t14.nvs` no longer compiles
- [x] RT-1: a dynamic call named `constructor` through `mixed` throws `Error`. `crates/nvs-codegen/src/emit.rs:3018`
  - [x] `call_erased_method_from` throws a `LogicError` (the class every erased-call refusal throws), and `construct_erased_from` keeps the unchecked path
- [x] Port every program in the corpus the new rule refuses (`bun nv brief --where constructor` lists the examples; an `Explore` subagent lists the failing ones from one `nvs check` sweep). `docs/examples/README.md:1`
  - [x] one `nvs check` sweep of the examples, attacks, benches, snippets and apps: nothing to port
  - [x] eight conformance cases ported to the two-phase order, and the conformance tree is green
  - [x] the observer, guard, hook and `LateInit` proofs run green
- [x] The attacks: RT-2's `?restore=1` server shape and RT-1's readonly rewrite, each as a conformance reject case and a hostile case. `tests/hostile/README.md:1`
  - [x] `tests/conformance/reject/a-constructor-runs-only-through-new-and-parent.nvst` and `this-does-not-leave-a-constructor-before-its-properties-are-set.nvst`
  - [x] `tests/hostile/lang/classes/declaring-a-class/02-a-constructor-called-again-by-name.nvs` and `propertyobserver/02-an-observer-reads-a-property-before-the-constructor-sets-it.nvs`

### 3. every string made from outside bytes is checked UTF-8

**Does:** RT-3's two-byte CSV throws an error for that request, and no other unchecked path is left.

- [x] RT-3: `Core\Csv::rows` validates each field and throws `ParseError` with the line; `::parse` needs no check, and its docs say why (`rule:types/string-is-utf8`). `0259de321`. `crates/nvs-stdlib/src/csv.rs:1000`
- [x] RT-3: `Core\Jwe::decrypt` validates its plaintext, with a Rust attack beside it. `69dcbb666`. `crates/nvs-stdlib/src/jwe.rs:1034`
- [x] The lint, first half: `from_utf8_unchecked` in its three forms is in `clippy.toml`'s `disallowed-methods`, and the three audited sites carry an `expect`. `7141bbf8d`. `clippy.toml:1`
- [x] The lint, second half: `Value::str` joins the list, and `Value::text(&str)` is added (the standing decision *Stage 3's second half*). `38c110098`. `crates/nvs-runtime/src/value.rs:341`
  - [x] `Value::text(&str)` in `value.rs`, with a doc line, and `Value::str`'s doc says it checks nothing
  - [x] `nvs_runtime::value::Value::str` in `clippy.toml`; confirm clippy names a site in `nvs-stdlib` (touch `src/lib.rs` first, since clippy does not see a `clippy.toml` change)
  - [x] the mechanical sweep: 766 sites became `Value::text(...)`, by a paren-balanced script and the compiler, not subagents
- [x] Fix every other site the lint names: 128 were left, each now checked or under an `expect` that says why. `Core\SignedCookie::open`, `Core\Session::getSecret` and `Core\Cache::getSecret` gained a check. `38c110098`
- [x] The five sites still under `#[allow(clippy::disallowed_methods, reason = "unchecked: ...")]` are checked (`grep -rn '"unchecked: ' crates/` finds none). `4036dd605`, `53101d858`. `crates/nvs-stdlib/src/out.rs:205`
  - [x] MySQL column labels: `mysql_described_columns` returns a `Result` and throws the `IoError` a non-UTF-8 text value already throws (`nvs_db::mysql::text`); the row keys at `db/execute.rs:834` and `db/stream.rs:344` get the same check. Rust test beside it. `crates/nvs-stdlib/src/db/column.rs:58`
  - [x] captured output that holds raw bytes from `Core\Response::bytes` or `::sendFile`: `crate::out::carried` (`Core\Out::capture`, `spawn script`'s `output`), `Core\Html::later` run at once (`html.rs:705`) and a `later` slot's `render` (`crates/nvs-runtime/src/later.rs:621`). Follow the decision filed for it; until it is answered, the safe default is a `RuntimeError` naming the member when the captured output is not UTF-8
  - [x] `Core\Test\Response`'s body (`crates/nvs-stdlib/src/test.rs:3040`) from a route that answers with `Core\Response::bytes`: the same decision
  - [x] an attack under `tests/hostile/` for the capture: `Core\Out::capture` around `Core\Response::bytes` of `FF`, then the program runs on. `tests/hostile/core/Out/capture/02-bytes-that-are-not-text.nvs:7`
- [x] The attack: an upload of `FF 0A` and of `C0 BC`, then the program runs on. Before the fix, the release binary panicked in `unicode-segmentation` at step 6. After it, steps 6 and 7 each throw a `ParseError`, and the program runs to its end. `tests/hostile/core/Csv/rows/01-a-file-an-upload-form-could-send.nvs:67`

### 4. one test walks every Core signature for a lost mark, and `Core\Arr` and `Core\Str` keep it

**Does:** the keystone test exists and is green, and the `Core\Arr` and `Core\Str` members it named
carry the mark.

- [x] The keystone test over every `Core` signature, with the launderer allow list (`rule:security/taint-propagation`). `crates/nvs-stdlib/tests/taint_signatures.rs:1`
- [x] DB-1: `Core\Str::format`'s `%s` arguments make the result tainted. `crates/nvs-stdlib/src/str.rs:632`
- [x] TAINT-1, DB-3: `keys`, `firstKey`, `lastKey`, `keyOf`, `findKey` and the key in a `map` callback keep the mark. `crates/nvs-stdlib/src/arr.rs:304`
- [x] REQ-1: `Core\Arr::shapeAs<T>` fills a field from a tainted value only if the field is `tainted`. `crates/nvs-types/src/expr/quals.rs:@converts_outside_text`
- [x] TAINT-5: each Low item in the sinks report that the keystone test names. It names none, since none is a signature that drops a mark, and stage 13 owns each one. `.agent-tmp/sec-audit/reports/wb-sinks.md:191`
- [x] The attacks: `sprintf`-style and key-name SQL injection do not compile. `tests/conformance/reject/every-sink-a-marked-value-can-reach-refuses-it.nvst:23`

### 5. signed cookies, streams, database rows and extension calls keep the mark

**Does:** the rest of the keystone test's list is fixed, and stored injection does not compile.

- [x] CRYPTO-1, TAINT-3: `SignedCookie::open` returns `tainted string`, and the module doc says so. `crates/nvs-stdlib/src/signed_cookie.rs:149`
  - [x] the row answers `tainted string`, its `$cookie` is neutral, and the module doc, card and roster tests say so
  - [x] `rule:security/verification-does-not-launder` loses its exception and declines it
  - [x] a contagious member answering `mixed` admits `tainted` text, so `Core\Json::decode` of an opened cookie compiles
  - [x] every `.nvs` and `.nvst` calling `open` compiles, and the reject case is written and green
- [x] TAINT-2: `Core\Response\Stream::write` is a sink and refuses `tainted`. `crates/nvs-stdlib/src/response.rs:603`
  - [x] both arms of the chunk union are `Qual::Sink`, and the card and the rule say why
  - [x] every `.nvs` that streams compiles; the hostile case escapes its query value first
  - [x] the reject case is written and green
- [x] REQ-2, DB-2: a text column fills only a `tainted string` property, and every example, page and app compiles (`rule:security/tainted-sources`). `crates/nvs-types/src/derive.rs:1466`
  - [x] `written_class_of` treats a `Core\Db` owner as outside text, for a class and a shape
  - [x] `DecodeSite` carries its format, and a field no column maps gets no second report
  - [x] the reject case is written and green, and both repros are refused
  - [x] every `.nvs` and `.nvst` calling `queryAs<`/`streamAs<` compiles, and the rule, card and pages say so
- [x] EXT-4: a `secret` value does not cross an extension's `mixed` parameter (`rule:security/secret-does-not-cross-an-extension`). `crates/nvs-types/src/ext_lib.rs:304`
  - [x] every argument an extension parameter admits is asked for `secret`, and a literal at `mixed` is walked as written
  - [x] the reject case covers `mixed`, alone and inside arrays, and is green
- [x] TAINT-4: `Core\Ldap\Connection::add` refuses attribute names from a tainted array's keys. `crates/nvs-stdlib/src/ldap/registry.rs:464`
  - [x] a call-site rule reads a literal's keys one by one, and any other array by `keys_from_outside`
  - [x] the rule, the card and a reject case say so, and every `Core\Ldap` example compiles
- [x] HTML-1: output a spawned child or a capture wrote through `Core\Response::text`, `::json`, `::bytes` or `::stream` takes the sink's rendering before it becomes the carrier, so the parent's page shows it as text, and a child's `echo` is still escaped once, not twice. Rendering at the write keeps `output`'s type and covers `output: 'inherit'` too, where a plain `string` result would not (`rule:security/capture-answers-the-carrier`, `rule:security/isolate-output-is-captured`). `crates/nvs-runtime/src/ctx/output.rs:324`
  - [x] `Ctx::write_body` escapes (HTML) or substitutes (terminal) at `script_depth > 0` or inside a capture, byte by byte for bytes that are not UTF-8, and the four body members write through it
  - [x] two Rust tests pin both sinks, and the rule, `response.rs`'s module doc and `carried`'s doc say so
- [x] HTML-1's attack: a child writes `<script>` and an `ESC` sequence with `Core\Response::text`, the parent reads `$result->output`, and the text has no control byte; a child that `echo`es the same text gives the same text; a capture of the parent's own `text` does too. It fails on the binary from before the fix. The HTML side is the Rust test, because the hostile harness runs on the terminal sink. `tests/hostile/lang/concurrency/isolates-spawn-script-and-await/02-a-child-writes-markup-the-parent-prints.nvs:1`
- [x] The attacks: stored injection, a sealed visitor value, a stream echo and a `mixed` secret, each a reject case (`tests/conformance/reject/a-text-column-fills-only-a-tainted-property.nvst`, `tests/conformance/reject/a-sealed-visitor-value-opens-tainted.nvst`, `tests/conformance/reject/a-response-stream-part-refuses-a-tainted-value.nvst`, `tests/conformance/reject/a-secret-argument-to-an-extension-does-not-compile.nvst`). `tests/conformance/reject/a-sealed-visitor-value-opens-tainted.nvst:1`

### 6. an array a Rust parser builds stops at the request's memory ceiling

**Does:** an 8 MB body stops at the 128 MiB ceiling inside the parse, in each of the four parsers.

- [x] PARSE-1: `Core\Csv::parse` and `::rows` grow arrays through the fallible form. `crates/nvs-stdlib/src/csv.rs:715`
- [x] PARSE-2: `Core\Json::decode` and `Core\Request::json()`. `crates/nvs-stdlib/src/json.rs:1807`
- [x] PARSE-3: `Core\Xml::parse` charges attribute and name growth. `crates/nvs-stdlib/src/xml.rs:1619`
- [x] RT-4: `Core\Html::parse` refuses at the ceiling, not after it. `crates/nvs-runtime/src/budget.rs:1153`
- [x] The attacks: one 8 MB body per parser, the peak stays under the ceiling, and the next request answers. `tests/hostile/core/Csv/parse/01-documents-an-upload-form-could-send.nvs:1`

### 7. connections, spans, metrics and blocking work grow with requests in flight, never with requests served

**Does:** HTTP-1's idle connections and slow readers no longer flip admission, a server pushed past
capacity recovers every place it gave out, and LC-3, LC-5, LC-6, LC-7 and LC-9 are closed.

- [x] HTTP-1: set hyper's buffer size, and keep a buffered response's in-flight place until its body is written. `crates/nvs-server/src/serve.rs:2373`
- [x] HTTP-1: admission reads memory charged to requests, not the thread's total (`rule:http-server/admission-is-arithmetic-not-a-number`). `crates/nvs-server/src/admit.rs:1`
- [x] LC-3, LC-9: no span queued without an exporter, the client's flag read only with one, and `request_id` always the server's (`rule:observability/sampling-is-head-based`). `crates/nvs-server/src/trace.rs:278`
- [x] LC-6, LC-7: the `method` label is closed, and a 501 logs at `info`. `crates/nvs-server/src/serve.rs:1706`
- [x] LC-5: blocking-pool CPU is charged to its request (`rule:http-server/a-core-is-never-blocked-on-a-syscall`). `crates/nvs-host/src/blocking.rs:349`
- [x] ~~LC-8: try the native stack overflow shapes in the lifecycle report; if the process ends, the deep helper checks the stack it has left and throws.~~ Ruled out on a2b4854a4: every native walk over a runtime value is iterative or depth-bounded, JSON decode's 1024-deep recursion runs on the 8 MiB spare stack, and each shape answered a clean `500` with the worker still serving (LC-8 in the lifecycle report). `.agent-tmp/sec-audit/reports/wb-lifecycle.md:1`
- [x] LC-12: a configured `[log]` escalation handler runs on a task of its own behind a bounded queue, never joined on the serving worker; a full queue drops the event and counts it in a metric, so forced uncaught throws cannot each hold a worker for `handler_reserve_time`. The queue's length is a constant with its reason in the code (`rule:http-server/a-core-is-never-blocked-on-a-syscall`). `crates/nvs-host/src/ladder.rs:122`
- [x] HTTP-1's fix keeps a Rust test beside `admit.rs`: a core thread holding connection buffers past the room still admits a request, because admission reads request memory only. No idle-connection or slow-reader attack, since a proxy stops those first. `crates/nvs-server/src/admit.rs:1`
- [x] The overload attack, a Rust test that starts the real `nvs serve` with `max_in_flight = 50` and a small `[limits] memory`. A flood several times the ceiling runs beside requests that reach the memory limit, reach the CPU limit, and lose their client, while a probe sends ordinary requests. Every refusal is a `503` with `Retry-After`, every probe gets `200` or that `503` and never hangs, and the process lives. Once the flood ends, 50 requests held open together are all admitted, `nvs_memory_bytes` is back within a stated bound of its reading before the flood, and the next request answers `200`. `crates/nvs-cli/tests/serve_isolation.rs:1`
- [x] A `flood` row in `bench-compare --scenarios` puts load past capacity on each stack behind nginx and reports refusals, probe latency and recovery; the nightly deep lane runs its smoke form. `benches/compare/README.md:80`
- [x] d2: an admitted request holds a provisional share of the room until its first publish, so a burst past the room gets `503` before it allocates; the size of the share and its cost are in `rule:http-server/admission-is-arithmetic-not-a-number`, a Rust test beside `admit.rs` sends a burst on one core and sees `503`, and the `flood` row run once shows Novis's holds refused with `503`, not stopped with `500`. `crates/nvs-server/src/admit.rs:1`
- [x] `[limits] max_tasks`'s comment says what `claim_task` enforces, not "bounds nothing today". `crates/nvs-config/src/default.toml:86`

### 8. an extension instance ends with its request

**Does:** EXT-1's 70 requests all answer, and EXT-2, EXT-5 and EXT-6 are closed.

- [x] EXT-1: the tree's `Calls` are dropped when the tree ends, and the instance with them. `crates/nvs-cli/src/extensions.rs:333`
- [x] EXT-2: the instance limit comes from `max_in_flight`. `crates/nvs-ext/src/call.rs:1170`
- [x] EXT-5: a narrowed `grants:` reaches an instance the tree already holds. `crates/nvs-cli/src/extensions.rs:442`
  - [x] the attack: a served request calls `Geo\Atlas`, then three child scripts without `fs.read` call it (red on main)
  - [x] `Calls` keeps one request per narrowing, and the attack is green
  - [x] both rules say so
- [x] EXT-6: a redirect hop is checked against the extension's intersection (`rule:security/extension-grants-are-an-intersection`). `crates/nvs-ext/src/wasi/http.rs:488`
  - [x] the attack: a granted host redirects the guest to `127.0.0.2`, which only the caller may reach (red on main)
  - [x] `Outgoing::connect` carries the set and `extension_send` refuses the hop, and the attack is green
  - [x] the rule says so
- [x] EXT-7 to EXT-10: each Low item in the sinks report, fixed or declined into its rule. `.agent-tmp/sec-audit/reports/wb-sinks.md:212`
  - [x] EXT-7: a manifest that requests any I/O makes every method a source (`rule:security/extension-declares-sink-or-source`). `crates/nvs-ext/src/manifest.rs:439`
  - [x] EXT-8: a set where two extensions declare one `[ext.<name>]` block is refused (`rule:packaging/a-guest-has-no-ambient-authority`). `crates/nvs-ext/src/load.rs:194`
  - [x] EXT-9: random bytes and `as-bytes` ask what the request has left first (`rule:packaging/a-guest-runs-under-the-requests-budget`). `crates/nvs-ext/src/call.rs:1403`
  - [x] EXT-10: the WASI open, create and rename go through the beneath walk (`rule:security/writes-open-beneath-a-handle`); the device names move to stage 11. `crates/nvs-ext/src/wasi/files.rs:212`
- [x] The attack: 200 requests that each call `Novis\Intl`, every answer correct, memory flat; a Rust test in `nvs-cli`. `crates/nvs-cli/tests/serve_isolation.rs:1`

### 9. sessions, the CSRF door and signed cookies read and bind one way

**Does:** CSRF-1, CSRF-2, SESS-1 and CRYPTO-2 are closed, and CRYPTO-3 and CRYPTO-6 are fixed.

- [x] CSRF-1, RS-3: the door reads cookies with `Core\Session`'s reader. `crates/nvs-server/src/route.rs:315`
- [x] CSRF-2: a program with no route table gets the Origin check (`rule:security/csrf-is-on-by-default`). `crates/nvs-server/src/route.rs:124`
- [x] SESS-1: `Session::start` writes no row and sends no cookie until the first write (`rule:core-classes/session-is-started-explicitly`). `crates/nvs-stdlib/src/session.rs:912`
- [x] CRYPTO-2: signed cookies bind the name and a purpose into the associated data. `crates/nvs-stdlib/src/signed_cookie.rs:261`
- [x] CRYPTO-3: `Core\Config::get` and `::all` return a secret directive as `secret string`. `crates/nvs-stdlib/src/config.rs:192`
- [x] CRYPTO-6: `Core\Random\Seeded` output is refused where a key or a token goes. `crates/nvs-stdlib/src/random.rs:395`
- [x] BBK-1's pin: a token issued for no session, sent with no cookie, is refused same-origin and cross-origin by a keyed door. It passed on main (the claim that a2b4854a4 refused it was wrong), and the attack keeps it so once CSRF-1 changes how the door reads cookies (`rule:security/csrf-is-on-by-default`). `crates/nvs-server/src/route.rs:245`
- [x] The attacks: a session cookie on a second line, a cookie swapped between names, and 20,000 cookieless starts that leave no rows. `tests/hostile/core/Session/start/01-identifiers-nobody-issued.nvs:1`

### 10. a pooled database connection starts clean, and SQLite opens only what the grant names

**Does:** CVE-1, SINK-2, DB-4 and DB-5 are closed.

- [x] CVE-1: Postgres resets the role and session authorization, and the comment says what `RESET ALL` does not cover (`rule:security/db-pool-reset-is-a-boundary`). `crates/nvs-db/src/pg.rs:3427`
- [x] CVE-1: correct the unit test that asserts the old mapping. `crates/nvs-db/src/pg.rs:6030`
- [x] CVE-1: SQLite's authorizer refuses `ATTACH` and `DETACH`, and its reset drops `temp` objects and applies the opening pragmas. `crates/nvs-db/src/sqlite.rs:1347`
- [x] SINK-2: a `file:` URI in `path` is refused at load. `crates/nvs-stdlib/src/db/open.rs:930`
- [x] DB-4, DB-5: the SQL Server rule text and `quoteIdentifier`'s help. `crates/nvs-db/src/tds/rows.rs:446`
- [x] The attacks: `SET ROLE` then a second request, a `temp` table and an `ATTACH` then a second request; Rust tests in `tests/pool_reuse.rs`, the Postgres one where the existing Postgres tests run. The SQLite ones are unit tests in `sqlite.rs`, over a real engine. `crates/nvs-db/tests/pool_reuse.rs:1`

### 11. a Windows path is refused by its text before the file system sees it

**Does:** SINK-1 sends no packet, SINK-3's aliases are refused, and CVE-2's checked-then-reopened
operations open beneath a handle.

- [x] SINK-1, SINK-3: the textual refusal runs before `canonicalize` (`rule:security/path-scope-canonicalise-then-prefix`). `crates/nvs-runtime/src/capability.rs:1159`
- [x] SINK-3: `Core\IO::within` returns no alias. `crates/nvs-stdlib/src/io.rs:3688`
- [x] EXT-10's names: the WASI filesystem's `refused_name` refuses the same Windows text, DOS device names (`CON`, `NUL`, `COM1`, `LPT1`, with or without an extension) among it. `crates/nvs-ext/src/wasi/files.rs:253`
- [x] CVE-2: `remove_file`, `remove_dir`, `read_dir`, `metadata` and `exists` open beneath a handle (`rule:security/writes-open-beneath-a-handle`), and so do the WASI layer's `unlink-file-at`, `remove-directory-at`, `stat-at` and `read-directory`, through a `Budget` method each beside `open_file`. `crates/nvs-runtime/src/capability.rs:950`
- [x] The attacks: a UNC, `\\?\`, `\\.\` and 8.3 path through `within`, refused with no network attempt; a Rust test that times the refusal. `tests/hostile/core/IO/within/01-names-built-to-escape.nvs:1`

### 12. development serving answers only loopback and serves no project file, and the HTTP front's small gaps are closed

**Does:** RS-1, RS-2, RS-4, RS-6, RS-7, HTTP-2, HTTP-4 and HTTP-5 are closed, and RS-5 is declined
into its rule.

- [x] RS-1: static serving skips the set program reads skip, in every mode (`rule:http-server/static-serving-is-one-policy`). `crates/nvs-server/src/mount.rs:571`
- [x] RS-2: a loopback development server answers only a loopback `Host` or a listed one. `crates/nvs-server/src/mount.rs:307`
- [x] RS-4, RS-6, RS-7: the case rule through a link, a strict `Range` parse, and `sandbox` on static SVG and HTML. `crates/nvs-server/src/statics.rs:239`
- [x] HTTP-2: an absolute-form target's host wins over `Host` (`rule:http-server/host-matching-is-on-the-host-part-only`). `crates/nvs-server/src/mount.rs:424`
- [x] HTTP-4, APP-3: a malformed chunked body answers `400`, and so does a request that carries both `Content-Length` and `Transfer-Encoding` (it is framed by `Transfer-Encoding` and closed today). `crates/nvs-server/src/body.rs:198`
- [x] APP-2: when a body reader trips `[limits] request_body`, the runtime records `Refusal::BodyTooLarge` on the request as it throws, and `dispatch` sends a handler that throws after that to the gate's `refused` with `413` instead of `failed` with `500`. No new error class: the reader still throws its `RuntimeError` (`rule:http-server/the-body-is-read-on-demand-under-two-caps`, `rule:routing/a-gate-answers-around-every-handler`). `crates/nvs-stdlib/src/request.rs:679`, `crates/nvs-stdlib/src/router.rs:2523`
- [x] APP-2's attack: a 9 MB JSON body and a 9 MB form body to a route that reads them answer `413` through the gate, and the next request answers `200`. `tests/hostile/core/Request/jsonAs/02-a-body-over-the-limit.nvs:1`
- [x] HTTP-5: `X-Forwarded-Proto` from a trusted peer that never set it is ignored (`rule:http-server/x-forwarded-proto-sets-the-scheme-and-hsts-only`). `crates/nvs-server/src/forwarded.rs:466`
- [x] RS-5: the Declined entry for a hard link. `docs/rules/http-server/static-serving-is-one-policy.md:1`
- [x] The attacks: `GET /nvs.toml`, `/.env` and `/data.sqlite` in development mode, a rebinding `Host`, and the static policy rows; Rust tests beside `mount.rs`. `crates/nvs-server/src/mount.rs:1586`

### 13. the outbound client's redirects, the leftovers, and the audit's scratch is gone

**Does:** the client's redirects follow RFC 9110, the remaining small items are fixed, and
`.agent-tmp/sec-audit/` is deleted.

- [x] A 303 becomes a `GET`, and a 301 or 302 after a `POST` becomes a `GET`; a cross-origin hop drops the body (`rule:http-server/a-cross-origin-redirect-drops-credentials`). `crates/nvs-stdlib/src/http/transport.rs:2361`
- [x] NET-1: the two IPv6 forms that carry private addresses are denied (`rule:security/net-address-policy`). `crates/nvs-config/src/capability.rs:502`
- [x] Correct the comment that says an unquoted attribute is safe. `crates/nvs-stdlib/src/html.rs:119`
- [x] HTML-2: `Core\Html::sanitize` drops a URL attribute whose value, after the tab, newline and carriage-return strip, starts with `//`, `\\`, `/\` or `\/`; a browser reads a backslash as a slash, so each names another host. The `addressable` doc and the rule say a relative reference never names a host (`rule:core-classes/html-sanitize`). `crates/nvs-stdlib/src/html.rs:1847`
- [x] HTML-2's cases join the sanitizer's tests: the four forms are dropped, and `/path`, `?q` and `#frag` are kept. `crates/nvs-stdlib/src/html.rs:1847`
- [x] Raise `wasmtime` and `cranelift`'s minimums to the fixed releases in the lock file. `Cargo.toml:98`
- [x] Run the HTML-escaping probes the audit never reported on, and fix or record each result here — run, recorded as HTML-1 (stage 5) and HTML-2 above. The audit's `wb-html.md` ran every probe: `app.nvs`'s quoted holes are escaped, and its unquoted, `on*`, `<script>`, `<style>`, tag-name and `javascript:` holes are the blind spot `rule:core-classes/html-template` declines; `p1`–`p3` (forging a `Markup`) do not compile; `p6` (the mXSS corpus) kept only `//host`, which is HTML-2; `cap.nvs` answers `text/plain` or JSON with `nosniff`; `parent`/`child` is HTML-1. `.agent-tmp/sec-audit/wb-stdlib-request/html/app.nvs:1`
- [x] TAINT-5: `$m[$t] = …` with a tainted key makes the keys of `$m` tainted, as `keys_from_outside` reads them at a `foreach`. `crates/nvs-types/src/expr/assign.rs:1062`
- [x] TAINT-5: `Core\Response::redirect` refuses a relative target that names another host (`//x`, `/\x`, `\/x`), so a launderer's plain answer cannot open a redirect. `crates/nvs-stdlib/src/response.rs:1473`
- [x] TAINT-5: the regex replacement template is a sink and refuses `tainted` (`rule:security/every-grammar-is-a-sink`). `crates/nvs-stdlib/src/regex.rs:171`
- [x] TAINT-5: `Core\Xml`'s writer `instruction` refuses a tainted target and data. `crates/nvs-stdlib/src/xml.rs:760`
- [x] TAINT-5: `setHeader` refuses `Content-Length`, `Transfer-Encoding`, `Connection` and `Upgrade` as it refuses `Content-Type`; probe what hyper sends first. `crates/nvs-stdlib/src/response.rs:433`
- [x] TAINT-5: `Core\Mail` writes `To:` and `Cc:` in angle brackets, `address_of` refuses `( " : ; %`, and a long subject is folded. `crates/nvs-stdlib/src/mail.rs:453`
- [x] TAINT-5: `Core\Db::quoteIdentifier` refuses the niladic SQL names (`current_user`, `session_user`, `user`, …). `crates/nvs-db/src/schema.rs:93`
- [x] TAINT-5: file and `Core\Storage` reads are written into `rule:security/tainted-sources`, as tainted or declined with the reason. Safe default: `Core\Storage` reads are tainted, and a file the program ships is declined. `crates/nvs-stdlib/src/storage.rs:202`
- [x] Delete `.agent-tmp/sec-audit/`. `.agent-tmp/sec-audit/BRIEF.md:1`

### 15. `nvs serve` and every reload check who owns the configuration, and a read-only mount passes

**Does:** a served `nvs.toml`, include or secret file that another account can write stops the boot
with `E0607`, and a reload that finds one keeps the running snapshot and names the path. The Docker
smoke test's read-only bind mount still serves.

**Split:** one read-only `Explore` subagent lists every place the repository boots `nvs serve` on a file
in the checkout (the harnesses under `tools/nv/`, `tests/docker/smoke.sh`, the compose files under
`benches/`, the apps and the website's snippets), as `file:line` and one line each. The session makes
the code change before it runs any of them.

- [x] `nvs serve`'s boot and every reload resolve through `Disk`. Every other reader keeps `Unowned` (`rule:config/the-ownership-check-runs-where-it-can-be-answered`). `crates/nvs-cli/src/config.rs:400`, `crates/nvs-cli/src/config.rs:575`, `crates/nvs-cli/src/reload.rs:370`, `crates/nvs-cli/src/reload.rs:414`, `crates/nvs-cli/src/serve.rs:429`, `crates/nvs-cli/src/serve.rs:1982`
- [x] The `LocalFiles` doc and the module doc say which reader each command uses, as it now is. `crates/nvs-cli/src/config.rs:25`, `crates/nvs-cli/src/config.rs:109`
- [x] On Unix, `trust::check` waives the owner test for a file and a directory on a read-only mount, and still refuses a group- or world-writable mode (`rule:config/ownership-is-the-trust-boundary`). `crates/nvs-config/src/trust.rs:184`
- [x] Rust tests: a group-writable root and a group-writable include each stop `nvs serve` with `E0607` naming the path; a reload after a `chmod g+w` keeps serving and names the path; the read-only waiver passes and the writable mode under it does not. Each is named in its rule's `guardedBy`. `crates/nvs-cli/tests/serve_ownership.rs:1`, `crates/nvs-config/tests/trust.rs:1`, `data/rules/config/the-ownership-check-runs-where-it-can-be-answered.json:9`
- [x] The rule gains the read-only mount paragraph, its reason, and what Docker Desktop on Windows reports. `docs/rules/config/ownership-is-the-trust-boundary.md:1`
- [x] Every repository server from the subagent's list passes on Linux and on a Windows checkout, through the safe default's `bun nv` command where Windows needs it, and CI's Windows jobs run it. `tools/nv/main.ts:1`, `.github/workflows/ci.yml:829`
  - [x] `bun nv checkout-acl` exists (`tools/nv/cmd/checkout-acl.ts`), and CI's four Windows jobs with Bun run it after the tool install
  - [x] it finished on this checkout (`icacls E:\mwl` shows no `(I)` entry and no `Authenticated Users` grant)
  - [x] on Windows: `bun nv site --check` over the snippets and the apps, and one served Rust test under `target/tmp` (`cargo test --test per_mount_config`), pass. No `E0607` anywhere; the snippets and `apps/notes` pass after b4ee8866d and da6b5cb72; `apps/paperwork`'s one red is d6, not ownership
  - [x] `apps/paperwork`'s `eachDocumentIsSentAsAPdf` passes, by d6's option 1 (f70139ab1): 18 of 18, and `bun nv site --check snippets apps --nvs target/debug/nvs.exe` is green
  - [x] on Linux: not runnable here, so stage 14's first item reads CI's `docker` job (smoke `:ro` mount) and `bench-compare --smoke`
- [x] The Docker guide says a read-only mount passes, and a writable one must belong to uid 65532 or root. `website/src/content/docs/guides/production/docker.mdx:59`

### 14. the goal is verified once, and every failure is fixed

- [ ] CI's Linux `docker` job (`tests/docker/smoke.sh:52`, the `:ro` mount) and `bun nv bench-compare --smoke` pass with no `E0607`; stage 15 could not run them on Windows.
- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stage 15 is done. The gate stage 14 is next.

- **d6, option 1 (f70139ab1).** `Core\Test::request` keeps the body slot as `bytes`
  (`Value::bytes`, and `b""` for a failed request), so it no longer throws for a PDF body.
  `body_text` in `crates/nvs-stdlib/src/test.rs` is the one UTF-8 check: `body` returns
  `Value::text` of it, and `response_text` (`json`, `jsonAs`) calls it before the empty-body
  `ParseError`. Each throws `RuntimeError` "`Core\Test\Response::<member>(): the response body is
  not valid UTF-8 at byte N, ...`". The attack
  `a_body_that_is_not_utf8_reads_its_status_and_only_its_text_readers_throw` was red on main
  (`request` threw) and passes, with the two neighbouring tests. The `body`, `json`, `jsonAs` and
  `request` cards and `rule:types/string-is-utf8` say so, and so does the d1 standing decision.
  `apps/paperwork` is 18/18; `bun nv site --check snippets apps --nvs target/debug/nvs.exe` is
  green with nothing to bless.
- **A test for da6b5cb72 (707419177).** `every_core_nvs_test_starts_calls_a_built_in_extension`
  in `crates/nvs-cli/tests/test_command.rs`: eight `Novis\Intl\Collator` tests pass under
  `--jobs 4` and `--jobs 1`. It was not run against the commit before da6b5cb72. The budget half
  (two tests each spending most of an extension CPU budget under `--jobs 1`) is not written: no
  fixture guest spins (`tests/conformance/ext/fixtures/ledger` has echo methods only). See the
  backlog.
- d6 is still listed open in `looper decision list`. It was built on option 1, so the gate
  session closes it with `bun tools/looper/main.ts decision close d6` before `DONE`.
- `cargo build`, `cargo clippy --all-targets -- -D warnings`, `bun nv rules --render`/`--check`
  are green.
- The sweep's `env-secret-proofs` red is the 00:07 log again, and the nightly run 39746060276 is
  the cancelled `workflow_dispatch`. Neither is the tree's.

Tests that may go red from this session: a `.nvst` or test that expects `Core\Test::request` to
throw for a body that is not UTF-8 (grep found the old message only in `out.rs`, which is
`Core\Out::capture`'s); a Rust test that reads `Core\Test\Response`'s body slot with `as_text`
(the two in `test.rs` are updated); the help snapshots and the website's `core.json` /
`llms-full.txt` for `Core\Test::request` and `Core\Test\Response::body`/`json`/`jsonAs`; the
website's rule page for `string-is-utf8`.

The session before (Windows run):

Stage 15's Windows run is done, and no repository server hits `E0607`. It found two real bugs,
both fixed, and one design gap, filed as d6 and built on.

- **Served bodies were HTML-escaped (b4ee8866d).** HTML-1's `Ctx::write_body` rendered a body
  whenever `script_depth > 0`, and `nvs-host` builds every request, job and schedule run as
  `ctx.isolate(..)`, so at depth 1: every served `Core\Response::text`/`::json` body went out
  escaped (`{&quot;id&quot;:2}`). It now renders only for `RunKind::Isolate` (a spawn child, a
  method entry) or an open capture. Unit attack
  `a_body_a_host_started_run_writes_is_verbatim_and_its_childs_is_not` was red; all 16
  `ctx::output::tests` pass; the child-markup hostile case still prints its three steps.
- **`nvs test` and extensions (da6b5cb72).** The runner used `--jobs 1` only when
  `config.extension` was non-empty, and only the suite's thread had a host, so a test calling a
  built-in (`Novis\Intl`, `Novis\Pdf`) on a core threw "not loaded in this process". Under
  `--jobs 1` one `Calls` spanned the suite, so extension CPU time added up and later tests hit the
  limit. `extensions::Tested(Arc<Set>)` is `Served` over a fixed set (shared `in_tree`): each
  test's calls live in its own tree. `nvs test` installs it, and `runner::run` /
  `run_suite_in_a_task` take `Option<Arc<Set>>` and install it on each core. The `--jobs` help and
  `rule:testing/tests-run-across-every-core` say so. No Rust test yet (Next 2).
- **Run on Windows:** `cargo test --test per_mount_config` 9/9; `bun nv site --check snippets apps
  --nvs target/debug/nvs.exe`: snippets ok, `apps/notes` ok, `apps/paperwork` 17/18. The red is
  `eachDocumentIsSentAsAPdf`: `Core\Test::request` throws on the PDF body (d1). A
  `#[Test(server: true)]` rewrite was tried and reverted: the test server serves under the default
  tree (`runner.rs`, `TestServer::bind`'s doc), so the app's `fs` grant for its logo is absent and
  the route answers `500`. d6 is filed with option 1 built on.
- `bun nv site --check` with no `--nvs` uses `target/release/nvs.exe` first, which is from 14:32
  and predates most of this goal; its failures then (`SignedCookie` arity) are the stale binary.
- `cargo build`, `cargo clippy --all-targets -- -D warnings`, `bun nv rules --render`/`--check`
  are green.
- The sweep's `env-secret-proofs` red is the 00:07 log again (exit 0, `nothing owed`), and the
  nightly run 39746060276 is the cancelled `workflow_dispatch`. Neither is the tree's.

Tests that may go red from this session: a help snapshot of `nvs test --help` (the `--jobs` text
lost a sentence); a runner test that expected extension calls to share one budget across tests;
any `.nvst` or test that expected a job's or scheduled run's `Core\Response::text` body to be
substituted (it is verbatim again, as before HTML-1); the website's rule page for
`tests-run-across-every-core`.

The session before (stage 15's code):

Stage 15's code, tests, rules, guide and CI step are landed; its last item (every repository server
passes on Windows and Linux) is open. Then the gate stage 14.

- **Serve reads through `Disk`.** `crate::config::ServedFiles` (= `nvs_config::resolve::Disk`) is
  the reader of `boot_set`, `stored_set`, `set_of`, both `Published` calls in `reload.rs`, every
  non-test `Published::new`/`entry`/`folder` call in `serve.rs` (lines 429, 1100, 1103, 1985, 2002,
  2252, 2262, 2308), and the queue job's `configure` in `worker.rs`. `resolved_in` takes the reader;
  `boot_in`/`boot_origins` (`nvs run`, `check`, bundle) keep `LocalFiles`. `serve/mounts.rs`'s
  `Stamping` keeps `LocalFiles`: it enumerates app roots, which are not configuration files.
- **Read-only waiver.** `trust.rs`'s Unix `check` reads a `Stat` and calls `judged(stat, me,
  read_only)`. `read_only_mount` (`statvfs`, `ST_RDONLY`) is asked only of a path another account
  owns. The mode test still runs. Unit tests `another_accounts_file_passes_on_a_read_only_mount_and_not_on_a_writable_one`,
  `a_writable_mode_on_a_read_only_mount_is_still_a_breach` and
  `a_writable_file_system_and_a_missing_path_are_not_read_only` (Unix only, not run; they compiled
  in WSL with `cargo check --tests`, exit 0, which also compiled CVE-2's Unix `beneath.rs` half).
- **Attacks.** `crates/nvs-cli/tests/serve_ownership.rs`: a private tree serves (development mode);
  a writable root and a writable include each stop the boot with `E0607` naming the path; a reload
  after the file is opened to others and saved logs `E0607` naming it and still answers `200`.
  `open_to_others` is `chmod g+w` on Unix and `icacls /grant *S-1-5-11:(M)` on Windows. All four
  pass on Windows. Both rules' `guardedBy` name them.
- **Rules.** `ownership-is-the-trust-boundary` has the read-only paragraph, the Docker Desktop
  `0777` paragraph and two Declined entries. `the-ownership-check-runs-where-it-can-be-answered`
  says serve checks every read in every mode, names `bun nv checkout-acl`, and declines naming it in
  `E0607`'s help: the help is end-user text (AGENTS.md § *Text an end user reads*), so the standing
  decision's "E0607's fix line names it" went to `docs/agent/commands.md` instead.
- **`bun nv checkout-acl`** (`tools/nv/cmd/checkout-acl.ts`): on Windows, when the checkout root
  has an inherited `(I)` entry, `icacls /inheritance:d`, grant this account `(OI)(CI)(F)`, remove
  `*S-1-5-11` and `*S-1-1-0`; `--check` only reports. It calls `whoami.exe` by its System32 path
  (Git Bash's coreutils `whoami` shadows it). CI's `test`, `cost-guards`, `proofs` and
  `ext-templates` jobs run it after `bun install`. **It ran on `E:\mwl`**: the root now grants this
  account, `Administrators` and `SYSTEM` full control and `Users` read, and `nvs.toml` inherits
  that; `--check` says nothing to do. That run removed before it granted, so this account could not
  write for a while (`EPERM` under `.looper/jobs`); the command now grants first.
- **The subagent's list** of checkout servers: `tests/docker/smoke.sh:52` (`:ro`),
  `benches/proxied/compose.nvs.yaml:37` (`:ro`), `benches/compare/compose.nvs.yaml:39` (`:ro`, and
  the deploy rows' `site` volume, written by `deploy.sh` inside the container as uid 65532),
  `tools/nv/cmd/bench-load.ts:256`, `tools/nv/cmd/site.ts:817` (snippets) and `:1534` (apps),
  `tools/nv/cmd/scaling.ts:844`, and `ci.yml:393`'s `live_bytes_stay_flat` test. Every Rust serve
  test writes under `CARGO_TARGET_TMPDIR`, which on a Windows checkout inherits the drive's grant
  until `checkout-acl` ran. None of these was run this session.
- The Docker guide says a `:ro` mount passes, a writable one needs uid 65532 or root, and Docker
  Desktop's `0777` needs `COPY`.
- `cargo build`, `cargo clippy --all-targets -- -D warnings`, `bun nv rules --render`/`--check` and
  `bun nv proofs --comments` on the guide are green.
- The sweep's `env-secret-proofs` red is the 00:07 log again (exit 0, `nothing owed`), and the
  nightly run 39746060276 is the cancelled `workflow_dispatch`. Neither is the tree's.

Tests that may go red from this session: on a Windows checkout whose root still inherits the drive's
grant, every served Rust test under `target/tmp` (`live_config`, `per_mount_config`, `live_edit`,
`serve_isolation`, `request`, `env_context`, `gate`) and every `bun nv` harness that serves a checkout
file, with `E0607`, until `bun nv checkout-acl` has run; on Linux, a served test whose scratch file
is group-writable (umask 002) or owned by another account; `serve.rs`'s in-process tests that call
`boot_set` (lines 3365, and `worker.rs:2351`) on a scratch directory that is not private; the
website's rule pages for the two rules changed.

The session before (Mail, Storage, tainted key):

- **Mail (d33b15bc6).** `compose` writes `To:`/`Cc:` through `address_line` (each address in
  `<...>`, folded between addresses) and the subject through `subject_line`: plain text is folded at
  spaces under 78 columns (`folded`), anything else, or a run past 998 with no space, is RFC 2047
  encoded words of at most 42 UTF-8 bytes, one per line. `address_of` refuses
  `NOT_IN_AN_ADDRESS` (`< > , ; : ( ) " \ %` and a space) and more than 254 bytes. Unit attacks
  `a_long_subject_is_folded_and_unfolds_to_itself` and
  `recipients_are_in_angle_brackets_and_a_long_list_is_folded` were red on main; all 10
  `mail::tests` pass. The hostile case gained step 8; example 02's `.out` has the new message.
  No rule fragment owns `Core\Mail`; the module doc, card and about page do.
- **Storage (63ebf8640).** `Core\Storage::get` returns `?tainted bytes`; `list` stays plain (a key
  is a closed alphabet). `rule:security/tainted-sources` says so and declines a tainted `Core\IO`
  read. Attack `tests/conformance/reject/an-object-read-from-storage-is-tainted-and-its-keys-are-not.nvst`.
  The benches and `get`/`put` examples declare `tainted bytes`; the `Core\Storage` proofs pass.
- **Tainted key (8c3e472cc).** `quals::reject_tainted_key_write` is `E0401` at a `tainted` key
  written into an array whose type `keys_from_outside` reads as plain: from the `ExprKind::Index`
  arm in `expr/mod.rs` at every write-target level (new `Env::unset_target` exempts `unset`), and
  from both literal paths in `literals.rs` against the expected or synthesized type. Attack
  `tests/conformance/reject/a-tainted-key-cannot-enter-an-array-whose-keys-read-back-plain.nvst`.
  A compile sweep of 11142 corpus `.nvs`/`.nvst` files found three examples, rewritten
  (`Db-Row/string/03`, `Http/allowUrl/03`, `SignedCookie/seal/03`); their proofs pass.
  `rule:security/taint-propagation` has the paragraph and a Declined entry.
- **`.agent-tmp/sec-audit/` is deleted**, and the goal's evidence paragraph says so.
- `cargo build`, `cargo clippy --all-targets -- -D warnings`, `bun nv rules --render`/`--check` and
  `looper lint` are green. The sweep's `env-secret-proofs` red is the 00:07 log again (exit 0,
  `nothing owed`), and the nightly run 39746060276 is the cancelled `workflow_dispatch`.

Tests that may go red from this session: a Rust test that compiles source writing a `tainted` key
into a plain-keyed array (not swept; `nvs-types` and `nvs-cli` tests are likeliest), or reading
`Core\Storage::get` into `bytes`; a `.nvst` whose FILE section the sweep's regex missed; the help
snapshots and the website's `core.json`/`llms-full.txt` for `Core\Mail::send` and
`Core\Storage::get`; the website's rule pages for `tainted-sources` and `taint-propagation`; the
`perf: stale` proofs of `Core\Mail::send` and `Core\Storage::get`.

The session before (setHeader, regex, XML, quoteIdentifier):

- **`setHeader` (dfc0696f0).** hyper 1.12's `role.rs` was read: with a known body length it trusts
  a user `Content-Length` (and `assert!`s a mismatch in a debug build); with an unknown one it frames
  the body by it. `Core\Response::setHeader` throws `LogicError` for `FRAMING_HEADERS`
  (`Content-Length`, `Transfer-Encoding`, `Connection`, `Upgrade`, any case), and
  `nvs-server`'s `overrides` drops the same four (`FRAMING`). Attacks: step 2 of
  `tests/hostile/core/Response/setHeader/01-...nvs` and
  `a_declared_framing_header_is_dropped_and_the_body_frames_itself` (serve.rs). The rule
  `rule:http-server/policy-headers-are-runtime-class-and-setheader-wins` has the paragraph and one
  Declined entry; the card and about page say so.
- **Regex, XML (2fb5eb4a5).** `Core\Regex::replace`'s replacement and both of
  `Core\Xml\Writer::instruction`'s parameters are `Qual::Sink`. The corpus passes only literals or
  plain strings there (grepped). The attack
  `tests/conformance/reject/a-regex-replacement-and-an-xml-instruction-refuse-tainted.nvst` was
  red on main. `rule:security/every-grammar-is-a-sink` keeps "four grammars" (the no-mode-strings
  rule closes that list) and adds a paragraph for each; it declines a literal-replacement option.
- **`quoteIdentifier` (005616bdf).** `nvs_db::NILADIC_NAMES` (16 names: the SQL standard's niladic
  functions and MySQL's `UTC_*`) and `is_niladic_name`, read only by
  `nvs_core_db_quote_identifier`; schema `Ident`s are delimited by `ddl.rs`, so they do not read it.
  Attacks: `a_name_the_database_reads_as_a_value_is_refused` (open.rs) and step 5 of the
  quoteIdentifier hostile case. `rule:core-classes/db-one-api`, the card and about page say so.
- `cargo build`, `cargo clippy --all-targets -- -D warnings`, `bun nv rules --render` and `--check`
  are green. `bun nv proofs --run` for the four members passes (examples and hostile).
- The sweep's `env-secret-proofs` red is the 00:07 log again (exit 0, `nothing owed`), and the
  nightly run 39746060276 is the cancelled `workflow_dispatch`. Neither is the tree's.

Tests that may go red from this session: any served test or `.nvst` that sets one of the four
framing headers through `setHeader` (none grepped); a `Core\Regex::replace` or `instruction` call
with a `tainted` argument in a Rust test that compiles source (not swept); a query that passes a
niladic name through `quoteIdentifier` (none grepped); the help snapshots and the website's
`core.json` / `llms-full.txt` for `setHeader`, `Regex::replace`, `Xml\Writer::instruction` and
`Db::quoteIdentifier`; the website's rule pages for the three rules changed; the `perf: stale`
proofs of `Core\Response::setHeader`, every `Core\Regex` and `Core\Xml\Writer` member and
`Core\Db::quoteIdentifier`.

The session before (HTML-2, pins, redirect):

- **HTML-2 and the comment (d7ac42606).** `addressable` in `crates/nvs-stdlib/src/html.rs`
  returns `false` for a value whose first two characters, after the tab/LF/CR strip and the
  leading-space trim, are both `/` or `\`. The attack
  `a_reference_that_names_another_host_is_dropped_and_a_path_query_or_fragment_stays` was red on
  main; all 31 `html::tests` pass, the mXSS fixed point included. The escape's module doc now says
  an unquoted attribute is **not** safe, and points at `rule:core-classes/html-template`'s blind
  spot. `rule:core-classes/html-sanitize` has the paragraph.
- **The probes item** is recorded on the item from the audit's `wb-html.md`, which ran them all.
- **Pins (a9f294809).** Every `cranelift*` is `0.136.4` and every `wasmtime*` is `49.0.4`,
  the locked releases; `Cargo.lock` did not change.
- **Redirect (99421501c).** `names_another_host` in `response.rs`: `Core\Response::redirect`
  throws `LogicError` for an address starting with two of `/` and `\` after leading spaces.
  The unit attack `a_relative_destination_that_names_another_host_is_refused` was red on main;
  `tests/hostile/core/Response/redirect/02-a-relative-address-that-names-another-host.nvs` and
  `bun nv proofs --run --id 'Core\Response::redirect'` (3 examples, 2 hostile) pass. The card's
  `LogicError` row and the about page say so. No rule fragment owns this sink; the module doc does.
- `cargo clippy --all-targets -- -D warnings`, `bun nv rules --render` and `--check` are green.
- The sweep's `env-secret-proofs` red is the 00:07 log again (exit 0, `nothing owed`), and the
  nightly run 39746060276 is the cancelled `workflow_dispatch`. Neither is the tree's.

Tests that may go red from this session: a `.nvst`, example or test that sanitizes a `//` or
`\\` link and expects it kept, or that redirects to `//...` (grep found none); the help
snapshots and the website's `core.json` / `llms-full.txt` for `Core\Response::redirect`'s card;
the website's rule page for `html-sanitize`; the `perf: stale` proofs of
`Core\Response::redirect` and `Core\Html::sanitize` (`response.rs`, `html.rs` changed).

The session before (NET-1, redirects):

- **NET-1 (7f17d9b34).** `denied_by_default` denies `64:ff9b:1::/48` whole, as
  `local-use NAT64 (64:ff9b:1::/48)`, since RFC 6052 lets its translator carry an IPv4 address in
  any of six layouts; and `fec0::/10` as `site-local (fec0::/10)`. `embedded` does not read the
  local-use prefix. The attack `the_local_use_nat64_prefix_and_site_local_addresses_are_denied`
  (`crates/nvs-config/tests/capability.rs`) was red on main; the range sweep gained `fec0::/10`. The
  rule names both ranges and declines reading the prefix's low 32 bits. The whole `capability`
  test file passes (34 + 11).
- **Redirects (1f802a759).** `transport::Hop { verb, body }` is what one hop sends, threaded
  through `attempts`, `one`, `compose(call, hop, parts)` and `exchange(call, body, ..)`.
  `transport::onward(hop, status, same_origin)` decides each hop: `303` → `GET` (`HEAD` stays),
  `301`/`302` after `POST` → `GET`, both with no body; a `307`/`308` leaving the origin with a body
  is not followed, so the redirect is the answer. `Idempotency-Key` is sent only while the hop's
  verb is the call's. The attacks `a_303_and_a_301_or_302_after_a_post_follow_as_a_get_with_no_body`,
  `a_307_or_308_to_another_origin_with_a_body_is_the_answer` and the table test
  `a_redirect_rewrites_the_method_only_where_rfc_9110_says_so` pass with every existing redirect
  test in `transport.rs` (12 run by name). `rule:http-server/a-cross-origin-redirect-drops-credentials`
  has the paragraph and two Declined entries.
- `cargo clippy --all-targets -- -D warnings`, `bun nv rules --render` and `--check` are green.
- The sweep's `env-secret-proofs` red is the 00:07 log again (exit 0, `nothing owed`), and the
  nightly run 39746060276 is the cancelled `workflow_dispatch`. Neither is the tree's.

Tests that may go red from this session: a `Core\Http` `.nvst`, example or served test that follows
a `303`, or a `301`/`302` after a `POST`, and expects the origin to see the original method or body
(none grepped); one that follows a cross-origin `307`/`308` with a body and expects the far answer;
an `nvs-config` or `nvs-runtime` test that expects an address in `64:ff9b:1::/48` or `fec0::/10` to
connect; the website's rule pages for the two rules changed.

The session before (HTTP-5):

- **HTTP-5 (86e60e3b6).** `forwarded::walk` reads `X-Forwarded-Proto` only when the trusted peer
  also forwarded a non-empty `X-Forwarded-For` chain; with no chain the scheme is `http` and the
  client is the peer. The attack `a_trusted_peer_that_forwarded_no_address_cannot_assert_the_scheme`
  (127.0.0.1, 10.0.0.7 and a Unix socket, plus a blank chain line) was red on main and passes, with
  the updated `the_nearest_proxy_asserts_the_scheme_and_only_https_is_believed` (its cases now send
  a chain). `rule:http-server/x-forwarded-proto-sets-the-scheme-and-hsts-only` gains the paragraph
  and two Declined entries. The `.nvsr` files were checked: every one that expects `https` sends an
  `x-forwarded-for` line; the others are untrusted peers. `serve.rs`'s `a_forwarded_request` sends
  both. `cargo build`, `cargo clippy --all-targets -- -D warnings`, `bun nv rules --render` and
  `--check` are green.
- The sweep's `env-secret-proofs` red is the 00:07 log again (exit 0, `nothing owed`), and the
  nightly run 39746060276 is the cancelled `workflow_dispatch`. Neither is the tree's.

Tests that may go red from HTTP-5: any served test, `.nvst` or `.nvsr` with `trusted_proxies` set
that sends `X-Forwarded-Proto: https` with no `X-Forwarded-For` and expects `https` or HSTS (none
found by grep; `crates/nvs-cli` served tests were not grepped for a trusted loopback); the
website's rule page for the rule changed.

The session before (HTTP-2, HTTP-4, APP-3, APP-2):

- **HTTP-2 (fe441247c).** `mount::host` takes a target's own authority (absolute form, HTTP/2's
  `:authority`) over `Host`, and returns `Host::Refused` when a `Host` line names another host
  after both are `folded` (port, case and one trailing dot are the same host). So the door answers
  `400`, and `Table::select` and `refuses_host` see the target's host. An absolute-form request
  with no `Host` line still reads the target, as before. Attack
  `a_target_naming_one_host_and_a_host_line_naming_another_is_refused` (mount.rs) was red on main.
  The rule has a paragraph for it.
- **HTTP-4, APP-3 (7af3f5b7a).** `heads::Scanner::end_head` treats a `Content-Length` beside any
  `Transfer-Encoding` as a defect, so the door answers `malformed_head()` (`400`, close). The two
  rows moved from the smuggling suite's "disambiguated" table into its `refused` table (16 rows).
  `body::Supply` records `malformed` when `hyper`'s body error has an `io::Error` source of kind
  `InvalidInput` or `InvalidData` (its chunked decoder's defects; `UnexpectedEof` is a client that
  left). In `serve.rs`'s collect arm, a malformed supply replaces the answer with `malformed_head()`
  and sets `by_limit = false`. A response already streaming cannot change. Attack
  `a_chunked_body_framed_wrong_is_a_400_that_closes` (serve.rs, three framings) was red on main
  (`200` with the program's text). `rule:errors/http-message-defects` says both.
- **APP-2 (the commit after 7af3f5b7a).** `Inbound::refuse_body_too_large` / `body_too_large` (one
  `bool`). `request::whole_body` sets it when `pull_whole` fails with `RuntimeError`, which is the
  cap alone. `router::dispatch_matched` sends a handler throw to `refuse(.., BODY_TOO_LARGE, route)`
  when it is set. `Inbound::refusal` is not used: it would make `route()` return `null` mid-handler.
  The multipart readers (`post` over a multipart body, `files`) have their own caps and do not set
  it. The attack `tests/hostile/core/Request/jsonAs/02-a-body-over-the-limit.nvs` printed `500` twice
  without the router change and `413, 413, 200` with it (debug binary). Both rules say so.
- `cargo build`, `cargo clippy --all-targets -- -D warnings`, `bun nv rules --render` and `--check`
  are green. Run by name: the three new Rust tests, `the_request_smuggling_suite_passes`,
  `a_repeated_length_and_a_bare_line_feed_are_refused_in_their_own_request` and the attack.
- The sweep's `env-secret-proofs` red is the 00:07 log again (exit 0, `nothing owed`), and the
  nightly run 39746060276 is the cancelled `workflow_dispatch`. Neither is the tree's.

Tests that may go red from this session: any served test or `.nvst` that sends an absolute-form
target with a `Host` line naming another host (none found); any test that sends both
`Content-Length` and `Transfer-Encoding` and expects an answer other than `400` (the smuggling suite
was updated; `nvs-cli` tests not grepped); any served test that reads a malformed chunked body and
expects the program's answer; a `Core\Router` test or `.nvst` whose handler reads a body over
`request_body`, throws, and expects `failed`/`500`; the website's rule pages for the four rules
changed.

The session before (RS):

- **The `sendFile` `500` is not a bug.** On this tree `Response::sendFile("assets/logo.svg")` in
  `src/App.nvs` resolves beside the source file (`src/assets/logo.svg`), which
  `rule:programs/relative-paths-resolve-from-their-file` says; `"../assets/logo.svg"` answers `200`.
  The audit's copy no longer compiles either: `Config::get("http.csrf_key")` is now
  `?secret string` (CRYPTO-3), as intended.
- **RS-1, RS-2, RS-4, RS-5, RS-6, RS-7 (76d3483a5).** `nvs_config::capability::kept_from_requests`
  (a dot component or `nvs.toml`, any case) is the one predicate; `in_app_folder` calls it.
  `Table::withholds` in `mount.rs` declines a step-3 file it names, a `.nvst`, and a database file
  from `Table::withholding` (from `from_config`'s `[db]` paths, canonical, plus `-wal`, `-shm`,
  `-journal`). The root config's `[db]` blocks only: a mount entry's own tree's blocks are not read.
  `.well-known` is withheld too (the rule says a route answers it). RS-2: `Table::select` takes
  the peer (`origin.client()` in `nvs-cli`'s door) and `Table::refuses_host` refuses a loopback
  peer whose `Host` is not `localhost`, `*.localhost`, a loopback literal or a mount's host, in
  development (`local_hosts_only`, from `[mode] default`); only the probe answers it, else `404`.
  RS-4: `OnDisk::file` uses `spelled_through_links` (the folder before each link canonical, the
  link's name found exactly in its listing, and the walk equal to the canonical path). RS-6:
  `conditional::Asked` has `range_repeated`; `position` takes digits only; `statics::asked` keeps
  every `Range` line. RS-7: `statics::send` sets `sandbox; frame-ancestors 'none'` for HTML, SVG
  and XML (`SANDBOXED`), which replaces the configured policy on those files. RS-5 is a Declined
  entry. The attacks (all pass, run by name): `a_project_file_is_never_served_as_a_static_file`,
  `a_project_file_on_disk_is_never_served_through_a_link`, `a_link_inside_the_root_keeps_the_case_rule`
  (a junction on Windows), `a_rebinding_host_from_this_machine_reaches_no_mount` in `mount.rs`;
  `a_file_a_browser_runs_script_from_is_sandboxed`, `a_range_written_two_ways_is_unsatisfiable` in
  `statics.rs`; `two_range_lines_are_unsatisfiable` in `conditional.rs`.
- `cargo build`, `cargo clippy --all-targets -- -D warnings`, `bun nv rules --render` and `--check`
  are green.
- The sweep's `env-secret-proofs` red is the 00:07 log again (exit 0, `nothing owed`), and the
  nightly run 39746060276 is the cancelled `workflow_dispatch`. Neither is the tree's.

Tests that may go red from the RS slice: any served or `nvs-cli` test in development mode that
sends a non-local `Host` (such as `example.com` or a test host no mount lists) over loopback — it
now gets `404` (`live_config.rs`, `serve.rs` tests and `.nvst` request helpers are the likeliest;
not grepped); any test that serves a static `.html`/`.svg`/`.xml` and expects the configured
`Content-Security-Policy`; any test that fetches a dot-file, `nvs.toml`, a `.nvst` or a
`.well-known` file through static serving; a `Range` test with a space or `+`; the website's rule
pages for the two rules changed; the `perf: stale` proofs of `Core\Response::sendFile`
(`statics.rs` changed).

The session before (CVE-2):

- **CVE-2, runtime half (95017e2d3).** `beneath.rs` has `remove_file`, `remove_dir`, `metadata`,
  `entry_metadata` and `list` (returns `Vec<Entry { name, kind: Kind }>`). Unix: `unlinkat`; the
  entry opened `O_PATH|O_NOFOLLOW` (Linux) or `O_EVTONLY|O_SYMLINK` (macOS) and `fstat`ed;
  `fdopendir` on `.` beneath the walked folder, `d_type` with an `fstatat` fallback. Windows: the
  name opened `DELETE` beneath the folder and a `FileDispositionInfoEx` (POSIX) disposition, plain
  `FileDispositionInfo` on FAT; metadata from the entry's `FILE_OPEN_REPARSE_POINT` handle; a
  listing reopens the folder by an empty name for `FILE_LIST_DIRECTORY` and reads
  `FileFullDirectoryInfo`, opening a reparse entry once more for its tag. `metadata` returns
  `NotFound` for a link at the last name (it led nowhere at resolve, or was planted), so `exists`
  is `false` for a dangling link as before. `capability::metadata`, `metadata_if_present`,
  `exists` and `read_dir` now use `resolve`; `remove_file`/`remove_dir` use `resolve_name` (a link
  at the name is removed, and its grant is checked on the entry, not its target). `resolve` and
  `resolve_name` now call `alias_refusal`, which the open doors skipped before. `read_dir` returns
  the whole `Vec<Entry>`; `Core\IO::list`, `::walk` and `Core\Storage::list` read `entry.kind`.
  The attack `a_removal_a_stat_and_a_listing_never_follow_a_planted_link` and
  `removes_reads_and_lists_what_is_there` pass on Windows with all 10 `beneath::tests`.
  **The Unix half was not compiled** (the guard refuses a cross-target `-p` check, and the
  whole tree cannot cross-build here). CI's Linux and macOS legs are its first build.
- **CVE-2, WASI half (a00da75b3).** `Budget` has `remove_file`, `remove_dir`,
  `metadata(path, follow)` and `read_dir` (returns `Vec<Listed { name, kind: ListedKind }>`), each
  defaulting to the by-name call. `files.rs` calls them for `unlink-file-at`,
  `remove-directory-at`, `stat-at`, a directory's `stat`/`get-type`/`metadata-hash`,
  `metadata-hash-at` and `read-directory`. `Spent` fills them from `guest_files`. The guest in
  `crates/nvs-ext/tests/files.rs` gained `unlink`, `rmdir`, `stat` and `listing` exports, and
  `a_guest_opens_every_file_through_the_hosts_walk` checks all four refused by a walled host and
  run by an open one. It passes.
- `rule:security/writes-open-beneath-a-handle` names the new doors, its title changed, and its cost
  and memory lines say a listing is read whole.
- `cargo build`, `cargo clippy --all-targets -- -D warnings` (Windows), `bun nv rules --render` and
  `--check` are green.
- The sweep's `env-secret-proofs` red is the 00:07 log again (exit 0, `nothing owed`), and the
  nightly run 39746060276 is the cancelled `workflow_dispatch`. Neither is the tree's.

Tests that may go red from CVE-2: the Linux or macOS build itself (the Unix `list`, `stat_at`,
`remove_at`, `clear_errno` and `ENTRY_FLAGS` in `beneath.rs` were never compiled); any `.nvs`,
`.nvst` or Rust test that removes a link whose target is outside the grant and expects a refusal
(it is now removed), that expects `Core\IO::exists`, `isFile`, `isDir`, `size` or `list` on a
path with `..` past a missing folder to answer (it now throws `IOError`), or that hands a stat,
removal or listing door a Windows alias; `Core\IO::walk` over a tree with a folder link (the kind
is read without following, as before); the website's rule page for `writes-open-beneath-a-handle`
(title changed); the `perf: stale` proofs of `Core\IO::list`, `::walk`, `::exists`, `::size`,
`::remove`, `::removeDir` and `Core\Storage::list`.

The session before (SINK-1, SINK-3, EXT-10's names):

- **SINK-1, SINK-3 (fb186cf9e).** `nvs_config::capability::windows_alias(text)` judges a path by
  its text, on every platform: two separators first (`\\`, `//`, `\\?\`, `\\.\`), a `:` after the
  drive letter, and each component through `alias_component` (a trailing dot or space; an 8.3
  name, `~` and digits before the end or the first `.`). `.` and `..` are not names.
  `nvs_runtime::capability::require_as` calls it as `alias_refusal`, after `device_refusal` and
  before any grant is read, so every door refuses with a `RuntimeError` ending "on any platform".
  `resolved` returns `None` for a path with two separators first, and reads only that form,
  because `temp_dir` resolves an 8.3 `[io] temp_root` (`C:\Users\RUNNER~1\...` on CI) to its long
  name through it. So `granted()` callers that skip the door (`Core\Cap::has`, `script::resolve`,
  `Core\Db::open`) never open a network path either. The attacks: step 7 of
  `tests/hostile/core/IO/within/01-names-built-to-escape.nvs` (eight aliases, all refused; ran
  green on the debug binary), `a_windows_alias_is_refused_by_its_text_before_any_network_attempt`
  in `io.rs` (ten aliases under 500 ms; on main the UNC one waited on the network for over 60 s
  per the audit), and `a_windows_alias_is_found_by_its_text` in `crates/nvs-config/tests/capability.rs`.
  `rule:errors/path-component-refusals` says every door refuses the aliases and states the cost (a
  POSIX `foo.`, `a:b`, `v~2` and a `//` share are unreachable). `path-scope-canonicalise-then-prefix`
  says the text is judged first.
- **EXT-10's names (3e74f54fa).** `refused_name` in `crates/nvs-ext/src/wasi/files.rs` refuses a
  `:` on every platform (before, Windows only), a device name and `alias_component`. Unit test
  `a_guest_name_windows_reads_as_another_is_refused_on_every_platform`.
- `cargo build`, `cargo clippy --all-targets -- -D warnings`, `bun nv rules --render` and `--check`
  are green. Run by name: the three new tests, `core_io_within_...`, and `bun nv proofs --run --id
  'Core\IO::within'` (3 examples, 1 hostile, all ok).
- The sweep's `env-secret-proofs` red is the 00:07 log again (exit 0, `nothing owed`), and the
  nightly run 39746060276 is the cancelled `workflow_dispatch`. Neither is the tree's.

Tests that may go red from SINK-1/3: any Rust test, `.nvst` or example that hands a door a path
with a component ending in `.` or a space, a `:` past the drive, a `~<digits>` name, or a `//`
start (a Windows CI leg whose test builds a path from the raw `TEMP`/`std::env::temp_dir()` rather
than `nvs_repo::scratch` or `Core\IO::temporaryDir` is the likeliest; `nvs_repo::scratch` is under
`target/` and safe); a WASI guest test that names a file with `:` on Unix; the website's rule
pages for the two rules changed.

The session before:

- **CRYPTO-6 (fd55a8c78).** `Core\Random\Seeded::bytes` returns `tainted bytes` and `::token`
  returns `tainted string`. `nvs_stdlib::keyring::KEY` is now `SecretBlob(Qual::Sink)` and is the
  one type every key parameter is written with: `Core\Crypto::seal`/`open`/`expandKey`,
  `Core\Crypto\KeyPair::read`, `Core\Hash::hmac`, `Core\Csrf`'s and `Core\Jwt`'s `KEY`,
  `Core\Jwe\Key::shared`, `Core\Totp`'s `SECRET`, and every ring (`SignedCookie`, `Signature`,
  session, cache, router, `Uri`). Before, every one was `Neutral` and took `tainted`. A key from a
  database row now needs `Core\Taint::assertTrustedBytes`. The attack
  `tests/conformance/reject/seeded-output-cannot-become-a-key.nvst` (seven refusals, and a
  `generateKey` seal that compiles) compiled on main and passes. A compile sweep of the 440 corpus
  files that call a key member or `Seeded` (`.nvst` with an expected error skipped) found only the
  Seeded files themselves. Their benches, examples 03, hostile cases and
  `random-seeded-gives-the-same-sequence-for-the-same-seed.nvst` declare `tainted` now.
  `docs/examples/core/Signature/sign/02-...nvs` failed on stage 4's keys mark, not this change, and
  now declares `array<tainted string> $names`. `rule:security/tainted-sources` says so and declines
  a `predictable` qualifier and a run-time randomness check. Both Seeded cards and about pages say
  the result is `tainted`.
- `cargo build`, `cargo clippy --all-targets -- -D warnings`, `bun nv rules --render` and `--check`
  are green. Run by name: the new reject case, the Seeded conformance case, and `bun nv proofs
  --run` for `Seeded::bytes`, `Seeded::token` and `Signature::sign`. All pass.
- The sweep's `env-secret-proofs` red is the 00:07 log again (exit 0, `nothing owed`), and the
  nightly run 39746060276 is the cancelled `workflow_dispatch` named below. Neither is the tree's.

Tests that may go red from CRYPTO-6: the registry or `nvs-types` tests that count marks per
signature (`every_mark_a_row_can_write_reaches_a_signature` only needs each mark once; a test that
counts `Neutral` key parameters or lists `Qual::Sink` parameters by class, such as `sinks_by_class`
in `crates/nvs-types/src/core_lib.rs`, gains `Core\Crypto`, `Hash`, `Csrf`, `Jwt`, `Jwe`, `Totp` and
every ring class); any test that passes a `tainted` value (a request field, a row, an HTTP body)
as a key (the sweep found none, but `.nvst` cases with an expected error were skipped and Rust
tests that compile source were not swept); the help snapshots and the website's `core.json` /
`llms-full.txt` for `Seeded::bytes`/`::token` and every key member; the `perf: stale` proofs of
both Seeded members (`bun nv proofs --record-perf --group 'Core\Random\Seeded'`); the website's
rule page for `tainted-sources`.

The session before:

- **CRYPTO-3, `get` half (827baed9b).** `crates/nvs-types/src/secret_directive.rs`'s `types_secret`
  is called from the static call path in `expr/calls.rs`. For `Core\Config::get` with a literal
  name that `nvs_config::secret::is_secret_key` names, it files `PreparedFact::SecretDirective`
  under the call's span, and the call is typed `?secret string`. The word reaches the member through
  the existing prepared channel: `Core\Config::get` joined `PREPARED_MEMBERS`, the helper is
  `args: [2]` (word, name), and the word is `nvs_stdlib::config::PREPARED_SECRET = 4`
  (`nvs_ir::ir::Prepared::SecretDirective`, printed `prepared config.secret`). A secret directive
  with any other word throws `LogicError`, so a computed name and a method reference throw. The
  attacks: `tests/conformance/reject/a-secret-setting-read-by-config-get-cannot-be-printed.nvst`
  (new), `tests/conformance/core/password-file-is-read-under-run.nvst` (now compares the password
  and pins `computed=LogicError`; on main the computed name read the value), and
  `tests/hostile/core/Config/get/02-a-secret-name-built-at-run-time.nvs`. Two example `.nvs`
  files under `docs/examples/config/` (`http-csrf_key`, `http-csrf_key_file`), the
  `secrets-from-files` example (its `.out` lost the `15` line), and three hostile cases under
  `tests/hostile/config/` (`http`, `http-csrf_key`, `http-client-proxy`) changed. Each one printed
  a secret setting or read one by a computed name. `get`'s card has a `LogicError` row.
  `rule:config/a-secret-is-a-file-whose-content-is-the-value` says so and declines two options, and
  `rule:security/secret-has-no-ambient-source` names the third origin.
- `cargo build` and `cargo clippy --all-targets -- -D warnings` are green. `bun nv rules --render`
  and `--check` are green. Run by name: the three attacks and the dump case (`nvs test`), and
  `bun nv proofs --run` for `Core\Config::get`, `directive:http`, `directive:http.csrf_key`,
  `directive:http.csrf_key_file`, `directive:http.client.proxy`, `directive:cache.shared` and
  `tools:config/secrets-from-files`. All pass.
- The sweep's `env-secret-proofs` red is the 00:07 log again (exit 0, `nothing owed`).

Tests that may go red from this session: a registry or footprint test that checks a
`PREPARED_MEMBERS` helper's `args: [N]` against its parameters (none read; `get` is now one more);
an `nvs-ir` lowering snapshot of a `Core\Config::get` call (it gains a `prepared` constant first);
the help snapshots and the website's `core.json` / `llms-full.txt` for `Core\Config::get`; the
website's rule pages for the two rules changed; any `.nvs`, `.nvst` or page outside the files
grepped that echoes or assigns to `string` a literal `get` of a `*.password` or `http.csrf_key`
(a `git grep` found none left); `crates/nvs-config/src/default.toml:384`'s comment shows
`Core\Csrf::issue($session, Core\Config::get("http.csrf_key"))`, which never type-checked
(`?string` for a `secret bytes` key) and still does not.

The session before:

- **d4, d5 (c4259bed0).** Recorded and closed with `bun tools/looper/main.ts decision close`.
- **CVE-1, Postgres (ac872e079).** `RESET_COMMANDS` is eight: `SET SESSION AUTHORIZATION DEFAULT`
  and `RESET ROLE` follow `ROLLBACK`, because `RESET ALL` skips both settings. The old doc said it
  covered them, and the property test mapped `SET ROLE` to `RESET ALL`. Both are corrected, and the
  unit test that pinned six commands is renamed `..._eight_commands_...`. The attack
  `a_role_one_request_assumed_is_gone_for_the_next` in `crates/nvs-db/tests/pool_reuse.rs` runs
  only on the Postgres matrix leg (`bun nv db-matrix`), and **was not run**. It assumes the matrix
  account is the image's superuser (it sets session authorization to `pg_monitor`).
- **CVE-1, SQLite (2a4b20976).** `sqlite::authorize` is installed at `open`: it denies every
  `ATTACH` but an unnamed one (`''`, the temporary database a plain `VACUUM` attaches), every
  `DETACH`, and a bound-name attach by its code. `VACUUM INTO` goes through the same internal
  `ATTACH`, so it is denied too (checked in the bundled `sqlite3.c`: `sqlite3Vacuum` has no auth
  call of its own). `SESSION_PRAGMAS` is applied at open and again by `reset`. `reset`'s new
  `clean` rolls back, refuses a connection with any database still attached, applies the
  pragmas first (`query_only` would refuse the drops), then drops every `temp` trigger, view and
  table. Three unit attacks in `sqlite.rs` were red on main and are green, with all 43 of the
  module's tests.
- **SINK-2 (e5b7d5e82).** `sqlite_read_only` in `crates/nvs-stdlib/src/db/open.rs` throws for a
  program's path that starts with `file:`, whatever is granted. A `[db.<name>]` block's `path`
  may still be a URI. It is the operator's, and the queue and lock tests use
  `file:...?mode=memory&cache=shared`. That is the safe reading of the standing decision's "at load", since
  option keys would be new configuration. The attack
  `tests/hostile/core/Db/open/02-a-path-written-as-a-uri.nvs` (with an `[[app]]` granting `db.open`
  and both `fs` keys in the root `nvs.toml`) was red on main and is green.
  `rule:core-classes/db-capabilities` says so.
- **DB-4, DB-5 (73223c0cd).** `rule:core-classes/db-one-api` says SQL Server's guard against a second
  statement is the tainted check, and declines a run-time scan and an allow-list parameter.
  `quoteIdentifier`'s card and about page say it checks syntax only. Example 03 checks the column
  against an allow list first, and its `.out` is unchanged.
- `cargo build`, `cargo clippy --all-targets -- -D warnings` and `bun nv rules --render` are green.
  Run by name: the `reset` and `sqlite::` unit tests in `nvs-db`,
  `a_sqlite_file_asks_the_file_rule_and_opens_read_only_where_it_is_not_writable`, the new hostile
  case and example 03.
- The sweep: `env-secret-proofs`'s log is the 00:07 one again (exit 0, `nothing owed`), and the
  nightly run 39746060276 is the cancelled `workflow_dispatch` named below. Neither is the tree's.
- `AGENTS.md` and `docs/agent/loop-authoring.md` had changes from outside this session. They were
  not staged.

Tests that may go red from this session: any `.nvs`, `.nvst` or test that opens SQLite through
`Core\Db::open` with a `file:` path (grep found only the unit test, updated); any SQLite test
through this driver that runs `ATTACH` or `VACUUM INTO`, or that expects a `temp` table or a
`PRAGMA` to survive a pooled reuse (none found); the help snapshots and the website's `core.json` /
`llms-full.txt` for `Core\Db::quoteIdentifier`; the website's rule pages for `db-one-api`,
`db-capabilities` and `db-pool-reset-is-a-boundary`; Postgres matrix tests if the account cannot
run `SET SESSION AUTHORIZATION DEFAULT` (any account can, but it was not run against a server).

The session before:

- **CRYPTO-2 (65240abf6).** `Core\SignedCookie::seal(string $name, string $value, $keys)` and
  `::open(string $name, string $cookie, $keys)`: the name is a new required first parameter, and
  the associated data is `cache::bound(COOKIE_DOMAIN = 3, b"", name)`. Cache is octet 1, session
  secrets 2, and `Core\Crypto::seal` and `Core\Csrf` tokens seal under empty associated data, so
  none opens as another. The application is not bound, so a deployment that moves folders keeps
  its cookies. Every caller passes a name (benches, examples, conformance, hostile, the website
  snippet, `examples/session.nvs`); the about pages, the cookbook page and the two cards say so.
  The attacks: `a_cookie_opens_only_under_the_name_it_was_sealed_for` (unit, in
  `signed_cookie.rs`), `tests/hostile/core/SignedCookie/open/02-a-cookie-under-another-name.nvs`,
  and the rewritten `csrf-a-signed-cookie-of-the-session-is-not-a-token-under-the-same-key.nvst`
  (a token and a `Crypto::seal` message now throw in `open`; on main the token opened). Every
  cookie sealed before this commit stops opening. `rule:security/protocol-roster` says so and
  declines a sealed expiry and a per-purpose derived key.
- **CRYPTO-3, `all` half (fde9fb2dc).** `nvs_config::Request::all` renders `<secret>` for every
  key `nvs_config::secret::is_secret_key` names (a `SECRETS` row's value key, `*` one segment),
  whether a file or the tree set it. `REDACTED` moved from `audit.rs` to `secret.rs`. The attack
  is `tests/conformance/core/password-file-is-read-under-run.nvst`, which pinned the leak
  (`all= s3cr3t`) and now expects `<secret>` for a file and an inline password; red on main.
  `all`'s card and `rule:config/a-secret-is-a-file-whose-content-is-the-value` say so.
- **CRYPTO-3, `get` half: parked as d4.** `get` returns `?string`, and secrecy is an erased static
  qualifier, so typing it `?secret string` for a secret directive needs a literal-name hook in the
  checker. The intrinsic roster (`crates/nvs-types/src/intrinsics.rs`) validates literals and never
  changes a return type. Recommended: option 1, that hook.
- **CRYPTO-6: parked as d5.** `Seeded::bytes`/`::token` return plain `bytes`/`string`, which widen
  onto `secret`. Recommended: option 1, return them `tainted` and have every key parameter refuse
  `tainted` (not checked whether key parameters accept `tainted` today).
- The sweep's `env-secret-proofs` red is the 00:07 log again; `bun nv proofs --gate --only
  'Core\Env::secret'` exits 0 on this tree.
- `cargo build`, `cargo clippy --all-targets -- -D warnings` and `bun nv rules --check` are green.
  Run by name: the five `signed_cookie` unit tests, `a_secret_key_is_a_rows_value_half_with_one_segment_for_its_star`,
  the nine signed-cookie conformance cases, the two secret config cases, and
  `bun nv proofs --run` for both `Core\SignedCookie` members (examples, attacks and benches).

Tests that may go red from this session: the help snapshots and the website's `core.json` /
`llms-full.txt` for `Core\SignedCookie::seal`/`::open` (new `$name` parameter) and
`Core\Config::all` (card); the `perf: stale` proofs of both `Core\SignedCookie` members
(`bun nv proofs --record-perf --group '<group>'` re-records); any `.nvs`, `.nvst` or page outside
the files grepped that calls `SignedCookie::seal`/`open` with two arguments (a `git grep` found
none left); any test that reads a secret's value through `Core\Config::all` (none found); the
website's rule pages for `protocol-roster` and the secret rule.

The session before:

- **CSRF-1/RS-3 and BBK-1 (0de803e8b).** `nvs_runtime::Inbound::cookie(name)` is the one cookie
  reader (every `Cookie` line, pair by pair, the `__Host-`/`__Secure-` rule). It moved out of
  `nvs-stdlib/src/request.rs`; `cookie_of` there is now a one-line call to it. The door's
  `route::session` reads through it and treats an empty or non-UTF-8 value as no session, and
  `route::verified` refuses every token on a request with no session. BBK-1's pin was **not**
  refused on main: a token for `""` with no cookie passed from any origin. The attacks
  `a_token_issued_for_no_session_is_refused_from_any_origin` and
  `the_door_finds_the_session_wherever_the_session_reader_finds_it` (in `route.rs`'s tests) were
  red on main and are green. `live_config.rs`'s `csrf_token` now binds `sid-live` and the post
  sends `Cookie: nvsid=sid-live`. The rule says a login form must write the session before it
  renders its token.
- **CSRF-2 (913f5df4d).** `route::unmatched_write` (an unsafe wire verb with no match) gets the
  origin half in `csrf`, key or no key. The token half stays a matched row's, and the rule says
  why. The attack `a_cross_origin_write_nothing_matched_is_refused` was red on main.
- **SESS-1 (c8292355a).** A cookieless or unknown-id `start` opens `Session { id: "" }`: no `put`,
  no cookie. `write_back` draws the id and sends the cookie (after `head_open`) on the first write
  that leaves the record non-empty; `send_at_end` writes the row. `regenerate` on an empty id is a
  no-op, `destroy` skips the store. The attack `a_session_nobody_wrote_leaves_no_row_and_sends_no_cookie`
  (20,000 starts over the fake shared store) was red on main. The old start test is renamed
  `start_opens_a_held_record_and_a_fresh_one_for_an_id_the_store_does_not_hold`, and its rule
  sentence follows. Step 6 of `tests/hostile/core/Session/start/01-identifiers-nobody-issued.nvs`
  runs 20,000 read-only start/regenerate/destroy cycles. It needs Redis on 16379 and was not run.
  `START_DOC` and `SET_DOC` help changed.
- The sweep: `env-secret-proofs`'s log is the 00:07 one again (exit 0). The nightly run 39746060276
  is the cancelled `workflow_dispatch` already named below. Neither is the tree's.
- `cargo build`, `cargo clippy --all-targets -- -D warnings` and `bun nv rules --check` are green.

Tests that may go red from this session: `nvs-stdlib`'s `request.rs` cookie tests (the reader
moved, the logic is the same; not run); any served or `.nvst` test that sends a CSRF token issued
for `""` or with no session cookie under a key (grep found only `live_config.rs`, updated); any
served test that `POST`s with a foreign `Origin` to an unmatched path or a route-less program
(none found); a `.nvst` or served test that expects a `Set-Cookie` or a store row from a bare
`Core\Session::start()` (grep found none outside `session.rs`); the help snapshots and the
website's `core.json`/`llms-full.txt` for `Core\Session::start` and `::set`; the website's rule
pages for the three rules changed (`bun nv site --check` names them).

The session before:

- **EXT-10 (f8b660100).** `nvs_ext::call::Budget` has `open_file(path, FileOpen)`, `create_dir`
  and `rename`, each defaulting to the by-name call. `files.rs`'s `open` (every file open, read
  included), `create_directory_at` and `rename_at` call them. `Spent` in
  `crates/nvs-cli/src/extensions.rs` fills them from `nvs_runtime::guest_files`, which re-exports
  the beneath walk's new `Open` flag set, `open_with`, an exclusive `create_dir`, and `rename`.
  `beneath.rs`'s `Mode` maps to `Open`, so the `Core\IO` doors open exactly as before; `make_at`
  on Unix no longer swallows `EEXIST` itself (`child` does). The attack
  `a_guest_opens_every_file_through_the_hosts_walk` (`crates/nvs-ext/tests/files.rs`, a host that
  refuses every open) did not compile on main, since main had no seam. The link proof is
  `beneath.rs`'s `a_planted_link_is_never_followed`, extended to `open_with` and `create_dir`, plus
  `a_guest_open_takes_each_flag_and_a_folder_is_made_once`. All pass on Windows (a junction). No
  `nvs-cli` test builds a `Spent`, since it needs a live `Ctx`: its three methods are one call each.
  Removal, `stat-at` and `read-directory` moved into stage 11's CVE-2 item. The module doc and
  `rule:security/writes-open-beneath-a-handle` say so.
- The sweep: `env-secret-proofs`'s log is the 00:07 one again (exit 0, `nothing owed`). It is the
  driver's state, not the tree's.
- `cargo build`, `cargo clippy --all-targets -- -D warnings` and `bun nv rules --check` are green.

Tests that may go red from EXT-10: an `nvs-ext` test whose budget overrides nothing and expects a
WASI open to follow a folder link (the default still opens by name, so none should); on Windows, a
served extension test that opens a file under a granted root reached through a junction the root
itself does not resolve through (none found).

The session before (0153):

- **EXT-7 (deaadec0c).** `nvs_ext::manifest::Manifest::is_source(method)` is `method.source`
  or any `requests.read`/`write`/`connect` entry. `nvs_types::ext_lib::method_sig` and
  `nvs ext inspect` (`crates/nvs-cli/src/ext.rs`, which prints `tainted ` only when the return text
  has `string`) read it. The request decides, never the grant. The attack
  `an_extension_that_requests_any_io_is_a_source_whatever_its_methods_declare` in
  `crates/nvs-types/tests/extensions.rs` was red on main and is green. The `Geo\Atlas` examples
  needed no change: each `echo`es the result, and `echo` takes a `tainted string` under `nvs run`
  (01 and 03 run and match their `.out`).
- **EXT-8 (c292f63c5).** `nvs_ext::load::Set::insert` refuses an extension whose `settings.name`
  another of the set declares (case-insensitive). The attack
  `a_settings_block_another_loaded_extension_declares_is_refused` in `crates/nvs-ext/tests/load.rs`
  was red on main. Binding the block to the class is declined in the rule.
- **EXT-9 (8aa84b4d3).** `nvs_ext::call::afford(budget, what, bytes)` returns an `Unaffordable`
  error past `Budget::left`, and `failure` reads it as `Limit::Memory`. `Context::bytes` (both
  random imports) and `handle::bytes_of` (a whole `as-bytes` of a byte string, and every lent read)
  ask it first. The attacks: `a_guest_asking_for_more_random_bytes_than_its_request_has_left_reaches_the_memory_limit`
  (`crates/nvs-ext/tests/wasi.rs`, a new `flood` export on the probe guest) and
  `a_whole_lent_source_larger_than_the_memory_left_is_refused_before_it_is_read`
  (`crates/nvs-ext/tests/convert.rs`, a new `WHOLE` component). Both were red on main as a
  `realloc return: beyond end of memory` trap after the host allocated 64 MiB, and are green.
- The nightly run 39746060276 was a `workflow_dispatch` cancelled after 8 minutes, so every job
  reads `cancelled`. No job failed.

Tests that may go red from EXT-7 to EXT-9: a `.nvst`, example or test whose extension manifest
requests I/O and assigns a method's text result to a plain `string` (grep found only `Geo\Atlas`,
which passes); a help or inspect snapshot of such a method; a test that loads two extensions
declaring one settings block (none found); a guest test whose budget has `left` below a lent
source's size and expected `as-bytes` to read it (the spreadsheet tests use `range` under no
memory limit).

Tests that may go red from EXT-5 and EXT-6: any caller of `nvs_stdlib::extension_send` or
constructor of `Outgoing` outside the two updated (grep found none); a served test whose child
script passes a resource an extension returned to its parent, under a different `grants:` (it is
now refused as a number the child's request does not keep; none found); the website's generated
rule pages (`bun nv site --check` names them).

Tests that may go red from d2: a scheduler or isolate test that expects a served request's isolate
to run after tasks already ready (none found); a test that reads `held_by_this_core` or
`held_by_cores` on a gauge-claimed thread while a place from `admit` is held and expects the balance
alone (it now includes 1 MiB per share; `admit.rs`'s
`connection_buffers_between_requests_do_not_refuse_a_request` still fits by arithmetic, not run);
`crates/nvs-runtime/tests/server_memory.rs` if it admits and then reads the gauge exactly (not run).
From EXT-2: a test that expects a pool of 64 instances (none found).

Tests that may go red from EXT-1: any test that reads `TreeState::extensions()` as a `OnceLock`
(none found by grep), and a guest destructor that runs after its tree ended and loops (it now traps,
since the weak view reads as spent; the failure is not reported).

Tests that go red now (seen in the conformance run, left for stage 14):
`core/a-response-bytes-body-sends-octets-no-text-member-carries.nvst` and
`core/a-response-stream-carries-octets-and-agrees-with-a-whole-body.nvst` (stage 3: `Core\Out::capture`
throws on output that is not UTF-8; since HTML-1 a capture also substitutes a body's control bytes,
so their rewrite cannot read raw octets back through a capture at all); `core/arr-fill-keys-takes-any-keys-and-fill-always-starts-at-zero.nvst`,
`core/arr-overlay-deep-at-a-key-is-itself-one-level-down-and-keeps-the-bases-order.nvst`,
`core/arr-sort-by-key-compares-numeral-keys-bytewise-and-a-comparator-sorts-them-as-numbers.nvst`,
`core/bytes-pack-names-the-bound-each-field-stops-at.nvst` and
`core/signature-a-round-trip-hands-back-the-payload-in-canonical-order.nvst` (stage 4: keys and
packed values keep the mark); `novis/spreadsheet-an-export-writes-the-same-file-to-each-target.nvst`,
`novis/spreadsheet-from-rows-styles-its-header.nvst` and
`novis/spreadsheet-writes-with-no-configuration.nvst` (not looked at; likely stage 3 or 4).
Tests that may go red: the website's generated `core.json` and `llms-full.txt` (`open`'s card and
`$cookie` qualifier changed; `bun nv site --check` would name them), a help snapshot of
`Core\SignedCookie::open`, and the earlier stages' list: the E0810 card, `Core\Arr::shapeAs`'s card,
an `nvs-ir` lowering snapshot of a class with an observer, a guard, `LateInit` or a hook, and a page
listing every diagnostic code (`E0885`). Stage 5 adds: the website's generated `core.json` and
`llms-full.txt` (the cards of `Core\Response\Stream::write` and `Core\Ldap\Connection::add` changed),
and any help snapshot of those two members. A `.nvst` or `.nvs` that streams a `tainted` value or
passes a `secret` to an extension's `mixed` parameter would go red too. None was found by grep.
HTML-1 adds: the `perf: stale` proofs of every `Core\Response` body member and `Core\Out::capture`
(`response.rs` and `out.rs` changed; `bun nv proofs --record-perf --group '<group>'` re-records), any
`.nvst`, example or hostile case that captures or spawns a child that writes a body with a control
byte other than `LF`/`TAB` or with `<`, `>`, `&`, `"` or `'` under the HTML sink (grep found none
beyond the two above), and an `nvs-server` test that spawns a child writing a text body inside a request.
PARSE-1 adds: any `.nvs` or `.nvst` that parses a CSV document whose rows outgrow the memory limit and
expected it to finish (none known). Its own attack's `ends-early 5 FATAL` passes only on a binary
built after 66fc862ac. PARSE-2 and PARSE-3 add: any `.nvs` or `.nvst` that decodes a JSON or XML
document that outgrows the memory limit and expected it to finish, or that expected
`Core\Json::isValid` to answer `false` past the limit (none known). Both attacks' `ends-early 3 FATAL`
pass only on a binary built after 1ea3cb0ad. RT-4 adds: an HTML document whose tree outgrows the
limit by its arena's doubling and expected to finish (none known), and the Html attack 04's
`ends-early 3 FATAL` passes only on a binary built after 65c1adb0a. 8c8a86f6d adds: an `nvs-server`
test that reads `in_flight()` right after a buffered response's head arrives and expects `0`
(none found), and on Linux, any test that counts places while a client has not read a large body.
The admission slice adds: any test that reads `held_by_cores` or `nvs_memory_bytes` from a served core
between requests and expects the connection's buffers in it (`crates/nvs-cli/src/serve.rs:4271`
reads `held_by_cores`; not run), and a lowering or layout snapshot of `Ctx` (one new `bool`).
LC-3/LC-9 add: any test, `.nvst` or snapshot that expects a log record's `request_id` to equal an
inbound `traceparent`'s trace id, or a continued unsampled request's record to have no `trace_id`
(grep found only the ones updated here); any `nvs-cli` test that serves with a sampled inbound
`traceparent` and no `[trace] exporter` and expects spans, a `-01` `traceresponse` or `-01` on an
outbound call (none found); the website's generated rule pages (two rule titles changed). LC-6/LC-7
add: any test that reads `nvs_requests_total` under a non-standard method label, or expects a door
`501` at `error` with `[log] access` off (none found). LC-5 adds: a watchdog test whose request runs
blocking work and expects its charge to stay small (none found). LC-12 adds: any served test that
expects a `[log] handler`'s output in a response or on stdout (it goes to stderr now) or expects the
handler to have run by the time the response arrives (`live_config.rs` polls, and passes); any test
that reads `take_finished()` after a detached `afterResponse` tree and expects it filed (none found
by grep); the website's generated rule pages and any page that counts the default series (`bun nv
site --check` names them; grep found none outside `docs/rules/`). The overload slice adds: a
scrape or OTLP push test that expects the exact set of series and finds a new
`nvs_memory_bytes{scope="request"}` member (`prometheus.rs` and `otlp.rs` tests build their own
registries and do not call `every_core`; `live_config.rs`'s scrapes only check `contains`), and the
website's rule pages (`default-series`'s title changed).

### Next

1. The gate stage 14 (`### 14.` above). First read CI's Linux `docker` job and run
   `bun nv bench-compare --smoke` where Docker runs, for the `:ro` mount and `E0607`.
2. Start `looper verify` with `run_in_background`, write the handoff, and end the turn. Fix every
   failure it names in one pass, using the "Tests that may go red" lists in every State block
   above as the map, and the conformance reds listed under the oldest block. Then run it once
   more.
3. Before `DONE`: `bun tools/looper/main.ts decision close d6`, `bun nv verify --doc`, and
   `bun nv owners --closes security-hardening`, as AGENTS.md § *In the loop* says.

### Backlog

- da6b5cb72's budget half has no test: two tests that each spend most of an extension's CPU
  budget under `--jobs 1`. It needs a fixture guest with a method that spins for a given time,
  beside `tests/conformance/ext/fixtures/ledger`. No item owns it.
- The audit's RS-1 also proposed limiting step 3 to `GET` and `HEAD` (`POST /style.css` is sent as
  the file). No item owns it.
- `Table::from_config` withholds the root configuration's `[db]` files only. A mount entry whose own
  `nvs.toml` names a database under its static root is not covered, except by name rules. No item
  owns it.

- Under a keyed door, a page that renders a token for a visitor with no session must write the
  session first, since a token for no session is now refused and a bare `start` sends no cookie.
  No `Core\Session` member returns the id to issue the token for (BBK-1's first half). The pages
  that teach the pattern are `taught-app-works`'s.
- The extension pool stops at 4096 places (`MOST_SLOTS`), under the default `max_in_flight` of
  10000 once one class is loaded. Lifting it means a smaller memory reservation per place and a
  bounds check per guest memory access, which the rule declines. If a deployment needs more, the
  choice goes to the user.
- `nvs_memory_bytes{scope="cache"|"process"}`, `nvs_tasks_in_flight` and `nvs_deferred_trees` are
  seeded and never written, so they read `0` or have no member. `every_core`'s gather is where a
  process-wide figure can be set without a request writing it. No item owns them.
- The audit's LC-5 also proposed a SQLite progress-handler deadline from the request's remaining CPU
  time, so a slow statement stops at `cpu_time` instead of only being charged after it ends. No item
  owns it; the charge alone stops the request at its next safepoint once the statement returns.
- `crates/nvs-stdlib/src/` holds many `default_*.profraw` files from a coverage run. git ignores them,
  but `grep -r` over `src/` matches them. Whoever runs coverage there should write it under `target/`.
- The conformance case `tests/conformance/reject/an-array-key-or-a-formatted-value-from-outside-keeps-its-mark.nvst`
  has not been run by its runner; stage 14's verify runs it.
- `Core\Json::decodeAs<T>` (and the other `decodeAs`/`shapeAs` doors) refuse a `tainted string`
  argument outright, though `written_class_of` already takes a `subject_outside` flag that would
  check each field for `tainted`. A program can decode a request body only through
  `Core\Request::jsonAs`. Admitting it where the decode site's field check runs is a separate
  change, and no item owns it yet.
- `bun nv suite <tree> --case <path>` names cases that must pass and still judges the whole tree, so
  it ran all 3096. Run a few cases by name with `target/debug/nvs test <path>...`.
- `crates/nvs-stdlib/src/db/execute.rs:477`'s `rows_fit` checks `over_ceiling()` after each row is
  appended, not before. The "reserve band" gap named here earlier did not reproduce: under
  `Ctx::set_memory_limit` the armed ceiling is the limit itself, so `over_ceiling()` and
  `memory_breach()` agree, and `set_config`'s reserve lowers `memory_limit` and the ceiling together.
  Asking `affords` in front of the append is still the rule's shape, but no test fails without it, so
  it was not landed. No item owns it.
- `cargo fmt --all` rewrites about 14 files nobody touched this goal (`serve.rs`, `csv.rs`,
  `html.rs`, `quals.rs` and others). Run `rustfmt` on the files a slice changed, never the whole
  tree, or the slice's commit carries that churn.
