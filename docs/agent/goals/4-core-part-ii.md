# Loop goal 4 — `Core`'s capability-bearing half

Finish **M8 except its databases** — [docs/plan/m8.md](../../plan/m8.md) is the scope and this file does
not restate it. Every member of [01-core-library.md](../../spec/01-core-library.md) §§ 14, 16, 17 and the
compiler-facing entries of § 13 that were waiting on a capability, a reactor or an open handle **runs, is
gated, and is cased.**

This is order 4 of the parity program ([goals/README.md](README.md)) and it is the largest goal in it —
nine subsystems, each with its own ADR. It is deliberately not cut smaller: the stages below are grouped
by **file set**, and a session takes its group from one of them, so the goal's size costs sessions rather
than costing any one session context.

**This is where "PHP core parity" is actually earned.** Goal 1 covered strings, arrays, numbers and
dates; this goal covers the filesystem, processes, crypto, the network, caching, logging and reflection —
which is most of what is left in `tools/data/php-builtins.txt`. Its migration floor is 85%, the largest
single jump in the program, and that is not incidental: it is the measure of the goal.

## Stage 0 — the catch-up, and it is the regex tiering

1. **The regex engine is two-tier, and the tiering is a rule.**
   [ADR 0056](../../adr/0056-regex-engine-policy.md): a linear-time engine by default, backtracking only
   for patterns it cannot express and only under a **throwing step budget**, with a literal pattern's tier
   settled at compile time and the *pattern* argument refusing `tainted`. Goal 1 shipped `Core\Regex`
   against one engine and folded its literal patterns; this changes what an existing pattern *does*, so it
   goes first — every case written in the meantime is written against the wrong engine, and a
   catastrophic-backtracking pattern that silently works today is a case that silently stops working later.
   `crates/nvs-stdlib/src/regex.rs`.

## Stage 1 — the floor

M4's, goal 1's, goal 2's and goal 3's whole acceptance lists, **never traded.**

## Stage 2 — the filesystem

2. **`Core\IO`, whole.** [01-core-library.md](../../spec/01-core-library.md) § 14 is the member list and
   is authoritative: whole-file, streaming write, metadata, manipulation, resolution and handles. Every
   member is an [ADR 0024](../../adr/0024-taint-tracking-for-injection-sinks.md) **path sink** and needs
   an `fs.read` or `fs.write` grant, which goal 3 built the gate for.
3. **`resource` is never exposed**, and `FileMode` is an enum rather than a mode string — ADR 0063 R14 and
   R11. That is what replaces `fopen`'s `"r+b"` grammar and the whole `fread`/`fgets`/`fseek`/`feof`
   family, and it is a rule rather than a convenience: a mode string is a parser at every call site.
4. **`Core\IO::within` is the path-traversal launderer**, and it is the one member in this stage worth
   naming separately. It resolves and then *proves containment* — the check `Core\Path::normalize`
   structurally cannot make, because normalising a path says nothing about where it ended up. A `tainted`
   path becomes trusted here or nowhere.
5. **`Core\IO::writeStream` is where a stream reaches disk** —
   [ADR 0105](../../adr/0105-an-uploaded-file-is-a-stream-and-there-is-one-way-to-receive-it.md) § 4. Any
   `Iterable<bytes>` to a file in one member; `overwrite` defaults to `false`; **a write that fails
   mid-stream removes the partial file.** Goal 6's uploads are its second caller and this is where it is
   built.
6. **No stream wrappers, no `php://`, no `phar://`, no user-registered protocols** —
   [ADR 0052](../../adr/0052-closed-doors.md). There is no scheme dispatch anywhere in the filesystem or
   stream abstractions, and the refusal is by construction rather than by a blocklist.
7. **`Core\Env`**, and the standard streams `IO::stdin()`/`stdout()`/`stderr()`.

## Stage 3 — processes and the terminal

8. **`Core\Process`**, per [ADR 0044](../../adr/0044-core-process-argv-only-no-shell.md): `run()`/`spawn()`,
   **argv-only with no shell-string form at all**, a Windows batch/PowerShell-target refusal, coroutine-
   suspending waits behind the `process.exec` gate. A tainted `$path` or `$argv` element is a compile-time
   diagnostic. The suspending wait is goal 2's blocking pool — a child process has no readiness to wait on.
9. **`Core\Cli`, whole.** [ADR 0086](../../adr/0086-core-cli-terminal-is-a-sink.md) §§ 1–5 and 8: terminal
   output is a **sink** that substitutes visibly, styling is a value type and never a grammar, streams and
   tty and colour depth and width resolve **once per process** over `anstream`, the five prompts read the
   controlling terminal rather than stdin and never block forever, and in-place output is a **scoped** live
   region. Two dependencies arrive with it — `crossterm`, `unicode-width` — both pure Rust, both owing a
   notice regeneration.
10. **§ 8's terminal restoration is part of this slice and not a follow-up.** ADR 0020 § 4's obligation: a
    live region that survives a panic is the defect the whole scoped shape exists to prevent.
11. **`Core\Command::run`, its generated `--help`, and its shell completions**, over the table goal 1
    built. Unlike `Core\Router` it *dispatches*, because a CLI has one entry point and no middleware
    question — ADR 0086 § 6 says so and it is the reason the two classes differ.

## Stage 4 — crypto, and the protocols built on it

12. **Hashing and crypto**, RustCrypto and **AEAD-only** per ADR 0051 § 3: `sha2`, `blake3`, `argon2`,
    `bcrypt`, `aes-gcm`. `Core\Password` rides them.
13. **`Core\Secret::reveal()`**, which with the password-hashing helpers is one of exactly two ways a value
    legitimately loses `secret` — [ADR 0033](../../adr/0033-secret-qualifier-for-confidential-values.md).
14. **[ADR 0060](../../adr/0060-application-security-protocols.md)'s closed roster** — signed cookies,
    CSRF, TOTP, JWT — and § 4's rule that they are **correct by construction, not by careful use**. § 5 is
    the one most often got wrong: a verified signature does **not** launder. A JWT whose signature checks
    out still carries `tainted` claims.

## Stage 5 — the network

15. **`Core\Http\Client`**, over goal 2's parking stream with `rustls` on it.
    [ADR 0058](../../adr/0058-outbound-request-policy.md): the URL parameter refuses `tainted`,
    `Core\Http::allowUrl` is the launderer **and it pins**, and the address policy lives in the
    **capability** rather than in the client (§ 5) — which is what makes it enforceable at all.
16. **Its finiteness half.** [ADR 0074](../../adr/0074-http-defaults-safe-and-finite.md) §§ 5–7: **there is
    no spelling for an unbounded wait**; retry is opt-in, jittered, and covered by the same deadline; and a
    `post` retried without an `idempotencyKey` is a **compile error**. The third is the one that needs
    `nvs-types` and is therefore easy to defer and then forget.
17. **Outbound `traceparent` propagation** — [ADR 0076](../../adr/0076-observability-export.md) § 2, which
    is the point at which a trace crosses a service boundary at all. The exporter itself is goal 6's.
18. **`Core\Net`** — sockets over the runtime's own reactor rather than a second event loop, which is
    ADR 0051's own phrasing and the whole reason goal 2 came first.

## Stage 6 — the two stores

19. **`Core\Cache`'s two tiers.** [ADR 0059](../../adr/0059-cross-request-state-is-explicit.md): `local()`
    per-core and in-process, whose contract states any entry may be absent at any time; `shared()` a real
    store over the network, Redis by default, gated by `net.connect`. **Two methods and not one API with a
    flag**, so the choice is visible in review. Values cross by the graph copy goal 2 built — not a third
    mechanism.
20. **`Core\RateLimit`.** [ADR 0075](../../adr/0075-core-ratelimit.md): GCRA over the shared store as
    `consume`, per-core and approximate as `shed`, **no configuration at all**, and an unreachable store
    **throwing rather than deciding *allowed***. `governor` is the `shed` tier essentially unchanged; the
    shared tier's atomic script is ours, and § 6 says why no crate exists for it.

## Stage 7 — the escalation ladder's remaining half

21. **`Core\Fatal` and `Core\Log`**, plus the operator-configured `.nvs` error-handler script and the
    engine-native logging floor beneath it — [ADR 0020](../../adr/0020-error-escalation-ladder.md).
    `Core\Log`'s JSON-Lines writer **is the same native serialiser the engine floor calls directly**, so
    the two never disagree on log shape; two writers that agree today is the failure this item prevents.
22. **The floor is rotated and rate-limited**, per ADR 0106's amendment to ADR 0020 § 4 — the floor cannot
    fill the disk it writes to.
23. **`nvs check` refuses a `secret` operand at `Core\Log::write()`'s `fields` argument**, despite that
    parameter's open `array<string, mixed>` type. A qualifier check that an open type defeats is a
    qualifier check that does not work.

## Stage 8 — introspection

24. **`Core\Reflect` and `Core\Ast`**, built-in rather than extension-provided —
    [ADR 0019](../../adr/0019-reflection-and-ast-parsing-are-core-features.md). Reflective access enforces
    **the same visibility and hook checks ordinary code does**, and a parsed AST is typed, inert data with
    no path back into execution. `Core\Ast::parse()` is fuzzed with M1's own corpus.
25. **`Core\Attributes`' retrieval body** — [ADR 0046](../../adr/0046-attributes-shape-literal-metadata.md)
    §§ 4–6. `get<T>` on a site with zero matches compiles to a constant `null`, one match compiles to that
    constant value **with no runtime lookup**, and more than one is a compile-time diagnostic naming
    `all<T>`. M4 landed the attach grammar and the call-site `<T>`; this is the half that reads.
26. **`Core\Decimal`, `Core\BigInt`, `Core\BigDecimal`** — the method surface around the `decimal` scalar
    M2–M4 already built ([ADR 0054](../../adr/0054-decimal-scalar-type.md)), including `divExact`,
    `divRound` and `allocate`, since division is the one place a decimal result may be inexact.

## Stage 9 — the launderers and the framework's privileged half

27. **`Core\Html::escape`/`Markup` and the `Core\Taint` launderers** — ADR 0024. Every other stage in this
    goal produces `tainted` values; this is the stage that says how one stops being one.
28. **The privileged half of the framework**, each entry placed by ADR 0051's own six tests rather than by
    a new rule — [ADR 0082](../../adr/0082-the-first-party-framework.md) § 2: `Core\Validate` (the
    launderer, and the one entry that could never be a package), `Core\Password`, `Core\Mail` against an
    operator-named SMTP endpoint, `Core\Storage` over local disk, `Core\Cldr::pluralCategory`.
    **`Core\Session` is not here** — it needs a request and lands in goal 6.
29. **§ 17's documents and formats.**

## Stage 10 — the gates

30. **Every member of §§ 14, 16, 17 is registered, cased and classified.** The three tests goal 1 built for
    Part I, run over Part II: registered, has a conformance case, carries a qualifier classification.
31. **The C-dependency enumeration.** A check listing the default binary's C dependencies and failing on
    any addition not recorded against ADR 0051 § 4's two questions. m8.md names this as CI infrastructure
    rather than a fixture, and it is here rather than in goal 5 because goal 5 adds the one exception
    (SQLite) and a gate written by the goal that needs an exemption is a gate with an exemption in it.
32. **No class outside Tier 0 registers a name beginning `Core\`.** Same paragraph, same reason.

## The harness this goal owes

**`python tools/gen-attribution.py --check-c-deps`** — item 31's enumeration. That tool already walks the
dependency tree to regenerate the notice file, so the C-dependency ledger is a second question over a walk
it already does, not a second walk. m8.md names it as CI infrastructure rather than a fixture.

## Acceptance

**The checks live in [`4-core-part-ii.toml`](4-core-part-ii.toml), and only there.**

## Standing decisions — pre-authorized, do not stop the loop for these

- **Decide and record; never `BLOCKED` for a design call.**
- **No ADR slots, and that is unusual enough to say why.** Every subsystem in this goal already has its
  own argued ADR — 0019, 0020, 0024, 0033, 0044, 0046, 0052, 0054, 0056, 0058, 0059, 0060, 0074, 0075,
  0076, 0082, 0086, 0105. A session that believes it needs a new number has almost certainly found a
  section it has not read.
- **Redis is the default shared store**, decided in ADR 0059 § 1. Its client is a synchronous one over
  goal 2's parking stream; picking it is pre-authorized under ADR 0051 § 4.
- **A verified signature does not launder.** ADR 0060 § 5. This one is stated here because it reads like
  an oversight and is a decision.
- **An unreachable store throws; it never decides *allowed*.** ADR 0075 § 5. The failure mode is the
  application's to choose, in the file that knows whether the limit is a quota or a lock.
- **Two writers that agree today is the bug.** `Core\Log` and the engine floor are one serialiser reached
  twice, exactly as goal 2's graph copy is one walk reached twice.
- **`Core\Session`, `Core\Metrics`, `Core\Router::match` and `Core\Queue` are not in this goal.** The
  first three need a request and are goal 6's; `Core\Queue` needs a database and is goal 5's.
- **Picking every dependency but the two the user named** stays pre-authorized under ADR 0051 § 4. A new
  Rust dependency owes the `[workspace.dependencies]` line with a comment saying why, `cargo deny check`,
  and `python tools/gen-attribution.py`.

## What this goal does not touch

The databases (goal 5). The listener and everything request-shaped (goal 6). PHP's optional extensions —
`gd`, `intl`, `imap`, `zip` — which are M9's and are not parity work; a session that reaches one puts it
in `## Backlog`.
