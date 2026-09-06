# `rule:statements/no-host-populated-variables` — There are no superglobals; request, session, environment and CLI state are `Core` accessor classes

- **Status:** Accepted
- **Date:** 2026-08-20
- **Scope:** PHP's superglobal variables (`$GLOBALS`, `$_SERVER`, `$_GET`, `$_POST`, `$_COOKIE`, `$_FILES`,
  `$_REQUEST`, `$_SESSION`, `$_ENV`), the CLI SAPI's `$argv`/`$argc`, and Novis's own `$_ARGS` from
  [ADR 0006](0006-isolated-script-execution.md); the `Core\Server`, `Core\Request`, `Core\Session`,
  `Core\Cli` and `Core\Script` classes; what `Core\Env` gains beyond the `EOL` constant
  [ADR 0011](0011-functions-and-constants-are-class-members.md) already gave it
- **Amends:** [0006](0006-isolated-script-execution.md) — the spawn-script surface's `$_ARGS` becomes
  `Core\Script::args()`, the same rename this ADR gives every other superglobal; the "not shared" table's
  superglobal row is restated as a **thrown error** on `Core\Request`/`Core\Server`/`Core\Session` inside a
  spawned isolate, not silently-empty state.
  [0007](0007-explicit-type-system.md) § 6 — every `$_GET`/`$_POST`/`$_ARGS` example becomes the matching
  `Core\Request`/`Core\Script` call; the shape (`array<mixed>`, landing in `mixed`, checked out with `as`) is
  unchanged.
  [0008](0008-static-and-global.md) § 2 — the *superglobal* row is dropped from the exhaustive storage
  table. Host-populated request/session/CLI state is ordinary class-static state on a `Core` class — the
  table's existing **class static property** row, populated by the host at isolate construction instead of
  by a user initialiser — so no new row is needed; the list stays exhaustive at four rows, not five.
  [0009](0009-string-and-bytes.md) § 4 — the `$_GET`/`$_POST`/`$_SERVER` mention becomes
  `Core\Request`/`Core\Server`.
  [0010](0010-enums-are-a-value-type.md) — the `$_GET['status'] as Status` example becomes
  `Core\Request::query('status') as Status`.
  [0011](0011-functions-and-constants-are-class-members.md) — `Core\Server`, `Core\Request`, `Core\Session`,
  `Core\Cli` and `Core\Script` join the illustrative domain-class roster its *Revisiting* section already
  says is incomplete.
- **Amended by:** 0023, 0024, 0046, 0091, 0124, 0139

> **In short:** PHP populates `$_SERVER`, `$_GET`, `$_POST`, `$_COOKIE`, `$_FILES`, `$_SESSION`, `$_ENV` and
> `$GLOBALS` ambiently — a script never declares them, they are simply present, and `$GLOBALS` additionally
> exposes every top-level variable in the whole program as one mutable array keyed by name. Novis has none of
> that: **no variable is ever populated by the host.** `$GLOBALS` and `$_REQUEST` are dropped outright, with
> no replacement. Every other superglobal becomes a `static` method on a reserved `Core` class, populated
> per isolate by the host rather than by a user initialiser: `Core\Server` for server metadata and headers
> (`$_SERVER`), `Core\Request` for query/post/cookie/file input (`$_GET`/`$_POST`/`$_COOKIE`/`$_FILES`),
> `Core\Session` for session data (`$_SESSION`, no auto-start), `Core\Env` for environment variables
> (`$_ENV`, joining the `EOL` constant ADR 0011 already gave it), and `Core\Cli` for the CLI SAPI's process
> arguments (`$argv`/`$argc`). Novis's own spawn-script argument variable — `$_ARGS` from
> [ADR 0006](0006-isolated-script-execution.md) — is the same shape of ambient variable this ADR closes, and
> gets the same treatment: `Core\Script::args()`. Inside a spawned isolate, `Core\Request`, `Core\Server` and
> `Core\Session` **throw** rather than silently returning empty or the parent's data — an isolation-boundary
> violation should fail loudly, not look indistinguishable from a genuinely empty request. The exact method
> signatures are stdlib design, due with the milestone that implements each class
> ([ADR 0011](0011-functions-and-constants-are-class-members.md)'s *Revisiting* already says the roster is
> illustrative); what is fixed here is that no bare variable is ever ambiently populated, and every one of
> these facts is reached through a declared class exactly like the built-ins ADR 0011 already relocated.

## Context

- PHP treats "a script is handed data from outside" as magic variables, not declarations: superglobals
  are simply *there*, populated by the SAPI, mutable, and readable from any scope without a `global`.
- **`$GLOBALS` is a second, worse door onto the problem `global` already opens**
  (`rule:statements/static-is-a-member-modifier`): any function at any depth can read or overwrite any top-level
  variable by name, with no keyword and no declaration at all.
- **The request-input superglobals deliver untrusted data with no declared boundary.**
  [ADR 0007](0007-explicit-type-system.md) § 6 already treats `$_GET`/`$_POST`/`$_SERVER` as untyped
  input, but it still arrives as a bare, ambiently-populated variable rather than a declared, traceable
  entry point (contrast a grep-able `Core\Str::` call, [ADR 0011](0011-functions-and-constants-are-class-members.md)).
- **`$_SESSION`/`$_ENV` add ambient mutable/host-configuration state**, and `$_REQUEST` adds a third
  failure mode: it merges `$_GET`/`$_POST`/`$_COOKIE` in a `php.ini`-configurable order, so the same key
  can silently mean a different source on different servers.
- Not a new argument: `rule:statements/static-is-a-member-modifier`'s storage table is exhaustive, and a bare
  ambiently-populated variable does not fit any row in it — this ADR closes the one place PHP's magic
  variables were still standing.

## Decision

**No variable is ever populated by the host. Every fact PHP hands a script through a superglobal is instead
returned by a `static` method on a reserved `Core` class, and `$GLOBALS` / `$_REQUEST` are dropped with no
replacement at all.**

### 1. The mapping

| PHP superglobal | replacement | notes |
|---|---|---|
| `$GLOBALS` | **none** | dropped outright — see *2* |
| `$_REQUEST` | **none** | dropped outright — see *3* |
| `$_SERVER` | `Core\Server` | server metadata (`REQUEST_METHOD`, `REMOTE_ADDR`, …) and request headers |
| `$_GET` | `Core\Request` | query-string input |
| `$_POST` | `Core\Request` | parsed body input |
| `$_COOKIE` | `Core\Request` | cookie input |
| `$_FILES` | `Core\Request` | uploaded-file metadata |
| `$_SESSION` | `Core\Session` | see *4* — no auto-start, backing store deferred |
| `$_ENV` / `getenv()` | `Core\Env` | joins the `EOL` constant [ADR 0011](0011-functions-and-constants-are-class-members.md) already gave this class |
| `$argv` / `$argc` | `Core\Cli` | CLI SAPI only — see *5* |
| `$_ARGS` (Novis, [ADR 0006](0006-isolated-script-execution.md)) | `Core\Script::args()` | not a PHP superglobal, but the same shape of ambient variable — see *6* |

`Core\Server` and `Core\Request` are two classes, not one, matching the split PHP itself already draws
between "facts about the server and the request" and "input the client sent": exactly the [ADR 0011](0011-functions-and-constants-are-class-members.md) principle of one domain class per PHP-grouping-shaped
concern, not one class holding everything. The exact method signatures on each class — a single-key lookup,
a whole-array accessor, or both — are stdlib design due at the milestone that implements them ([ADR 0011](0011-functions-and-constants-are-class-members.md)'s *Revisiting* already flags the roster as
illustrative), not fixed by this ADR. What is fixed: each is a `Core` class, each method is `static`, and
the value handed to user code keeps the shape [ADR 0007](0007-explicit-type-system.md) § 6 and
[ADR 0009](0009-string-and-bytes.md) § 4 already decided — structured input as `array<mixed>`, a scalar
payload not yet asserted to be text as `bytes`.

```php
use Core\Request;

uint $id = Request::query('id') as uint;   // throws on "abc", "-1", "" — never quietly 0, exactly as
                                            // ADR 0007 § 6 already specifies for the old $_GET
```

### 2. Why `$GLOBALS` has no replacement at all

Every other superglobal is *data the host hands the script*; `$GLOBALS` is different in kind — it is a
reflective view onto the script's *own* variable table, keyed by name, mutable in both directions. Once
`rule:statements/static-is-a-member-modifier` makes a top-level variable a local of the script's own frame,
unreachable from any function without `global`, there is nothing left for `$GLOBALS` to expose that isn't
already reachable the declared way: a `static` property, a constant, an object property, or a parameter. A
replacement class would have to either (a) enumerate the script's locals reflectively, which no other part
of the language does or needs, or (b) become a second, informally-scoped static-property bag that competes
with the one `rule:statements/static-is-a-member-modifier` already settled on. Both are worse than the status quo of
"declare a `static` property and use its name," so nothing replaces it.

### 3. Why `$_REQUEST` has no replacement at all

`$_REQUEST`'s only job is merging three sources whose read-site names — `Core\Request::query()`,
`::post()`, `::cookie()` — already say, unambiguously, which one a value came from. A merged accessor would
have to either pick one fixed order (and quietly re-derive the `request_order` footgun the day someone
expects PHP's configurable one) or take an order as an argument (at which point it is no shorter than calling
the specific method). Every real use of `$_REQUEST` is "I don't care which source, give me anything named
`id`" — the one habit [ADR 0007](0007-explicit-type-system.md) already treats as the failure mode, applied to
provenance instead of to type.

### 4. `Core\Session` — what this ADR fixes, and what it defers

Fixed here: there is no ambient `$_SESSION` array and no implicit `session_start()`. A script calls
`Core\Session::start()` (or an equivalent that resumes a given session id) before reading or writing session
state, so "this request uses sessions" is a line in the source rather than a fact discoverable only by
grepping for `$_SESSION`.

The mechanics are **not** this ADR's, for the same reason [ADR 0008](0008-static-and-global.md)
§*Alternatives rejected* declined to smuggle a memoisation cache into a decision about a keyword: this
ADR fixes the *shape* — a class, explicit start, no ambient array — and the storage backend, its
selection via an [ADR 0005](0005-config-changeability.md)-style directive, locking semantics and garbage
collection are a real feature with their own questions. All four are answered in
[ADR 0139](0139-a-session-is-a-record-its-store-issued.md): the backends are the shared cache tier and
the database, with the local tier refused at boot naming
[ADR 0059](0059-cross-request-state-is-explicit.md) § 4; there is no lock; and expiry is the store's own
rather than a sweeper's.

### 5. `Core\Cli` is CLI-SAPI-only, and says so loudly

`Core\Cli::args()` and `::argc()` mirror `$argv`/`$argc` and are valid only when the process is running as
the CLI entry point (`nvs run`). Calling either while serving HTTP (`nvs serve`) **throws**, per
`rule:errors/propagation`'s checked-status propagation, rather than PHP's silent absence
(`$argv` under `php-fpm` is simply unset, which is its own class of "works until it doesn't" bug). Within a
request tree, `Core\Cli::args()` returns the same value at every depth — it reflects how the *process* was
invoked, a process-wide fact rather than a per-isolate one, so `spawn script` does not need to fake or
suppress it the way it does for request state.

### 6. `Core\Script::args()` replaces `$_ARGS`, for the same reason as everything else

[ADR 0006](0006-isolated-script-execution.md) introduced `$_ARGS` as "a fresh superglobal" for a spawned
isolate to receive its arguments — Novis's own construct, not inherited from PHP, but the identical shape of
ambient, undeclared variable this ADR closes for every PHP one. Consistency, not a PHP compatibility
concern, is the reason it changes too: `Core\Script::args(): mixed` is the deep-copied value the current
isolate was spawned with, and `null` where there was none — a child spawned without the option, and the
root script, which nothing spawned. The type is `mixed` rather than `array<mixed>` because `spawn script`'s
`args:` accepts any value that can cross ([ADR 0023](0023-clone-serialize-and-cross-boundary-copy.md) § 2
decides that at run time, so nothing narrows the option at the call site), and `null` rather than an empty
array because a program that wrote `args: []` said something a program that wrote no option did not.
[ADR 0006](0006-isolated-script-execution.md)'s provisional-syntax caveat already says the exact
spelling is a M5 question; this ADR fixes that the spelling is a method call, not a variable, matching every
other row in this table.

### 7. Inside a spawned isolate, request/server/session access throws

[ADR 0006](0006-isolated-script-execution.md)'s "not shared with the parent" table already lists
superglobals as fresh per isolate, and its M5 verify criteria already require that "a child cannot read … a
superglobal." This ADR makes that concrete: inside a spawned isolate, `Core\Request::*`, `Core\Server::*`
and `Core\Session::*` **throw** an isolation-boundary error rather than returning empty values. A spawned
isolate is not a new inbound request — it is a child task inside the one that is already being handled — so
"there is no request here" would be false if these methods quietly returned empty arrays; a child that
genuinely needs facts from the request that spawned it receives them as ordinary arguments via
`with(args: […])`, deep-copied like any other value crossing the boundary — the graph-copy operation
[ADR 0006](0006-isolated-script-execution.md) § *Values cross by copy* and
[ADR 0023](0023-clone-serialize-and-cross-boundary-copy.md) define. `Core\Env` and
`Core\Cli` are not restricted this way: environment variables and process arguments are process-wide facts
already governed by the existing capability/config-overlay machinery
([ADR 0005](0005-config-changeability.md), [ADR 0006](0006-isolated-script-execution.md)), not per-request
secrets this ADR needs to newly wall off.

### 8. Diagnostics

Each rejection names its replacement, in the style `rule:statements/no-function-static-and-no-global` and
[ADR 0011](0011-functions-and-constants-are-class-members.md) § 4 already set:

- `$GLOBALS` → *`$GLOBALS` does not exist; declare a `static` property, a constant, or pass the value as a
  parameter*
- `$_REQUEST` → *`$_REQUEST` does not exist; read `Core\Request::query()`, `::post()` or `::cookie()`
  explicitly, so the source is visible at the call site*
- `$_GET` / `$_POST` / `$_COOKIE` / `$_FILES` → *use `Core\Request`*
- `$_SERVER` → *use `Core\Server`*
- `$_SESSION` → *use `Core\Session`, after calling `Core\Session::start()`*
- `$_ENV` / `getenv()` → *use `Core\Env`*
- `$argv` / `$argc` → *use `Core\Cli`*
- `$_ARGS` → *use `Core\Script::args()`*

### 9. The divergence this creates

Novis is a PHP-syntax superset that does not preserve PHP's superglobal *semantics* at all — a ported file
reading `$_GET`, `$_SESSION` or `$GLOBALS` does not run unconverted. The table below is this decision's
own; [divergences.md](divergences.md) is the register that indexes it beside every other:

| # | PHP | Novis |
|---|---|---|
| 1 | every superglobal is an ambient, host-populated variable, readable and (except `$GLOBALS`'s targets) writable from any scope without a `global` | none exist as variables; each is a `static` method call on a reserved `Core` class, reached through ordinary `use`/FQN resolution like any other class member |
| 2 | `$GLOBALS` exposes the entire top-level variable table by name; `$_REQUEST` merges `$_GET`/`$_POST`/`$_COOKIE` in a configurable order | both are dropped outright, with no replacement of any kind |

## Consequences

**Positive**

- There is exactly one way a script ever learns anything from outside itself: a declared class member, an
  object property, or a parameter — the same answer `rule:statements/static-is-a-member-modifier` and
  [ADR 0011](0011-functions-and-constants-are-class-members.md) already gave for state and for behaviour,
  now complete for *input* too.
- `$GLOBALS`, PHP's single most dangerous piece of ambient reflection, has no Novis equivalent to accidentally
  reintroduce later — there is no class it could be added back as without recreating the exact bag
  `rule:statements/static-is-a-member-modifier` closed.
- `$_REQUEST`'s provenance ambiguity cannot exist in Novis: every request-input read names its source.
- A reviewer sees exactly which `Core` class, and therefore which trust boundary, a line depends on — the
  same traceability [ADR 0011](0011-functions-and-constants-are-class-members.md) already gives built-in
  calls, now extended to untrusted input itself.
- The isolation boundary [ADR 0006](0006-isolated-script-execution.md) already promised for superglobals
  becomes a checked, catchable error instead of a silent, easily-misread empty result.

**Negative**

- Every PHP file with a bare `$_GET`/`$_POST`/`$_SERVER`/`$_SESSION`/`$_COOKIE`/`$_FILES`/`$_ENV` reference
  needs a rewrite at `nvs convert` (M11), on top of every other rewrite [ADR 0007](0007-explicit-type-system.md)
  and [ADR 0011](0011-functions-and-constants-are-class-members.md) already require. Unlike a built-in
  function rename, a `$_REQUEST` or `$GLOBALS` use has no mechanical one-line replacement — a human has to
  decide which specific source was meant, or how to re-home the global state.
- `Core\Session`'s real design (storage backend, locking, GC) is still open, and this ADR does not resolve
  it — see *Revisiting*.
- One more class of PHP program — anything using `register_globals`-adjacent patterns, `extract($_GET)`
  chief among them — has no straight-line conversion at all, matching `extract()`'s existing rejection in the
  M1 list.

## Alternatives rejected

- **Keep the superglobals as read-only, host-populated variables**, dropping only `$GLOBALS`. Rejected: a
  read-only bare variable still has no declared import or traceable dependency, and still does not fit
  `rule:statements/static-is-a-member-modifier`'s exhaustive storage table without a row added back for it.
- **A single `Core\Http` class for everything request-and-server-shaped.** Rejected per
  [ADR 0011](0011-functions-and-constants-are-class-members.md): one class for every unrelated concern is
  a global namespace with extra syntax.
- **Keep `$_REQUEST` as a merged accessor with a fixed, documented order.** Rejected: still hides the
  source at the read site, which is the actual problem — a fixed order is a smaller footgun, not a fix.
- **Auto-start sessions on first `Core\Session` access**, matching `session.auto_start`. Rejected: an
  implicit side effect triggered by a read is exactly the action-at-a-distance this ADR removes elsewhere.
- **Let `Core\Request`/`Core\Server` return empty values inside a spawned isolate**, matching PHP's CLI
  behaviour. Rejected in *7*: "empty" and "you cannot see this" are different facts, and collapsing them
  is the silent-wrong-answer failure mode [ADR 0007](0007-explicit-type-system.md) exists to close.

## Revisiting

- **The exact method signatures on `Core\Server`/`Core\Request`/`Core\Session`/`Core\Cli`/`Core\Script`**
  are stdlib design, due at M2 (parser/resolver needs the shape) and M7/M8 (the classes are actually
  implemented) — this ADR fixes the architecture, not the API.
- **`Core\Session`'s storage backend, locking and GC** are an open feature design, deferred to whichever
  milestone builds session support — except id acceptance, which
  [0124](0124-php-86-lands-as-four-refusals-and-one-session-rule.md) § 6 fixes: an id the store did not
  issue is discarded and replaced, always, so the backend must be able to answer "did I issue this id".
  Revisit this ADR only if the backend design turns out to need a second
  ambient variable or a second storage class, which would be a new argument, not a variation on this one.
- **Whether `Core\Cli` should instead be a set of methods on `Core\Env`**, since both are "facts about the
  process rather than the request" — deferred; splitting them for now mirrors PHP's own SAPI-specific vs.
  SAPI-agnostic split (`$argv` exists only under the CLI SAPI; `getenv()` works under any of them).

Verification, in the order it becomes possible:

- **M1**: the parser rejects `$GLOBALS`, `$_REQUEST` and every other superglobal spelling with a diagnostic
  naming the `Core` replacement in *8*.
- **M2**: name resolution treats `Core\Server`/`Core\Request`/etc. exactly like any other `Core` domain
  class — no bare-name fallback, no special-cased "superglobal" concept anywhere in the resolver.
- **M5**: `Core\Script::args()` is exercised by the `spawn script` conformance suite
  [ADR 0006](0006-isolated-script-execution.md) already specifies; a spawned isolate calling
  `Core\Request::query()` throws, added to that milestone's isolation-boundary test list alongside "a child
  cannot read a parent variable."
- **M7**: `Core\Server`/`Core\Request` are populated by the built-in HTTP server from a real request;
  `Core\Cli` is populated by `nvs run` and throws under `nvs serve`.
- **M8**: `Core\Session` gets its storage backend, per whatever the stdlib milestone decides.
