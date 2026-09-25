---
milestone: M8
---
# Loop goal 4 — `Core`'s capability-bearing half

Finish **M8 except its databases** — [docs/plan/m8.md](../../plan/m8.md) is the scope and this file does
not restate it. Every member of [01-core-library.md](../../spec/01-core-library.md) §§ 14, 16, 17 and the
compiler-facing entries of § 13 that were waiting on a capability, a reactor or an open handle **runs, is
gated, and is cased.**

This is goal `core-part-ii` of the parity program ([goals/README.md](README.md)) and it is the largest goal in it —
nine subsystems, each with its own ADR. It is deliberately not cut smaller: the stages below are grouped
by **file set**, and a session takes its group from one of them, so the goal's size costs sessions rather
than costing any one session context.

**This is where "PHP core parity" is actually earned.** Goal `core-depth` covered strings, arrays, numbers and
dates; this goal covers the filesystem, processes, crypto, the network, caching, logging and reflection —
which is most of what is left in `tools/data/php-builtins.txt`. Its migration floor is 85%, the largest
single jump in the program, and that is not incidental: it is the measure of the goal.

## Why here

The first of the two goals with an external precondition, and the cheaper one: only its stage 6
needs a container, the Redis `Core\Cache`'s coherent tier and `Core\RateLimit`'s limiter both open.
The preflight is here rather than at that stage because a run that reaches stage 6 six hours in and
only then discovers there is no daemon has spent the six hours.

## Stage 0 — the catch-up, and it is the regex tiering

1. **The regex engine is two-tier, and the tiering is a rule.**
   `rule:core-classes/regex-two-tiers`: a linear-time engine by default, backtracking only
   for patterns it cannot express and only under a **throwing step budget**, with a literal pattern's tier
   settled at compile time and the *pattern* argument refusing `tainted`. Goal `core-depth` shipped `Core\Regex`
   against one engine and folded its literal patterns; this changes what an existing pattern *does*, so it
   goes first — every case written in the meantime is written against the wrong engine, and a
   catastrophic-backtracking pattern that silently works today is a case that silently stops working later.
   `crates/nvs-stdlib/src/regex.rs`.

## Stage 1 — the floor

M4's, goal `core-depth`'s, goal `concurrency`'s and goal `governance`'s whole acceptance lists, **never traded.**

## Stage 2 — the filesystem

2. **`Core\IO`, whole.** [01-core-library.md](../../spec/01-core-library.md) § 14 is the member list and
   is authoritative: whole-file, streaming write, metadata, manipulation, resolution and handles. Every
   member is an `rule:security/tainted-qualifier` **path sink** and needs
   an `fs.read` or `fs.write` grant, which goal `governance` built the gate for.
3. **`resource` is never exposed**, and `FileMode` is an enum rather than a mode string — `rule:core-api/shape-rules` R14 and
   R11. That is what replaces `fopen`'s `"r+b"` grammar and the whole `fread`/`fgets`/`fseek`/`feof`
   family, and it is a rule rather than a convenience: a mode string is a parser at every call site.
4. **`Core\IO::within` is the path-traversal launderer**, and it is the one member in this stage worth
   naming separately. It resolves and then *proves containment* — the check `Core\Path::normalize`
   structurally cannot make, because normalising a path says nothing about where it ended up. A `tainted`
   path becomes trusted here or nowhere.
5. **`Core\IO::writeStream` is where a stream reaches disk** —
   `rule:core-classes/io-write-stream`. Any
   `Iterable<bytes>` to a file in one member; `overwrite` defaults to `false`; **a write that fails
   mid-stream removes the partial file.** Goal `server`'s uploads are its second caller and this is where it is
   built.
6. **No stream wrappers, no `php://`, no `phar://`, no user-registered protocols** —
   `rule:security/closed-doors`. There is no scheme dispatch anywhere in the filesystem or
   stream abstractions, and the refusal is by construction rather than by a blocklist.
7. **`Core\Env`**, and the standard streams `IO::stdin()`/`stdout()`/`stderr()`.
33. **`Core\Hash::ofFile(string $path, Digest $digest): bytes` — the constant-memory file digest.** A
    path sink needing `fs.read`, replacing `md5_file`, `sha1_file` and `hash_file`. Numbered out of
    sequence for item 15's reason in goal `core-depth`; it is in *this* stage rather than goal `core-depth`'s because it opens
    a file, and goal `core-depth` does not.

    **It exists because neither route without it is O(1) in the file.**
    `Hash::of(IO::read($p), …)` holds the whole file, and so does the `Hash::stream` loop that looks like
    the fix — [hash.rs:606](../../../crates/nvs-stdlib/src/hash.rs) concatenates its retained chunks at
    `finish`, so that route peaks at twice the file. Hashing in Rust with a fixed buffer sidesteps the
    whole problem, because the hasher never has to survive a Novis call boundary — which is the actual
    obstacle item 34 describes. `rule:packaging/an-artifact-is-one-immutable-content-addressed-file`'s mmap
    argument is the technique, already argued for the artifact cache and reused rather than re-decided.
    **What it spends:** one fixed buffer per call and no per-file growth, which is the point.

## Stage 3 — processes and the terminal

8. **`Core\Process`**, per `rule:core-classes/process-is-argv-only`: `run()`/`spawn()`,
   **argv-only with no shell-string form at all**, a Windows batch/PowerShell-target refusal, coroutine-
   suspending waits behind the `process.exec` gate. A tainted `$path` or `$argv` element is a compile-time
   diagnostic. The suspending wait is goal `concurrency`'s blocking pool — a child process has no readiness to wait on.
9. **`Core\Cli`, whole.** `rule:tooling/terminal-output-is-a-sink`, `rule:tooling/styling-is-a-value-not-a-grammar`, `rule:tooling/the-terminal-profile-resolves-once`, `rule:tooling/a-prompt-is-a-core-member`, `rule:tooling/in-place-output-is-a-scoped-live-region` and `rule:tooling/the-terminal-is-restored-on-every-exit-path`: terminal
   output is a **sink** that substitutes visibly, styling is a value type and never a grammar, streams and
   tty and colour depth and width resolve **once per process** over `anstream`, the five prompts read the
   controlling terminal rather than stdin and never block forever, and in-place output is a **scoped** live
   region. Two dependencies arrive with it — `crossterm`, `unicode-width` — both pure Rust, both owing a
   notice regeneration.
10. **§ 8's terminal restoration is part of this slice and not a follow-up.** `rule:errors/engine-floor`'s obligation: a
    live region that survives a panic is the defect the whole scoped shape exists to prevent.
11. **`Core\Command::run`, its generated `--help`, and its shell completions**, over the table goal `core-depth`
    built. Unlike `Core\Router` it *dispatches*, because a CLI has one entry point and no middleware
    question — `rule:tooling/commands-are-compiled` says so and it is the reason the two classes differ.

## Stage 4 — crypto, and the protocols built on it

12. **Hashing and crypto**, RustCrypto and **AEAD-only** per `rule:core-api/tier-roster`: `sha2`, `blake3`, `argon2`,
    `bcrypt`, `aes-gcm`. `Core\Password` rides them. `Core\Digest`'s roster is goal `core-depth` item 16 and is not
    redone here; this item's hashing half is the migration rows for `hash_algos`'s names over it.
13. **`Core\Secret::reveal()`**, which with the password-hashing helpers is one of exactly two ways a value
    legitimately loses `secret` — `rule:security/secret-qualifier`.
14. **`rule:security/protocol-roster`'s closed roster** — signed cookies,
    CSRF, TOTP, JWT — and § 4's rule that they are **correct by construction, not by careful use**. § 5 is
    the one most often got wrong: a verified signature does **not** launder. A JWT whose signature checks
    out still carries `tainted` claims.
34. **`Core\Hash\Stream` holds a compression state rather than its chunks.** Today it accumulates:
    `update` retains each buffer into a slot and `finish` hashes the concatenation
    ([hash.rs:606](../../../crates/nvs-stdlib/src/hash.rs)), so a program that streams to keep its memory
    flat gets the opposite — the footprint is the total fed, and twice that at `finish`. The surface is
    already `hash_init`/`update`/`final` and does not change, which is exactly what makes this safe to
    land late and why it was written this way first.

    **Its precondition is not scheduled anywhere, and that is the finding.** A live `sha2::Sha256` cannot
    live in a `Core` instance's slots — they hold values Novis holds, and an instance has no destructor —
    which is the wall [`identity_store`](../../../crates/nvs-stdlib/src/identity_store.rs) met and
    answered the same way. It needs **a runtime tag owning a native object with a release hook**, which
    no goal in the goals directory currently owns; a session that finds it needs one before this item records
    the decision under the standing rule below rather than reporting `BLOCKED`. `digest 0.10` cannot
    serialize a hasher into a slot instead — `crypto_common::SerializableState` is a later major.

    **The same tag buys `Hash::of` over an `Iterable<bytes>`**, so `Core\Request::bodyStream()` is hashed
    without buffering; goal `server`'s uploads are that caller, and item 5's `writeStream` is the shape.

## Stage 5 — the network

15. **`Core\Http\Client`**, over goal `concurrency`'s parking stream with `rustls` on it.
    `rule:http-server/allow-url-pins-the-address`: the URL parameter refuses `tainted`,
    `Core\Http::allowUrl` is the launderer **and it pins**, and the address policy lives in the
    **capability** rather than in the client (§ 5) — which is what makes it enforceable at all.
16. **Its finiteness half.** `rule:http-server/no-spelling-for-an-unbounded-wait`, `rule:http-server/retry-is-opt-in-jittered-and-closed` and `rule:http-server/a-non-idempotent-retry-needs-an-idempotency-key`: **there is
    no spelling for an unbounded wait**; retry is opt-in, jittered, and covered by the same deadline; and a
    `post` retried without an `idempotencyKey` is a **compile error**. The third is the one that needs
    `nvs-types` and is therefore easy to defer and then forget.
17. **Outbound `traceparent` propagation** — `rule:observability/an-outbound-call-propagates-traceparent`, which
    is the point at which a trace crosses a service boundary at all. The exporter itself is goal `server`'s.
18. **`Core\Net`** — sockets over the runtime's own reactor rather than a second event loop, which is
    `rule:core-api/tier-placement`'s own phrasing and the whole reason goal `concurrency` came first.

## Stage 6 — the two stores

19. **`Core\Cache`'s two tiers.** `rule:concurrency/cross-request-state-is-explicit`: `local()`
    per-core and in-process, whose contract states any entry may be absent at any time; `shared()` a real
    store over the network, Redis by default, gated by `net.connect`. **Two methods and not one API with a
    flag**, so the choice is visible in review. Values cross by the graph copy goal `concurrency` built — not a third
    mechanism.
20. **`Core\RateLimit`.** `rule:core-classes/ratelimit-two-members`: GCRA over the shared store as
    `consume`, per-core and approximate as `shed`, **no configuration at all**, and an unreachable store
    **throwing rather than deciding *allowed***. `governor` is the `shed` tier essentially unchanged; the
    shared tier's atomic script is ours, and § 6 says why no crate exists for it.

## Stage 7 — the escalation ladder's remaining half

21. **`Core\Fatal` and `Core\Log`**, plus the operator-configured `.nvs` error-handler script and the
    engine-native logging floor beneath it — `rule:errors/escalation-ladder`.
    `Core\Log`'s JSON-Lines writer **is the same native serialiser the engine floor calls directly**, so
    the two never disagree on log shape; two writers that agree today is the failure this item prevents.
22. **The floor is rotated and rate-limited**, per `rule:http-server/a-requests-blast-radius-is-bounded-at-four-tiers`'s amendment to `rule:errors/engine-floor` — the floor cannot
    fill the disk it writes to.
23. **`nvs check` refuses a `secret` operand at `Core\Log::write()`'s `fields` argument**, despite that
    parameter's open `array<string, mixed>` type. A qualifier check that an open type defeats is a
    qualifier check that does not work.

## Stage 8 — introspection

24. **`Core\Reflect` and `Core\Ast`**, built-in rather than extension-provided —
    `rule:tooling/reflection-and-source-parsing-are-core-features`. Reflective access enforces
    **the same visibility and hook checks ordinary code does**, and a parsed AST is typed, inert data with
    no path back into execution. `Core\Ast::parse()` is fuzzed with M1's own corpus.
35. **The checked property key, `property<T>` — after item 24, never before it.** The design is
    [ADR 0014](../../decisions/0014.md) § *Revisiting*'s entry, and this goal's one ADR slot
    writes it down — the type, its `as`-only source over `T`'s public properties, the union-typed read, the
    checked write, and the hooked/`readonly` choice that entry leaves open. Then, in the shape goal `governance`'s
    stage 10 gave `class<T>`: the atom beside `Array` (`crates/nvs-syntax/src/ast.rs:138`, parsed in
    `parse_type_atom`, `crates/nvs-syntax/src/parser/ty.rs:361`); `Ty` beside `Array`
    (`crates/nvs-types/src/ty.rs:64`) and the `string → property<T>` row in `conversion_row_exists`
    (`crates/nvs-types/src/expr/operators.rs:1732`), a literal operand decided at compile time; `E0235`
    moving from `parse_member_name` (`crates/nvs-syntax/src/parser/expr.rs:825`) to the checker's
    member-name split (`crates/nvs-types/src/expr/members.rs:789`), which keeps it for every operand that
    is not a `property<T>`; the access lowering to the name-keyed `SlotGet`/`SlotSet`
    (`crates/nvs-ir/src/ir.rs:675`, `:715`), whose write check's four known gaps
    (`crates/nvs-runtime/src/object.rs:140`) close by carrying the declared type beside
    `field_tags`/`secret_fields` (`:584`). The write calls item 24's shared visibility-and-hook check —
    that is why the order. `$obj->$m()` stays refused, `rule:classes/no-call-magic`. Its own file set.
25. **`Core\Attributes`' retrieval body** — `rule:attributes/structural-retrieval`, `rule:attributes/retrieval-folds-while-checking` and `rule:attributes/call-site-type-argument`. `get<T>` on a site with zero matches compiles to a constant `null`, one match compiles to that
    constant value **with no runtime lookup**, and more than one is a compile-time diagnostic naming
    `all<T>`. M4 landed the attach grammar and the call-site `<T>`; this is the half that reads.
26. **`Core\Decimal`, `Core\BigInt`, `Core\BigDecimal`** — the method surface around the `decimal` scalar
    M2–M4 already built (`rule:types/decimal`), including `divExact`,
    `divRound` and `allocate`, since division is the one place a decimal result may be inexact.

## Stage 9 — the launderers and the framework's privileged half

27. **`Core\Html::escape`/`Markup` and the `Core\Taint` launderers** — `rule:security/tainted-qualifier`. Every other stage in this
    goal produces `tainted` values; this is the stage that says how one stops being one.
28. **The privileged half of the framework**, each entry placed by `rule:core-api/tier-placement`'s own six tests rather than by
    a new rule — `rule:programs/framework-core-half`: `Core\Validate` (the
    launderer, and the one entry that could never be a package), `Core\Password`, `Core\Mail` against an
    operator-named SMTP endpoint, `Core\Storage` over local disk, `Core\Cldr::pluralCategory`.
    **`Core\Session` is not here** — it needs a request and lands in goal `server`.
29. **§ 17's documents and formats.**

## Stage 10 — the gates

30. **Every member of §§ 14, 16, 17 is registered, cased and classified.** The three tests goal `core-depth` built for
    Part I, run over Part II: registered, has a conformance case, carries a qualifier classification.
31. **The C-dependency enumeration.** A check listing the default binary's C dependencies and failing on
    any addition not recorded against `rule:packaging/a-c-dependency-answers-two-questions`'s two questions. m8.md names this as CI infrastructure
    rather than a fixture, and it is here rather than in goal `database` because goal `database` adds the one exception
    (SQLite) and a gate written by the goal that needs an exemption is a gate with an exemption in it.
32. **No class outside Tier 0 registers a name beginning `Core\`.** Same paragraph, same reason.

## Stage 11 — the teardown sweep

Added by the user's decision, after review found that
`rule:security/isolate-teardown-is-a-drain-then-a-sweep`'s drain frees only what the
refcounts say is dead: a cyclic object graph survived request and isolate teardown for the life of the
process — in the server, a leak growing with requests served. The § 2 sweep closes it. The in-flight
collector for a long-running CLI script that builds cycles *between* teardowns stays a separate, open
decision and is **not** this stage.

36. **The per-context live-object list.** Every `ObjHeader` links into its `Ctx`'s intrusive doubly-linked
    list at allocation and out at dismantle — objects only, since an object is the one shape that can
    close a cycle (`graph.rs`'s identity decision). What it spends, said in the module doc as `rule:programs/memory-priority`
    requires: two pointers per live object, and a few non-atomic stores at each object's birth and death.
    **The crossing relinks, and the relink is structural rather than remembered.** `Live::adopt` is the
    one place in the runtime an allocation changes owners (`graph.rs`'s carrier decision: no second
    traversal, no third place to forget), so the relink lives *inside* that one implementation — never
    at a call site, because then there is no call site to get it wrong. An object left on the source
    list would be swept by the child's teardown while the parent still holds it, a use-after-free.
    **A debug build makes any future drift loud everywhere:** `ObjHeader` carries its owning `Ctx`
    (one word, `debug_assertions` only), and dismantle and sweep assert it — so every `cargo test` run
    and every WSL valgrind leg, which are debug builds, turns a mislinked object into a panic naming
    the invariant instead of a silent corruption. A guard test pins the relink and a `#[should_panic]`
    test pins the assertion. `crates/nvs-runtime/src/object.rs`, `ctx.rs`, `release.rs`, `graph.rs`.
37. **The sweep at `Ctx::drop`.** After the root drain, whatever is still on the list is exactly the
    cyclic garbage; dismantle it through `crate::release`'s one worklist so native teardown runs — order
    inside a dead cycle is unobservable, because teardown runs no user code. The slice that lands this
    also rewrites the two known-gap notes (`lib.rs` known gap 7, `object.rs` § *Decision: no cycle
    collector*) and writes `examples/cycles.nvs`, **which springs item 36's trap on purpose**: it
    builds cycles at the top level *and* spawns an isolate that builds a cycle of its own and returns
    an object by the refcount-1 move, then reads that object after the child is gone — the one program
    shape where a missed relink is a use-after-free, run under the WSL valgrind leg that exists to see
    exactly that. `rule:observability/trace-events-carry-a-kind`'s `gc` trace event is **not** owed here — it attaches to the future
    collector's run routine, not to teardown.

## Stage 12 — the exit hook

Added by the user's decision: `rule:observability/script-on-exit`
— `Core\Script::onExit`, the end-of-script queue that closes the one ending no user code could observe
(`exit` runs no `finally`). The ADR is the whole contract: § 2's three endings fire the queue, § 3's
`FATAL` and cancellation never do, § 4 orders it after tier 2 and before native teardown, § 5 makes it
observe-only. Nothing here reopens `rule:classes/no-magic-methods` — a flat per-request queue is not a destructor walk.

38. **The member and its report.** `Core\Script` joins the registry with the five edits of conventions §
    *A `Core` member*: `onExit(callable $hook): void`, the `Script\ExitReason` enum (`Normal`,
    `ExitCall`, `UncaughtThrow`) and the readonly `Script\ExitReport` each hook receives (a hook may
    declare no parameter, as `Core\Fatal`'s handlers may). The queue lives on `Ctx` — what it spends,
    said in the module doc as `rule:programs/memory-priority` requires: one vec of closures per request, hooks and captures
    held to the end of the script, O(registrations). `crates/nvs-stdlib/src/script.rs`,
    `crates/nvs-runtime/src/ctx/hooks.rs`.
39. **The three endings drain it; the two terminations do not.** FIFO, once, as the last user code:
    after the last statement, after `exit`, and on the throw path after `onUncaughtThrow` — never on a
    `FATAL` (only `Core\Fatal::onLimit` sees one) and never on a cancellation. A hook's own throw is
    logged with the trace id and abandoned, `exit` inside a hook throws `RuntimeError`, a hook may
    register a hook, and the report is fixed before the first hook runs. `examples/onexit.nvs` proves
    the clean path end-to-end.

## Stage 13 — the bcrypt read-path

Added by the user's decision:
`rule:security/bcrypt-read-roster` — PHP's `PASSWORD_DEFAULT` was
never Argon2, so a migrating application's user table is a bcrypt column, and until now
`Core\Password::verify` threw at it. The ADR is the whole contract: § 1's two-shape read roster, § 2's
unchanged write side, § 3's always-`true` `needsRehash`, § 4's cost ceiling of 17, § 6's surviving
refusals.

40. **The read roster grows its second entry, in `crates/nvs-stdlib/src/password.rs`.** `verify` reads
    `$2y$`/`$2a$`/`$2b$` (never `$2x$`) through a pure-Rust bcrypt crate — picking it is pre-authorized
    under `rule:packaging/a-c-dependency-answers-two-questions` and owes the `[workspace.dependencies]` comment, `cargo deny check` and `python
    tools/gen-attribution.py`. A stored cost past 17 is refused before any work, in `MAX_M_COST`'s
    shape; `needsRehash` reads a bcrypt hash rather than throwing at it and answers `true` for every
    tag. The module doc's refusal section is rewritten to defer to `rule:security/an-unreadable-stored-hash-throws` as the roster's home,
    [docs/reference/core/Password.md](../../reference/core/Password.md) gains the migration paragraph,
    and the landed refusal case
    `tests/conformance/core/password-refuses-a-stored-value-that-is-not-a-hash-it-wrote.nvst` passes
    unchanged. What it spends, said in the module doc as `rule:programs/memory-priority` requires: ~4 KiB transiently per
    legacy `verify`, on the calling task, shrinking as the table upgrades itself.

## The harness this goal owes

**`python tools/gen-attribution.py --check-c-deps`** — item 31's enumeration. That tool already walks the
dependency tree to regenerate the notice file, so the C-dependency ledger is a second question over a walk
it already does, not a second walk. m8.md names it as CI infrastructure rather than a fixture.

## Acceptance

**This goal is retired: its checks are the floor stage of the live goal**, carried
there by the switch that left it and folded forward at every switch since.

## Standing decisions — pre-authorized, do not stop the loop for these

- **Decide and record; never `BLOCKED` for a design call.**
- **One ADR slot — item 35's `property<T>`, its first slice — and otherwise none, which is unusual
  enough to say why.** Every subsystem in this goal already has its
  own argued ADR — 0019, 0020, 0024, 0033, 0044, 0046, 0052, 0054, 0056, 0058, 0059, 0060, 0074, 0075,
  0076, 0082, 0086, 0105. A session that believes it needs a new number has almost certainly found a
  section it has not read.
- **Redis is the default shared store**, decided in `rule:core-api/two-cache-tiers`. Its client is a synchronous one over
  goal `concurrency`'s parking stream; picking it is pre-authorized under `rule:packaging/a-c-dependency-answers-two-questions`.
- **`Core\Cache::local` and `Core\RateLimit::shed` need no capability, and each declares that as a
  `None` row in `registry::CAPABILITIES`.** `rule:core-api/two-cache-tiers` is the decision behind both: their state is a
  map in the calling core's own thread, so nothing leaves the process, no name is resolved and no file
  is opened, and `rule:security/capability-question-is-grant-and-scope` has no door to put a check at. What is left to bound is footprint, which
  `rule:concurrency/cache-memory-is-charged-to-the-core`'s `nvs.toml` cap bounds and a boolean grant would not. Their siblings `shared()` and
  `consume` declare `net.connect`, which is what makes the two classes capability-bearing at all. `rule:testing/capability-closure-test` is the home of why this is a row rather than an entry on an allowlist.
- **A verified signature does not launder.** `rule:security/verification-does-not-launder`. This one is stated here because it reads like
  an oversight and is a decision.
- **An unreachable store throws; it never decides *allowed*.** `rule:core-classes/ratelimit-unreachable-store-throws`. The failure mode is the
  application's to choose, in the file that knows whether the limit is a quota or a lock.
- **Two writers that agree today is the bug.** `Core\Log` and the engine floor are one serialiser reached
  twice, exactly as goal `concurrency`'s graph copy is one walk reached twice.
- **`Core\Session`, `Core\Metrics`, `Core\Router::match` and `Core\Queue` are not in this goal.** The
  first three need a request and are goal `server`'s; `Core\Queue` needs a database and is goal `database`'s.
- **Picking every dependency but the two the user named** stays pre-authorized under `rule:packaging/a-c-dependency-answers-two-questions`. A new
  Rust dependency owes the `[workspace.dependencies]` line with a comment saying why, `cargo deny check`,
  and `python tools/gen-attribution.py`.

## What this goal does not touch

The databases (goal `database`). The listener and everything request-shaped (goal `server`). PHP's optional extensions —
`gd`, `intl`, `imap`, `zip` — which are M9's and are not parity work; a session that reaches one puts it
in `## Backlog`.
