---
looper: 1
title: "A request cannot crash the server, slip past the tainted check, or outgrow its caps"
summary: "Every finding of the 2026-10-21 audit that no other goal owns is fixed or written into its rule as declined: no request crashes the process through an uninitialised property, a rewritten readonly property or a string that is not UTF-8; no Core member or derived row returns a tainted input without its mark; parsers, connections, spans, sessions and extension instances stay under a cap that is per request or per in-flight request; pooled database connections carry no session state into the next request; Windows network paths, development static serving and the CSRF cookie read are closed; every fix has its attack; every rule this goal touched says what is true; and the checks below pass."
context: [docs/rules/testing/hostile-case-contract.md, docs/rules/http-server/an-unsafe-or-unbounded-default-is-a-defect.md]
stages:
  - id: 1
    title: "every repro runs again on main, and what is already fixed is struck"
    status: open
    context: [tests/hostile/README.md]
  - id: 2
    title: "a constructor cannot leak `$this` early or run twice, so the JIT's slot reads hold"
    status: open
    context: [crates/nvs-types/src/ctor_init.rs, crates/nvs-types/src/expr/assign.rs, crates/nvs-types/src/expr/calls.rs, crates/nvs-codegen/src/emit.rs, docs/rules/classes/definite-property-initialization.md, docs/rules/classes/readonly-is-written-once.md, docs/rules/classes/constructor-is-a-method-named-constructor.md]
  - id: 3
    title: "every string made from outside bytes is checked UTF-8"
    status: open
    context: [crates/nvs-stdlib/src/csv.rs, crates/nvs-stdlib/src/jwe.rs, crates/nvs-runtime/src/string.rs, tools/nv/cmd/lints.ts, docs/rules/types/string-is-utf8.md]
  - id: 4
    title: "one test walks every Core signature for a lost mark, and `Core\\Arr` and `Core\\Str` keep it"
    status: open
    context: [crates/nvs-stdlib/src/arr.rs, crates/nvs-stdlib/src/str.rs, crates/nvs-types/src/derive.rs, crates/nvs-types/src/expr/args.rs, docs/rules/security/taint-propagation.md, docs/rules/security/launderers-are-sink-named.md]
  - id: 5
    title: "signed cookies, streams, database rows and extension calls keep the mark"
    status: open
    context: [crates/nvs-stdlib/src/signed_cookie.rs, crates/nvs-stdlib/src/response.rs, crates/nvs-types/src/derive.rs, crates/nvs-types/src/ext_lib.rs, docs/rules/security/tainted-sources.md, docs/rules/security/secret-does-not-cross-an-extension.md, docs/rules/core-classes/db-column-types.md]
  - id: 6
    title: "an array a Rust parser builds stops at the request's memory ceiling"
    status: open
    context: [crates/nvs-runtime/src/array.rs, crates/nvs-runtime/src/budget.rs, crates/nvs-stdlib/src/csv.rs, crates/nvs-stdlib/src/json.rs, crates/nvs-stdlib/src/xml.rs]
  - id: 7
    title: "connections, spans, metrics and blocking work grow with requests in flight, never with requests served"
    status: open
    context: [crates/nvs-server/src/serve.rs, crates/nvs-server/src/admit.rs, crates/nvs-server/src/trace.rs, crates/nvs-server/src/otlp.rs, crates/nvs-host/src/blocking.rs, docs/rules/http-server/admission-is-arithmetic-not-a-number.md, docs/rules/observability/sampling-is-head-based.md]
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
    context: [crates/nvs-server/src/mount.rs, crates/nvs-server/src/statics.rs, crates/nvs-server/src/body.rs, crates/nvs-server/src/forwarded.rs, docs/rules/http-server/static-serving-is-one-policy.md, docs/rules/http-server/host-matching-is-on-the-host-part-only.md]
  - id: 13
    title: "the outbound client's redirects, the leftovers, and the audit's scratch is gone"
    status: open
    context: [crates/nvs-stdlib/src/http/transport.rs, crates/nvs-config/src/capability.rs, docs/rules/http-server/a-cross-origin-redirect-drops-credentials.md]
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

**Stage 3: bytes that are not UTF-8 throw where they enter.** `Core\Csv::rows` and `Core\Csv::parse`
throw `Core\Csv\Error` naming the line. `Core\Jwe::decrypt` throws its own error. No string is made
from unchecked bytes anywhere but `crates/nvs-runtime/src/string.rs`, and a `bun nv lints` rule
refuses `from_utf8_unchecked` and the unchecked `NvsStr` constructor outside it, with an allow list
that names each audited site and its reason. Replacing bad bytes with U+FFFD is declined: it changes
the data silently, and a later check can still be fooled by the overlong form `C0 BC`.

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
size is set to the head limit `MAX_HEADER_LINES` already pairs with (`crates/nvs-server/src/serve.rs:2363`).
A buffered response keeps its in-flight place until its body is written. Admission's memory check reads
the memory charged to requests, never the core thread's total. No cap on open connections is added.
With no exporter configured, no span is queued, and a client's sampled flag is read only when one is.
The `method` metrics label is the method for the nine standard methods and `other` for anything else.
A 501 for an unknown method is logged at `info`, since the client caused it. The log's `request_id` is
always the server's own, and the trace id stays a separate field. CPU a request spends on the blocking
pool is added to its own CPU time.

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

- [ ] Build debug once (`cargo build`), and run every CONFIRMED repro in `.agent-tmp/sec-audit/` against it. `.agent-tmp/sec-audit/reports/wb-runtime-memory.md:1`
- [ ] Strike each item below whose finding no longer reproduces, with the commit that fixed it. `docs/goals/security-hardening.md:1`
- [ ] Correct each item's `path:line` anchor that moved since a2b4854a4. `docs/goals/security-hardening.md:1`

### 2. a constructor cannot leak `$this` early or run twice, so the JIT's slot reads hold

**Does:** RT-1 and RT-2 no longer compile, and no program reaches a slot the constructor has not
written.

- [ ] RT-2: two-phase construction in the init check: every own property is assigned before `$this` escapes or `parent::constructor` runs (`rule:classes/definite-property-initialization`). `crates/nvs-types/src/ctor_init.rs:73`
- [ ] RT-1: `constructor` is callable only through `new` and as one `parent::constructor(...)` statement per path (`rule:classes/constructor-is-a-method-named-constructor`). `crates/nvs-types/src/expr/calls.rs:1040`
- [ ] RT-1: a second `parent::constructor(...)` on a path does not compile. `crates/nvs-types/src/expr/assign.rs:1333`
- [ ] RT-1: a dynamic call named `constructor` through `mixed` throws `Error`. `crates/nvs-codegen/src/emit.rs:3018`
- [ ] Port every program in the corpus the new rule refuses (`bun nv brief --where constructor` lists the examples; an `Explore` subagent lists the failing ones from one `nvs check` sweep). `docs/examples/README.md:1`
- [ ] The attacks: RT-2's `?restore=1` server shape and RT-1's readonly rewrite, each as a conformance reject case and a hostile case. `tests/hostile/README.md:1`

### 3. every string made from outside bytes is checked UTF-8

**Does:** RT-3's two-byte CSV throws an error for that request, and no other unchecked path is left.

- [ ] RT-3: `Core\Csv::rows` and `::parse` validate each field and throw `Core\Csv\Error` with the line (`rule:types/string-is-utf8`). `crates/nvs-stdlib/src/csv.rs:799`
- [ ] RT-3: `Core\Jwe::decrypt` validates its plaintext. `crates/nvs-stdlib/src/jwe.rs:1`
- [ ] The lint: `from_utf8_unchecked` and the unchecked `NvsStr` constructor only in `string.rs` and the audited allow list. `tools/nv/cmd/lints.ts:70`
- [ ] Fix every other site the lint names. `crates/nvs-runtime/src/string.rs:1050`
- [ ] The attack: an upload of `FF 0A` and of `C0 BC`, then the next request answers. `tests/hostile/core/Csv/rows/01-a-file-an-upload-form-could-send.nvs:1`

### 4. one test walks every Core signature for a lost mark, and `Core\Arr` and `Core\Str` keep it

**Does:** the keystone test exists and is green, and the `Core\Arr` and `Core\Str` members it named
carry the mark.

- [ ] The keystone test over every `Core` signature, with the launderer allow list (`rule:security/taint-propagation`). `crates/nvs-stdlib/tests/taint_signatures.rs:1`
- [ ] DB-1: `Core\Str::format`'s `%s` arguments make the result tainted. `crates/nvs-stdlib/src/str.rs:632`
- [ ] TAINT-1, DB-3: `keys`, `firstKey`, `lastKey`, `keyOf`, `findKey` and the key in a `map` callback keep the mark. `crates/nvs-stdlib/src/arr.rs:304`
- [ ] REQ-1: `Core\Arr::shapeAs<T>` fills a field from a tainted value only if the field is `tainted`. `crates/nvs-types/src/expr/args.rs:1950`
- [ ] TAINT-5: each Low item in the sinks report that the keystone test names. `.agent-tmp/sec-audit/reports/wb-sinks.md:191`
- [ ] The attacks: `sprintf`-style and key-name SQL injection do not compile. `tests/conformance/reject/every-sink-a-marked-value-can-reach-refuses-it.nvst:23`

### 5. signed cookies, streams, database rows and extension calls keep the mark

**Does:** the rest of the keystone test's list is fixed, and stored injection does not compile.

- [ ] CRYPTO-1, TAINT-3: `SignedCookie::open` returns `tainted string`, and the module doc says so. `crates/nvs-stdlib/src/signed_cookie.rs:149`
- [ ] TAINT-2: `Core\Response\Stream::write` is a sink and refuses `tainted`. `crates/nvs-stdlib/src/response.rs:603`
- [ ] REQ-2, DB-2: a text column fills only a `tainted string` property, and every example, page and app compiles (`rule:security/tainted-sources`). `crates/nvs-types/src/derive.rs:1466`
- [ ] EXT-4: a `secret` value does not cross an extension's `mixed` parameter (`rule:security/secret-does-not-cross-an-extension`). `crates/nvs-types/src/ext_lib.rs:304`
- [ ] TAINT-4: `Core\Ldap\Connection::add` refuses attribute names from a tainted array's keys. `crates/nvs-stdlib/src/ldap/registry.rs:464`
- [ ] The attacks: stored injection, a sealed visitor value, a stream echo and a `mixed` secret, each a reject case. `tests/conformance/core/signed-cookie-open-launders-where-jwt-verify-does-not.nvst:1`

### 6. an array a Rust parser builds stops at the request's memory ceiling

**Does:** an 8 MB body stops at the 128 MiB ceiling inside the parse, in each of the four parsers.

- [ ] PARSE-1: `Core\Csv::parse` and `::rows` grow arrays through the fallible form. `crates/nvs-stdlib/src/csv.rs:715`
- [ ] PARSE-2: `Core\Json::decode` and `Core\Request::json()`. `crates/nvs-stdlib/src/json.rs:1807`
- [ ] PARSE-3: `Core\Xml::parse` charges attribute and name growth. `crates/nvs-stdlib/src/xml.rs:1619`
- [ ] RT-4: `Core\Html::parse` refuses at the ceiling, not after it. `crates/nvs-runtime/src/budget.rs:920`
- [ ] The attacks: one 8 MB body per parser, the peak stays under the ceiling, and the next request answers. `tests/hostile/core/Csv/parse/01-documents-an-upload-form-could-send.nvs:1`

### 7. connections, spans, metrics and blocking work grow with requests in flight, never with requests served

**Does:** HTTP-1's idle connections and slow readers no longer flip admission, and LC-3, LC-5, LC-6,
LC-7 and LC-9 are closed.

- [ ] HTTP-1: set hyper's buffer size, and keep a buffered response's in-flight place until its body is written. `crates/nvs-server/src/serve.rs:2363`
- [ ] HTTP-1: admission reads memory charged to requests, not the thread's total (`rule:http-server/admission-is-arithmetic-not-a-number`). `crates/nvs-server/src/admit.rs:1`
- [ ] LC-3, LC-9: no span queued without an exporter, the client's flag read only with one, and `request_id` always the server's (`rule:observability/sampling-is-head-based`). `crates/nvs-server/src/trace.rs:278`
- [ ] LC-6, LC-7: the `method` label is closed, and a 501 logs at `info`. `crates/nvs-server/src/serve.rs:1696`
- [ ] LC-5: blocking-pool CPU is charged to its request (`rule:http-server/a-core-is-never-blocked-on-a-syscall`). `crates/nvs-host/src/blocking.rs:349`
- [ ] LC-8: try the native stack overflow shapes in the lifecycle report; if the process ends, the deep helper checks the stack it has left and throws. `crates/nvs-host/src/stack.rs:85`
- [ ] The attacks: 500 idle connections with a large head, 100 slow readers, then a normal request answers `200`; Rust tests beside `admit.rs`. `crates/nvs-server/src/admit.rs:1`

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
- [ ] HTTP-4: a malformed chunked body answers `400`. `crates/nvs-server/src/body.rs:197`
- [ ] HTTP-5: `X-Forwarded-Proto` from a trusted peer that never set it is ignored (`rule:http-server/x-forwarded-proto-sets-the-scheme-and-hsts-only`). `crates/nvs-server/src/forwarded.rs:466`
- [ ] RS-5: the Declined entry for a hard link. `docs/rules/http-server/static-serving-is-one-policy.md:1`
- [ ] The attacks: `GET /nvs.toml`, `/.env` and `/data.sqlite` in development mode, a rebinding `Host`, and the static policy rows; Rust tests beside `mount.rs`. `crates/nvs-server/src/mount.rs:1586`

### 13. the outbound client's redirects, the leftovers, and the audit's scratch is gone

**Does:** the client's redirects follow RFC 9110, the remaining small items are fixed, and
`.agent-tmp/sec-audit/` is deleted.

- [ ] A 303 becomes a `GET`, and a 301 or 302 after a `POST` becomes a `GET`; a cross-origin hop drops the body (`rule:http-server/a-cross-origin-redirect-drops-credentials`). `crates/nvs-stdlib/src/http/transport.rs:2361`
- [ ] NET-1: the two IPv6 forms that carry private addresses are denied (`rule:security/net-address-policy`). `crates/nvs-config/src/capability.rs:502`
- [ ] Correct the comment that says an unquoted attribute is safe. `crates/nvs-stdlib/src/html.rs:119`
- [ ] Raise `wasmtime` and `cranelift`'s minimums to the fixed releases in the lock file. `Cargo.toml:98`
- [ ] Run the HTML-escaping probes the audit never reported on, and fix or record each result here. `.agent-tmp/sec-audit/wb-stdlib-request/html/app.nvs:1`
- [ ] Delete `.agent-tmp/sec-audit/`. `.agent-tmp/sec-audit/BRIEF.md:1`

### 14. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Written on 2026-10-22 from the audit at a2b4854a4. Nothing is built yet. The repro programs and reports
are under `.agent-tmp/sec-audit/`, and stage 13 deletes them.

### Next

Stage 1: build debug once, re-run every CONFIRMED repro with five read-only subagents, strike what is
already fixed, and correct the anchors. Then stage 2, starting at `crates/nvs-types/src/ctor_init.rs:73`.

### Backlog
