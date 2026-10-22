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
    title: "every string made from outside bytes is checked UTF-8"
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
    status: open
    context: [crates/nvs-server/src/serve.rs, crates/nvs-server/src/admit.rs, crates/nvs-server/src/trace.rs, crates/nvs-server/src/otlp.rs, crates/nvs-host/src/blocking.rs, crates/nvs-host/src/ladder.rs, docs/rules/http-server/admission-is-arithmetic-not-a-number.md, docs/rules/observability/sampling-is-head-based.md]
  - id: 8
    title: "an extension instance ends with its request"
    status: open
    context: [crates/nvs-cli/src/extensions.rs, crates/nvs-ext/src/call.rs, docs/rules/packaging/a-fresh-instance-per-request.md, docs/rules/security/extension-grants-are-an-intersection.md]
  - id: 9
    title: "sessions, the CSRF door and signed cookies read and bind one way"
    status: open
    context: [crates/nvs-server/src/route.rs, crates/nvs-stdlib/src/request.rs, crates/nvs-stdlib/src/session.rs, crates/nvs-stdlib/src/signed_cookie.rs, docs/rules/security/csrf-is-on-by-default.md, docs/rules/core-classes/session-is-started-explicitly.md]
  - id: 10
    title: "a pooled database connection starts clean, and SQLite opens only what the grant names"
    status: open
    context: [crates/nvs-db/src/pg.rs, crates/nvs-db/src/sqlite.rs, crates/nvs-stdlib/src/db/open.rs, crates/nvs-db/src/tds/rows.rs, docs/rules/security/db-pool-reset-is-a-boundary.md]
  - id: 11
    title: "a Windows path is refused by its text before the file system sees it"
    status: open
    context: [crates/nvs-runtime/src/capability.rs, crates/nvs-stdlib/src/io.rs, docs/rules/security/path-scope-canonicalise-then-prefix.md, docs/rules/security/writes-open-beneath-a-handle.md]
  - id: 12
    title: "development serving answers only loopback and serves no project file, and the HTTP front's small gaps are closed"
    status: open
    context: [crates/nvs-server/src/mount.rs, crates/nvs-server/src/statics.rs, crates/nvs-server/src/body.rs, crates/nvs-server/src/forwarded.rs, crates/nvs-stdlib/src/request.rs, crates/nvs-stdlib/src/router.rs, docs/rules/http-server/static-serving-is-one-policy.md, docs/rules/http-server/host-matching-is-on-the-host-part-only.md, docs/rules/http-server/the-body-is-read-on-demand-under-two-caps.md]
  - id: 13
    title: "the outbound client's redirects, the leftovers, and the audit's scratch is gone"
    status: open
    context: [crates/nvs-stdlib/src/http/transport.rs, crates/nvs-config/src/capability.rs, crates/nvs-stdlib/src/html.rs, docs/rules/http-server/a-cross-origin-redirect-drops-credentials.md, docs/rules/core-classes/html-sanitize.md]
  - id: 15
    title: "`nvs serve` and every reload check who owns the configuration, and a read-only mount passes"
    status: open
    context: [crates/nvs-cli/src/config.rs, crates/nvs-cli/src/reload.rs, crates/nvs-config/src/trust.rs, crates/nvs-config/src/resolve.rs, docs/rules/config/ownership-is-the-trust-boundary.md, docs/rules/config/the-ownership-check-runs-where-it-can-be-answered.md, tests/docker/smoke.sh]
  - id: 14
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: types-tests
    stage: 2
    name: the checker refuses an early `$this` and a second constructor call
    run: [bun, nv, verify, --fast, -p, nvs-types]
  - id: stdlib-tests
    stage: 5
    name: every Core signature keeps a tainted input's mark
    run: [bun, nv, verify, --fast, -p, nvs-stdlib]
  - id: server-tests
    stage: 12
    name: the server's admission, trace, mount and static tests pass
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
    name: the served configuration is ownership-checked, and a read-only mount passes
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

**The evidence is on disk, not in this file.** The reports are `.agent-tmp/sec-audit/reports/*.md`, one
block per finding with its ID (`RT-2`, `DB-1` ...), the request or program, the observed output, the
code path and a proposed fix. The repro programs sit beside them under `.agent-tmp/sec-audit/<area>/`,
and the release binary they were run with is `.agent-tmp/sec-audit/bin/nvs.exe`. Each item below names
its finding ID. Read that block before the item, never the whole report.

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
captured by `Core\Out::capture`, `Core\Html::later`, a later slot or `Core\Test`'s request helpers can
hold raw bytes from `Core\Response::bytes` or `::sendFile`. Each capture, and each `Core\Test`
request helper, throws `RuntimeError` naming the member when the output it ends with is not UTF-8.

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
- [ ] d2: an admitted request holds a provisional share of the room until its first publish, so a burst past the room gets `503` before it allocates; the size of the share and its cost are in `rule:http-server/admission-is-arithmetic-not-a-number`, a Rust test beside `admit.rs` sends a burst on one core and sees `503`, and the `flood` row run once shows Novis's holds refused with `503`, not stopped with `500`. `crates/nvs-server/src/admit.rs:1`
- [x] `[limits] max_tasks`'s comment says what `claim_task` enforces, not "bounds nothing today". `crates/nvs-config/src/default.toml:86`

### 8. an extension instance ends with its request

**Does:** EXT-1's 70 requests all answer, and EXT-2, EXT-5 and EXT-6 are closed.

- [ ] EXT-1: the tree's `Calls` are dropped when the tree ends, and the instance with them. `crates/nvs-cli/src/extensions.rs:333`
- [ ] EXT-2: the instance limit comes from `max_in_flight`. `crates/nvs-ext/src/call.rs:1170`
- [ ] EXT-5: a narrowed `grants:` reaches an instance the tree already holds. `crates/nvs-cli/src/extensions.rs:442`
- [ ] EXT-6: a redirect hop is checked against the extension's intersection (`rule:security/extension-grants-are-an-intersection`). `crates/nvs-ext/src/wasi/http.rs:488`
- [ ] EXT-7 to EXT-10: each Low item in the sinks report, fixed or declined into its rule. `.agent-tmp/sec-audit/reports/wb-sinks.md:212`
- [ ] The attack: 200 requests that each call `Novis\Intl`, every answer correct, memory flat; a Rust test in `nvs-cli`. `crates/nvs-cli/src/extensions.rs:1`

### 9. sessions, the CSRF door and signed cookies read and bind one way

**Does:** CSRF-1, CSRF-2, SESS-1 and CRYPTO-2 are closed, and CRYPTO-3 and CRYPTO-6 are fixed.

- [ ] CSRF-1, RS-3: the door reads cookies with `Core\Session`'s reader. `crates/nvs-server/src/route.rs:315`
- [ ] CSRF-2: a program with no route table gets the Origin check (`rule:security/csrf-is-on-by-default`). `crates/nvs-server/src/route.rs:124`
- [ ] SESS-1: `Session::start` writes no row and sends no cookie until the first write (`rule:core-classes/session-is-started-explicitly`). `crates/nvs-stdlib/src/session.rs:912`
- [ ] CRYPTO-2: signed cookies bind the name and a purpose into the associated data. `crates/nvs-stdlib/src/signed_cookie.rs:261`
- [ ] CRYPTO-3: `Core\Config::get` and `::all` return a secret directive as `secret string`. `crates/nvs-stdlib/src/config.rs:192`
- [ ] CRYPTO-6: `Core\Random\Seeded` output is refused where a key or a token goes. `crates/nvs-stdlib/src/random.rs:395`
- [ ] BBK-1's pin: a token issued for no session, sent with no cookie, is refused same-origin and cross-origin by a keyed door. It is refused on a2b4854a4, and the attack keeps it so once CSRF-1 changes how the door reads cookies (`rule:security/csrf-is-on-by-default`). `crates/nvs-server/src/route.rs:245`
- [ ] The attacks: a session cookie on a second line, a cookie swapped between names, and 20,000 cookieless starts that leave no rows. `tests/hostile/core/Session/start/01-identifiers-nobody-issued.nvs:1`

### 10. a pooled database connection starts clean, and SQLite opens only what the grant names

**Does:** CVE-1, SINK-2, DB-4 and DB-5 are closed.

- [ ] CVE-1: Postgres resets the role and session authorization, and the comment says what `RESET ALL` does not cover (`rule:security/db-pool-reset-is-a-boundary`). `crates/nvs-db/src/pg.rs:3427`
- [ ] CVE-1: correct the unit test that asserts the old mapping. `crates/nvs-db/src/pg.rs:6030`
- [ ] CVE-1: SQLite's authorizer refuses `ATTACH` and `DETACH`, and its reset drops `temp` objects and applies the opening pragmas. `crates/nvs-db/src/sqlite.rs:1347`
- [ ] SINK-2: a `file:` URI in `path` is refused at load. `crates/nvs-stdlib/src/db/open.rs:930`
- [ ] DB-4, DB-5: the SQL Server rule text and `quoteIdentifier`'s help. `crates/nvs-db/src/tds/rows.rs:446`
- [ ] The attacks: `SET ROLE` then a second request, a `temp` table and an `ATTACH` then a second request; Rust tests in `tests/pool_reuse.rs`, the Postgres one where the existing Postgres tests run. `crates/nvs-db/tests/pool_reuse.rs:1`

### 11. a Windows path is refused by its text before the file system sees it

**Does:** SINK-1 sends no packet, SINK-3's aliases are refused, and CVE-2's checked-then-reopened
operations open beneath a handle.

- [ ] SINK-1, SINK-3: the textual refusal runs before `canonicalize` (`rule:security/path-scope-canonicalise-then-prefix`). `crates/nvs-runtime/src/capability.rs:1159`
- [ ] SINK-3: `Core\IO::within` returns no alias. `crates/nvs-stdlib/src/io.rs:3688`
- [ ] CVE-2: `remove_file`, `remove_dir`, `read_dir`, `metadata` and `exists` open beneath a handle (`rule:security/writes-open-beneath-a-handle`). `crates/nvs-runtime/src/capability.rs:950`
- [ ] The attacks: a UNC, `\\?\`, `\\.\` and 8.3 path through `within`, refused with no network attempt; a Rust test that times the refusal. `tests/hostile/core/IO/within/01-names-built-to-escape.nvs:1`

### 12. development serving answers only loopback and serves no project file, and the HTTP front's small gaps are closed

**Does:** RS-1, RS-2, RS-4, RS-6, RS-7, HTTP-2, HTTP-4 and HTTP-5 are closed, and RS-5 is declined
into its rule.

- [ ] RS-1: static serving skips the set program reads skip, in every mode (`rule:http-server/static-serving-is-one-policy`). `crates/nvs-server/src/mount.rs:571`
- [ ] RS-2: a loopback development server answers only a loopback `Host` or a listed one. `crates/nvs-server/src/mount.rs:307`
- [ ] RS-4, RS-6, RS-7: the case rule through a link, a strict `Range` parse, and `sandbox` on static SVG and HTML. `crates/nvs-server/src/statics.rs:239`
- [ ] HTTP-2: an absolute-form target's host wins over `Host` (`rule:http-server/host-matching-is-on-the-host-part-only`). `crates/nvs-server/src/mount.rs:424`
- [ ] HTTP-4, APP-3: a malformed chunked body answers `400`, and so does a request that carries both `Content-Length` and `Transfer-Encoding` (it is framed by `Transfer-Encoding` and closed today). `crates/nvs-server/src/body.rs:198`
- [ ] APP-2: when a body reader trips `[limits] request_body`, the runtime records `Refusal::BodyTooLarge` on the request as it throws, and `dispatch` sends a handler that throws after that to the gate's `refused` with `413` instead of `failed` with `500`. No new error class: the reader still throws its `RuntimeError` (`rule:http-server/the-body-is-read-on-demand-under-two-caps`, `rule:routing/a-gate-answers-around-every-handler`). `crates/nvs-stdlib/src/request.rs:679`, `crates/nvs-stdlib/src/router.rs:2523`
- [ ] APP-2's attack: a 9 MB JSON body and a 9 MB form body to a route that reads them answer `413` through the gate, and the next request answers `200`. `tests/hostile/core/Request/jsonAs/02-a-body-over-the-limit.nvs:1`
- [ ] HTTP-5: `X-Forwarded-Proto` from a trusted peer that never set it is ignored (`rule:http-server/x-forwarded-proto-sets-the-scheme-and-hsts-only`). `crates/nvs-server/src/forwarded.rs:466`
- [ ] RS-5: the Declined entry for a hard link. `docs/rules/http-server/static-serving-is-one-policy.md:1`
- [ ] The attacks: `GET /nvs.toml`, `/.env` and `/data.sqlite` in development mode, a rebinding `Host`, and the static policy rows; Rust tests beside `mount.rs`. `crates/nvs-server/src/mount.rs:1586`

### 13. the outbound client's redirects, the leftovers, and the audit's scratch is gone

**Does:** the client's redirects follow RFC 9110, the remaining small items are fixed, and
`.agent-tmp/sec-audit/` is deleted.

- [ ] A 303 becomes a `GET`, and a 301 or 302 after a `POST` becomes a `GET`; a cross-origin hop drops the body (`rule:http-server/a-cross-origin-redirect-drops-credentials`). `crates/nvs-stdlib/src/http/transport.rs:2361`
- [ ] NET-1: the two IPv6 forms that carry private addresses are denied (`rule:security/net-address-policy`). `crates/nvs-config/src/capability.rs:502`
- [ ] Correct the comment that says an unquoted attribute is safe. `crates/nvs-stdlib/src/html.rs:119`
- [ ] HTML-2: `Core\Html::sanitize` drops a URL attribute whose value, after the tab, newline and carriage-return strip, starts with `//`, `\\`, `/\` or `\/`; a browser reads a backslash as a slash, so each names another host. The `addressable` doc and the rule say a relative reference never names a host (`rule:core-classes/html-sanitize`). `crates/nvs-stdlib/src/html.rs:1847`
- [ ] HTML-2's cases join the sanitizer's tests: the four forms are dropped, and `/path`, `?q` and `#frag` are kept. `crates/nvs-stdlib/src/html.rs:1847`
- [ ] Raise `wasmtime` and `cranelift`'s minimums to the fixed releases in the lock file. `Cargo.toml:98`
- [ ] Run the HTML-escaping probes the audit never reported on, and fix or record each result here — run, recorded as HTML-1 (stage 5) and HTML-2 above. `.agent-tmp/sec-audit/wb-stdlib-request/html/app.nvs:1`
- [ ] TAINT-5: `$m[$t] = …` with a tainted key makes the keys of `$m` tainted, as `keys_from_outside` reads them at a `foreach`. `crates/nvs-types/src/expr/assign.rs:1062`
- [ ] TAINT-5: `Core\Response::redirect` refuses a relative target that names another host (`//x`, `/\x`, `\/x`), so a launderer's plain answer cannot open a redirect. `crates/nvs-stdlib/src/response.rs:1473`
- [ ] TAINT-5: the regex replacement template is a sink and refuses `tainted` (`rule:security/every-grammar-is-a-sink`). `crates/nvs-stdlib/src/regex.rs:171`
- [ ] TAINT-5: `Core\Xml`'s writer `instruction` refuses a tainted target and data. `crates/nvs-stdlib/src/xml.rs:760`
- [ ] TAINT-5: `setHeader` refuses `Content-Length`, `Transfer-Encoding`, `Connection` and `Upgrade` as it refuses `Content-Type`; probe what hyper sends first. `crates/nvs-stdlib/src/response.rs:433`
- [ ] TAINT-5: `Core\Mail` writes `To:` and `Cc:` in angle brackets, `address_of` refuses `( " : ; %`, and a long subject is folded. `crates/nvs-stdlib/src/mail.rs:453`
- [ ] TAINT-5: `Core\Db::quoteIdentifier` refuses the niladic SQL names (`current_user`, `session_user`, `user`, …). `crates/nvs-db/src/schema.rs:93`
- [ ] TAINT-5: file and `Core\Storage` reads are written into `rule:security/tainted-sources`, as tainted or declined with the reason. Safe default: `Core\Storage` reads are tainted, and a file the program ships is declined. `crates/nvs-stdlib/src/storage.rs:202`
- [ ] Delete `.agent-tmp/sec-audit/`. `.agent-tmp/sec-audit/BRIEF.md:1`

### 15. `nvs serve` and every reload check who owns the configuration, and a read-only mount passes

**Does:** a served `nvs.toml`, include or secret file that another account can write stops the boot
with `E0607`, and a reload that finds one keeps the running snapshot and names the path. The Docker
smoke test's read-only bind mount still serves.

**Split:** one read-only `Explore` subagent lists every place the repository boots `nvs serve` on a file
in the checkout (the harnesses under `tools/nv/`, `tests/docker/smoke.sh`, the compose files under
`benches/`, the apps and the website's snippets), as `file:line` and one line each. The session makes
the code change before it runs any of them.

- [ ] `nvs serve`'s boot and every reload resolve through `Disk`. Every other reader keeps `Unowned` (`rule:config/the-ownership-check-runs-where-it-can-be-answered`). `crates/nvs-cli/src/config.rs:400`, `crates/nvs-cli/src/config.rs:575`, `crates/nvs-cli/src/reload.rs:370`, `crates/nvs-cli/src/reload.rs:414`, `crates/nvs-cli/src/serve.rs:429`, `crates/nvs-cli/src/serve.rs:1982`
- [ ] The `LocalFiles` doc and the module doc say which reader each command uses, as it now is. `crates/nvs-cli/src/config.rs:25`, `crates/nvs-cli/src/config.rs:109`
- [ ] On Unix, `trust::check` waives the owner test for a file and a directory on a read-only mount, and still refuses a group- or world-writable mode (`rule:config/ownership-is-the-trust-boundary`). `crates/nvs-config/src/trust.rs:184`
- [ ] Rust tests: a group-writable root and a group-writable include each stop `nvs serve` with `E0607` naming the path; a reload after a `chmod g+w` keeps serving and names the path; the read-only waiver passes and the writable mode under it does not. Each is named in its rule's `guardedBy`. `crates/nvs-cli/tests/serve_ownership.rs:1`, `crates/nvs-config/tests/trust.rs:1`, `data/rules/config/the-ownership-check-runs-where-it-can-be-answered.json:9`
- [ ] The rule gains the read-only mount paragraph, its reason, and what Docker Desktop on Windows reports. `docs/rules/config/ownership-is-the-trust-boundary.md:1`
- [ ] Every repository server from the subagent's list passes on Linux and on a Windows checkout, through the safe default's `bun nv` command where Windows needs it, and CI's Windows jobs run it. `tools/nv/main.ts:1`, `.github/workflows/ci.yml:829`
- [ ] The Docker guide says a read-only mount passes, and a writable one must belong to uid 65532 or root. `website/src/content/docs/guides/production/docker.mdx:59`

### 14. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stage 7 is open for one item, d2's, and it waits on decision d3. Stage 8 can go first.

This session (0149) landed no code:

- **d2's share has no size that meets the item, so d3 asks.** The scheduler's ready queue is FIFO
  (`crates/nvs-host/src/scheduler.rs:930`), and `Isolate::start` queues each request as a child task.
  So in a burst on one core, every ready connection passes `Admission::admit` before any request runs.
  Before it allocates, a hold looks the same as a light request. The default per-request cap is
  128 MiB, and the flood's room is 384 MiB (512 less `ENGINE_RESERVE`). A 1 MiB share (the publish
  step) still admits the 200 holds of the flood, because 200 MiB plus the cap fits. A share of the cap
  refuses the holds, and it also refuses a light burst past 3 at once, the division the rule declines.
  d3's option 3 (recommended) keeps the 1 MiB share and runs a started request next, so its
  allocation is published before the next admission. Without the FIFO wait, the gauge alone refuses
  holds past about 32.
- **`env-secret-proofs` is not a check any more.** `looper check --name env-secret-proofs` answers
  "no check matches", nothing in `docs/goals/` or `looper.toml` names it, and its only log (00:07)
  says exit 0. The sweep's red is a stale ledger line, not this tree. Nightly 39746060276 was
  cancelled whole.
- 7ede738e0 (looper's commit of session 0148's leftovers) adds three `[[launch]]` entries to
  `looper.toml` for the website. It is not this goal's work and was left as it is.

The previous session (the sweep's `env-secret-proofs` again exits 0 when its command is rerun, `nothing owed`;
its log on disk is from 00:07 and says exit 0, so the sweep's red is stale a third time. Nightly
39746060276 was cancelled whole):

- **The `flood` row (d70950be8).** `bench-compare --scenarios --row flood` serves
  `benches/compare/scenarios/flood/` (four programs: `/hold` keeps 8 MiB for 300 ms, `/` prints `ok`)
  with the 512 MiB cap of `loop` and `memory`. `floodProbe` runs `oha` against `/hold` at
  `FLOOD_CONNECTIONS = 200` for `FLOOD_SECONDS = 10` (`Probe.steady` takes a connection count now),
  sends `/` every 250 ms with a 10 s wait, then waits up to `CHANGE_WAIT_MS` for `/` and sends 8 holds
  together. `floodResult` passes when every probe is `200` or `503` with `Retry-After`, `/` recovered,
  the 8 holds answered and the backend never restarted. The README bullet, the site's `ROW_TEXT`, a
  unit test and the row order test are in. The CI job `flood` (deep lane only) runs
  `bun nv bench-compare --scenarios --smoke --row flood` with no `--expect`, and
  `rule:testing/the-deep-lane` says so.
- **What the row measured: every peer `no`.** Bun and Go were OOM-killed and restarted 5 times; PHP
  answered `502` and did not recover in 20 s. Novis lived and recovered in 0.3 s, but one probe in
  34 failed and only 6 holds of the load were answered. Reproduced without nginx on the same image
  with `--cpus 1 -m 512m`: all 200 holds of one burst are admitted, because none has allocated yet,
  and the process limit stops those past the room with `500` (no error line is logged for them).
  `/` waits up to 6 s behind the holds' CPU, about 25 ms each on one cpu. A second burst, sent while
  memory was still counted, got 21 `503`s with `Retry-After`. **Decision d2** asks whether admission
  should count a burst before it allocates. The user chose that (option 2). It is under *Standing
  decisions* and is stage 7's new last item; d2 is closed.
- **The shell wrapper killed the run twice as a hang (17ce4856b).** The Docker build of Novis's
  image runs in Docker's VM, so the wrapper saw no CPU and no output. `looper.toml`'s `[jobs]
  hang_exempt` now names `bench-compare` and `bench-proxied`. The two reports were deleted.
- `bun test tools/nv/test/bench-compare.test.ts` (64) and `bun nv rules --check` are green. No Rust
  changed.

The overload attack, for the record (the previous session):

- **The overload attack (08024fe6d).** `a_flood_past_capacity_is_refused_and_every_place_and_byte_comes_back`
  in `crates/nvs-cli/tests/serve_isolation.rs` serves `OVERLOADED` with two workers,
  `max_in_flight = 50`, `memory = "24M"`, `cpu_time = "200ms"` and a Prometheus scrape port. 200
  connections send 3 holds (300 ms sleeps) each, beside 10 requests that reach the memory limit, 10
  that reach the CPU limit and 10 whose client leaves, while a probe sends `/`. It asserts every
  refusal is `503` with `Retry-After`, a probe answers within 10 s, at least one hold was refused,
  the process lives, the request memory returns within `SETTLED` (2 MiB per worker) of its reading
  before the flood, 50 held requests sent together all answer `200`, and `/` answers `ok`. It ran
  once, green, in 2.6 s. `Client::answer` now also returns `Retry-After`.
- **`nvs_memory_bytes` was never written.** The family was seeded and nothing set it, so a scrape
  had no member. `nvs_runtime::metrics::every_core` now sets `scope="request"` on the first copy it
  gathers from `budget::held_by_cores`. `rule:observability/default-series` says so, and says
  `cache`, `process`, `nvs_tasks_in_flight` and `nvs_deferred_trees` are not written yet (see
  Backlog). Its title said "Ten series"; it says "Eleven" now. Both rules' `guardedBy` name the test.
- `cargo build`, `cargo clippy --all-targets -- -D warnings` and `bun nv rules --check` are green.

LC-12, for the record (the previous session):

- **LC-12 (18414cd54).** `nvs_host::ladder::escalate` on server work (`config.serves()`) calls the
  new `off_the_request`: it starts the handler on a root task of its own with the new
  `nvs_host::spawn_root`, under `ctx.handler_isolate(OutputSink::Stderr)`, and answers `true`. The
  failing request no longer joins it. The task writes `floor::report` itself if the handler fails.
  `HANDLERS_PER_CORE = 4` (reason in the code); past it the run is dropped, counted by the new
  `nvs_runtime::metrics::count_handler_dropped` in the new default series
  `nvs_log_handler_dropped_total` (eleven series now), and `escalate` answers `false`, so the floor
  writes the record. The recursion guard for these is the thread-local `HANDLERS` list of root task
  ids plus the new `nvs_host::running_beneath`; a `Slot` guard the task owns gives the place back
  however it ends. A CLI program still joins (the `RUNNING` path). `rule:errors/handler-script` has
  the paragraph, a memory line, two Declined entries and `guardedBy`; `default-series` has the row.
- **A leak found on the way, fixed in the same commit.** A tree `detach_current` cut loose (every
  request with `afterResponse` work) was filed as `Finished` with its whole `Ctx` when it ended, and
  `nvs serve` never takes the finished list, so it held one context per such request served.
  `TaskNode::detached` now marks it, and the `Return` arm drops its context like a child's.
  Tests `a_detached_or_spawned_root_is_not_filed_when_it_ends` (scheduler unit) and
  `a_served_failure_does_not_wait_for_its_handler_and_handlers_per_core_are_bounded`
  (`crates/nvs-host/tests/limits.rs`) are new and ran once, green; the served end-to-end test
  `a_log_handler_beside_the_configuration_runs_from_any_working_directory` (`live_config.rs`) ran
  once, green.
- **65e8d91d1:** `default.toml`'s `max_tasks` comment says a spawn past it stops the request.
- `cargo build`, `cargo clippy --all-targets -- -D warnings` and `bun nv rules --check` are green.

LC-3 to LC-5, for the record (the previous session):

- **e139f309d** renders the core-classes, errors, http-server and security chapters, which stages 5
  to 7 left stale. Run `bun nv rules --render` after editing a fragment.
- **LC-3, LC-9 (004430e72).** `nvs_config::export::head_sample` returns `Option<f64>`: `None` where
  `Tracing::of` is `None`. `nvs_server::trace::take(inbound, Option<f64>)` with `None` calls the new
  `TraceContext::unrecorded`, so neither a draw nor a client's `-01` records anything; the trace id
  is still continued, and an outbound call carries `-00`. A scheduled run uses `unwrap_or(0.0)`
  (`crates/nvs-cli/src/serve.rs`, `rooted`). `TraceContext` has a new `request_id` (the trace id
  for a root, a fresh draw for a continued trace), `request_id_hex` and `continued`.
  `Ctx::stamp_envelope` and `trace::access` write `request_id_hex`, and `trace_id`/`span_id` for a
  sampled **or continued** trace. `Core\Server::traceId()` still reads the trace id. Four rules say
  so (`a-trace-id-exists-for-every-request`, its title too, `a-log-record-carries-…`,
  `sampling-is-head-based`, `an-inbound-traceparent-is-continued`). Tests
  `with_no_exporter_a_sampled_header_records_nothing` and
  `a_continued_trace_gets_a_request_id_the_client_did_not_choose` are new; the edited trace, log and
  directives tests ran once, green.
- **LC-6, LC-7 (1befebd34).** `nvs_runtime::metrics::method_label` maps the nine RFC 9110 methods to
  themselves and anything else to `other`, inside `Registry::request`. `trace::access` and `serve.rs`'s
  `counted` treat `501` like a `4xx`: `Info`, and only with `[log] access` on. A program's own `501`
  is treated the same; the access-log rule says why. Tests `an_unknown_verb_is_labelled_other` and
  `a_501_is_an_access_line_at_info` ran once, green.
- **LC-5 (55d35f3de).** `blocking::run` and the pinned `Request::answer` read the pool thread's CPU
  clock (`cpu_now`, `cpu_since`) and send the spent time back with the answer; the task calls the
  new `watchdog::charge_running`, which adds it to the running tree's `Charge::offloaded` (a second
  word; `Charge::at` adds it). Test `cpu_spent_on_the_blocking_pool_is_charged_to_the_request` ran
  once, green, with a 60 ms job. The audit's second fix, a SQLite progress-handler deadline from the
  remaining CPU time, is not in the decision and was not done (see Backlog).
- `cargo build`, `cargo clippy --all-targets -- -D warnings` and `bun nv rules --check` are green.

Admission, for the record (the previous session):

- **A connection between requests is not counted.** Each core's gauge is now `LIVE + DETACHED −
  IDLE` (`crates/nvs-runtime/src/budget/gauge.rs`, `report`, `move_idle`). `IDLE` is a thread-local
  sum of what idle connection trees hold above their base. `TreeState` has three new words
  (`safepoint.rs`, after `balance`): `base`, `idling` (`UNBASED`/`COUNTED`/`IDLE` in `budget.rs`)
  and `idle_held`. `budget::begin_idle`, `end_idle`, `count_from_here` and `settle_idle` sit before
  `publish` in `budget.rs`. `ArmedStack::take` settles an idle tree at each park.
- **Who calls it.** `Ctx::between_requests` (`safepoint.rs`, before `join_tree`) marks the tree idle
  where the running stack is its turn, and sets the new `Ctx::idles_tree` field. `Ctx::reroot` ends
  the idle at its start and calls `count_from_here` after the swap. `Ctx`'s drop ends it too.
  `serve.rs` calls `between_requests` before the drive loop, when `sending` gives a place back, and
  at the end of `joined_when_ended`.
- **Why a base and not a running sum.** A block allocated on one kind of turn and freed on the other
  would make a running sum drift with requests served. The excess over the base is dropped at the
  next reroot, so nothing outlives one request. A fresh tree's balance starts where its spawner's
  stack stood (`leave_this_stack`), so its base is taken at its first idle (`UNBASED`). A tree with a
  running child (`TreeState::tasks > 0`, after-response work) sets nothing aside.
- Test `connection_buffers_between_requests_do_not_refuse_a_request` (`admit.rs`) ran once, green.
  It claims a gauge on its own thread, holds 64 MiB on an idle connection tree under a 32 MiB room,
  and is admitted; after `reroot` the same bytes refuse. It reads `budget::held_by_this_core` (new),
  so other tests' cores stay out. Before the base fix its first assert failed with 64 MiB published.
- `rule:http-server/admission-is-arithmetic-not-a-number` has the paragraph, the memory line and a
  Declined entry, and `guardedBy` names `gauge.rs`.

RT-4, for the record:

- **RT-4:** `html.rs`'s `Sink::push` asked one node's size (144 bytes), but the arena's doubling is
  the largest step: 8 MB of `<br>` held 393 MB against the 268 MB ceiling. The sink now asks, through
  `afford`, `room` and `grown`, for the doubling of the arena, of an element's `children` and of its
  `attributes`, for each attribute's bytes before `create_element` copies them, for each copied text
  (`copied`), and for a merged text's growth. A refusal still unwinds with `OverLimit` to `bounded`.
  Attack `tests/hostile/core/Html/parse/04-documents-larger-than-the-memory-limit.nvs` (`ends-early 3
  FATAL`) now stops at 158 MB with the 151 MB doubling refused. Rust test
  `many_siblings_stop_at_the_memory_limit_before_the_arena_doubles` fails without the fix (peak
  16.7 MB under an 8 MiB limit) and passes with it. `html5ever`'s tokenizer builds a tag's attribute
  vector before the sink sees it, so an attribute flood is charged only once it reaches the sink; that
  vector is bounded by the document's size.
- The rule's parser paragraph names `Core\Html::parse` and `::sanitize`, and `guardedBy` names
  `html.rs` and the attack. `Core\Html`'s figures were re-recorded. `bun nv proofs --run --id
  'Core\Html::parse'` (3 examples, 4 attacks), the two Html memory tests, `cargo build`, `cargo clippy
  --all-targets -- -D warnings` and `bun nv rules --check` are green.
PARSE-1, for the record:

- **PARSE-1, the shape the other three parsers follow.** A helper that returns `Err` while the
  request is over its limit is reported as the memory `FATAL` (`crates/nvs-runtime/src/abi.rs:496`),
  so a parser only has to stop. `csv.rs` asks `nvs_runtime::budget::affords(<the step's size>)` in
  front of each step: `affords_field` for a field's place in the record (the doubling when it is full),
  `row` for each entry and its string plus `out.affords_write()`, `parse` for each row's entry plus
  `rows.affords_write()`, and `RowReader::step` for each `CELL` chunk of a field that has not ended
  (`Step::OverMemory`). `over_memory(member)` is the fallback `Fault`.
- **Never ask zero, and never use `over_ceiling()` alone.** The budget's `CEILING` sits below
  `[limits] memory` by the 1 MiB limit-handler reserve. In that band `over_ceiling()` is true but
  `Ctx::memory_breach` is `None` unless a refusal was recorded, so the program saw the fallback
  wording. `affords(0)` records a refusal that reads "an allocation of 0 bytes was refused".
  `rule:errors/an-allocation-past-the-ceiling-is-refused-in-front-of-itself` now has a parser
  paragraph that says so, and names `csv.rs` and the attack in `guardedBy`. PARSE-2/3 and RT-4 extend
  that paragraph with their members. `db/execute.rs:477`'s `rows_fit` has the same band gap (not
  touched: no item owns it; see Backlog).
- The attack's step 5 parses 8 MB of `a\n` and ends `// hostile: ends-early 5 FATAL`. Under
  `nvs run` (256 MiB) it stops at 267 MB with "an allocation of 16 bytes was refused". The Rust test
  `records_past_the_memory_limit_stop_the_parse_at_the_limit` (short lines and one wide record, 1 MiB
  limit, peak under 2 MiB) fails without the fix, because the `Ok` arm reports no breach. Both ran
  once, green, and so did `Core\Csv`'s examples. The three `Core\Csv` figures were re-recorded.
- Decision d1 (captured output that is not UTF-8) is answered, recorded under *Standing decisions*
  and closed. Stage 3 already does it: `out.rs:224` and `test.rs:3034` throw `RuntimeError`.

Stage 5, for the record:

- **HTML-1:** `Ctx::write_body` (`crates/nvs-runtime/src/ctx/output.rs:324`) is how
  `Core\Response::text`, `::json`, `::bytes` and `::stream` (off a connection) write. At
  `script_depth == 0` with no capture open it is `write_output`. Otherwise it renders the bytes as
  `echo` renders a `string`: `nvs_render::html::escape` under the HTML carrier,
  `nvs_render::text::substitute` under the terminal one, and byte by byte for bytes that are not
  UTF-8, so `carried` still throws on them. `html` and `sendFile` stay verbatim. This differs from the
  item's first wording (a plain `string` result): it keeps `output`'s type and also covers
  `output: 'inherit'`, whose bytes `nvs_host::isolate::hand_over` appends raw. The rule
  `rule:security/capture-answers-the-carrier` says so and names the attack in `guardedBy`.
- `perf: stale` is likely for every feature whose source file stage 3 or 5 changed (see *Tests that
  may go red*). `bun nv proofs --record-perf --group '<group>'` re-records one group.

- **TAINT-2:** `Core\Response\Stream::write`'s chunk is `Union[Text(Sink), Blob(Sink)]`
  (`crates/nvs-stdlib/src/response.rs`). The card and `rule:security/response-body-is-one-typed-member`
  say why `text` may take `tainted` and a chunk may not. Step 2 of
  `tests/hostile/core/Response-Stream/write/01-parts-without-end.nvs` now escapes its query value with
  `Core\Html::escape` and `Core\Html::toSource`. Every other `.nvs` that streams checks clean.
- **TAINT-4:** `reject_tainted_ldap_attribute_names` (`crates/nvs-types/src/expr/quals.rs`), called
  from both instance-call paths in `calls.rs`, reports E0401 at a literal key that is not a literal, a
  constant or a variable with a clean type, at a spread, and at a non-literal array whose
  `keys_from_outside` is true. `rule:core-classes/ldap-filter-is-a-value` and `add`'s card say so.
- **EXT-4:** `reject_secret_extension_argument` (`quals.rs`), called from the static-call hook block,
  reports E0775 for a `secret` at any extension parameter that admitted it by type. A literal written at
  `mixed` is walked as written (`secret_written_in`), because its inferred entries are `mixed`.
- The three cases ran once by name, all green. `cargo build`, `cargo clippy --all-targets -- -D
  warnings` and `bun nv rules --check` are green.

- `Core\SignedCookie::open` answers `CoreTy::TaintedStr`, and its `$cookie` is `Qual::Neutral`
  (`crates/nvs-stdlib/src/signed_cookie.rs`). The module doc section is now "`open` answers
  `tainted string`". The card, `jwt.rs`'s doc, `signature.rs`'s and `core_lib.rs`'s roster tests and
  `html.rs`'s launderer roster agree. `rule:security/verification-does-not-launder` has no exception
  and declines one.
- **A side change to admission:** `admits_tainted_argument` (`crates/nvs-types/src/expr/quals.rs`)
  now admits a `tainted` argument at a contagious parameter whose return is all `mixed` (`mixed` or
  `array<mixed>`, `all_unchecked`). Text out of `mixed` is already tainted. Without it,
  `Core\Json::decode` of an opened cookie did not compile, and the cart example
  `docs/examples/core/SignedCookie/seal/03-a-shopping-cart-cookie.nvs` broke.
  `rule:security/taint-propagation` says so, and
  `tests/conformance/reject/a-tainted-argument-is-refused-where-the-answer-cannot-carry-it.nvst`
  now expects the `mixed` and `array<mixed>` calls to compile. `Core\Json::decodeAs<T>` over
  `tainted` text still does not compile. That is a usability gap (see Backlog), not a leak.
- The two old agreement cases were renamed to `…-both-answer-tainted.nvst` and rewritten, and the
  `guardedBy` paths in `data/rules/` follow them. The new reject case and the seven `SignedCookie`
  cases ran once by name, all green. Every `.nvs` calling `open` checks clean.
- `cargo build`, `cargo clippy --all-targets -- -D warnings`, `bun nv rules --check` and the three
  edited Rust tests are green. A full `nvs test tests/conformance` run showed 10 other reds, all from
  earlier stages (listed below).

Nightly run 39746060276 was cancelled whole and is not a failure of this tree.

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
listing every diagnostic code (`E0885`). This session adds: the website's generated `core.json` and
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
This session adds: any test that reads `held_by_cores` or `nvs_memory_bytes` from a served core
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

Stage 8 first, because d2's item waits on decision d3. Then d2 once d3 is answered.

0. d2, after d3 (`crates/nvs-server/src/admit.rs`, `Admission::admit` and `memory_fits`;
   `crates/nvs-runtime/src/budget/gauge.rs`, `report` and `publish_held`). Build what d3's answer
   names. For a share: `InFlight` holds it, and its `Drop` gives it back; `memory_fits` adds the
   shares to `used`. For option 3's hand-off: `crates/nvs-host/src/scheduler.rs:930` (`start`
   pushes a new task to the back of `ready`), and `Isolate::start` (`crates/nvs-host/src/isolate.rs:627`)
   is the caller that queues a request. Write why in
   `rule:http-server/admission-is-arithmetic-not-a-number`. Write the Rust test first: one core, a
   burst past the room, at least one `503` before any request allocates. Then run the `flood` row
   once (`bun nv bench-compare --scenarios --smoke --row flood`, about 15 minutes with the image
   build) and record Novis's evidence here. Set stage 7 done once it lands.

Stage 8: an extension instance ends with its request (`rule:packaging/a-fresh-instance-per-request`).

1. EXT-1: drop the tree's `Calls`, and the instance with them, when the tree ends
   (`crates/nvs-cli/src/extensions.rs:333`). Write the attack first: 200 requests that each call
   `Novis\Intl`, every answer correct and memory flat, a Rust test in `nvs-cli`.
2. EXT-2: wasmtime's instance limit from `max_in_flight` (`crates/nvs-ext/src/call.rs:1170`).
3. EXT-5 and EXT-6 (`extensions.rs:442`, `crates/nvs-ext/src/wasi/http.rs:488`), then EXT-7 to
   EXT-10 from `.agent-tmp/sec-audit/reports/wb-sinks.md:212`.


### Backlog

- `nvs_memory_bytes{scope="cache"|"process"}`, `nvs_tasks_in_flight` and `nvs_deferred_trees` are
  seeded and never written, so they read `0` or have no member. `every_core`'s gather is where a
  process-wide figure can be set without a request writing it. No item owns them.

- The audit's LC-5 also proposed a SQLite progress-handler deadline from the request's remaining CPU
  time, so a slow statement stops at `cpu_time` instead of only being charged after it ends. No item
  owns it; the charge alone stops the request at its next safepoint once the statement returns.

- Not checked: the stage 1 rerun saw `Response::sendFile("assets/logo.svg")` answer `500` in a copy of
  `.agent-tmp/sec-audit/bb-kitchen/app`, where the audit saw `200`. Serve that app as its
  `serve.log` shows and fetch `/logo` before stage 12 touches static serving.
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
