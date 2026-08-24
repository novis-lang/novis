# Next session prompt

Where the work stands right now. This file is **state**, overwritten every session and never appended to.
The traps and recipes that outlive a session are in [playbook.md](playbook.md); the rules that bind every
agent are in [AGENTS.md](../../AGENTS.md); `python tools/brief.py` is the rest of the orientation.

## State

**The live path is unchanged: `Core` breadth for Stage 3.** Six of seven fixtures produce their frozen
output; `examples/collect.mwl` is the first that does not. **No code has changed** — the last two commits
are documentation only, and **seven ADRs are written and none is built**.

**[ADR 0080](../adr/0080-the-audience-mwl-is-built-for.md) is the one to read first, because it reorders
the others.** MWL is built first for **multi-tenant and regulated platforms** — teams running code or data
they do not control — and that ranks the framework and the dependency story **above new `Core` breadth**.
Three consequences bind every later session: the pitch is isolation and qualifiers rather than speed (two of
the plan's three original premises have been answered inside PHP itself); **no document may claim PHP
compatibility**; and where two slices compete, the one serving that audience wins.

- **[0081](../adr/0081-packages-are-digests-resolution-is-a-maximum.md) — packages.** Identity is a BLAKE3
  digest, not a name. Registry *and* git, but git is **root-only**, so no transitive dependency can pull
  from a URL you never saw. **Minimal version selection**, so there is no solver and no unsolvable graph —
  the price is that a breaking release is a new package name. **No package code runs before your program
  does** (no install scripts, no build step, no macros). **Capabilities are granted per package**, one line
  at a time, and a `Core` call without a grant is a *compile* error — so a compromised dependency has no
  authority at all. Integrity is a lockfile plus a Go-style transparency log.
- **[0082](../adr/0082-the-first-party-framework.md) — the framework.** MWL ships one, split by
  [0051](../adr/0051-standard-library-tiers.md)'s **existing six tests** rather than a new rule: privileged
  halves in `Core` (`Validate` is the launderer, so it could never be a package), the opinionated layer as
  the **`mwl/web`** package providing the `Web` namespace. No ORM, no runtime container, and the language
  itself is the view layer. **`Web\Migration` is blocked** on § 7's open gap — migration semantics are
  deliberately undecided and need an ADR before that milestone can finish.
- **[0083](../adr/0083-persistent-connections-are-isolates.md) — WebSocket and SSE.** A connection is its
  own root isolate, opened by **naming a file** the way `spawn script` does, so 0006's grants/limits/args
  rules are reused whole. Inside it is an ordinary `while (receive())` loop — no callbacks, because
  suspension has no colour. `Core\Topic` fans out across cores and **closes a slow subscriber rather than
  blocking a publisher**.
- **[0084](../adr/0084-durable-background-jobs.md) — the queue.** A job is a row in a `Core\Db` table, which
  buys the one property a broker cannot: **`push` inside your transaction commits with it**. At-least-once,
  stated plainly; `SKIP LOCKED`-shaped claiming makes a fleet safe with no protocol of ours.
- **[0085](../adr/0085-openapi-is-generated-from-the-route-table.md) — API contracts.** OpenAPI 3.1 emitted
  while compiling from [0077](../adr/0077-compile-time-routing.md)'s table and
  [0071](../adr/0071-derived-codecs.md)'s codecs, so it cannot drift. An `#[Api]` that contradicts the code
  is a compile error, and `mwl api diff` gates a breaking change.
- **[0086](../adr/0086-core-cli-terminal-is-a-sink.md) — `Core\Cli`.** Terminal output is an
  [0024](../adr/0024-taint-tracking-for-injection-sinks.md) sink substituting control bytes with **visible**
  glyphs (`ESC` → `␛`) regardless of qualifier — the asymmetry with HTML escaping is that a control sequence
  is not text, so substituting it makes `echo` *more* faithful. Styling is the `Cli\Text` value type,
  prompts are `Core` members because raw mode is unreachable with FFI shut, and `#[Command]` builds the
  argument table while compiling.
- **[0087](../adr/0087-unbalanced-bidi-is-rejected-at-every-boundary.md) — Trojan Source, and the one ADR
  here with a slice that is buildable *now*.** A directional control that opens a scope and never closes it
  is a hard compile error in source and becomes `�` at both output sinks; **balanced** controls pass, which
  is what keeps legitimate Arabic and Hebrew working and is why a blanket ban was rejected. One predicate,
  three callers — the lexer (M1, re-opened), `Core\Html::escape` (M7), `Core\Cli`'s sink (M8) — so a second
  copy of it is a bug. Identifiers need nothing: [`lexer.rs`](../../crates/mwl-syntax/src/lexer.rs) is
  ASCII-only, which closes the homoglyph half structurally. It withdraws 0086 § 7's refusal to address bidi,
  which rested on a claim true only of a blanket ban.
- **[0067](../adr/0067-core-db.md) § 13 is new and is a rule, not a plan: connections are pooled per core.**
  Shared-nothing governs *program* state and a connection is host state, so pooling costs the model nothing.
  What it costs is a **reset that is a security boundary** — a connection that cannot be proven clean is
  destroyed, per backend, and PostgreSQL's reset deliberately preserves the statement cache while MySQL's
  cannot.
- **Do not start any of the seven while Stage 3 is open.** The plan's status block was deliberately **not**
  edited for the same reason the 0079 and 0086 sessions did not edit it: these are scheduled, not in flight,
  and `Open now` is already 8× its size target.
- Verified: `python tools/check-links.py` clean across 100 files, `python tools/verify.py` 4/4 green, 1291
  tests.

## Next

**`Core\Path` — spec § 11.** Unchanged and still the cheapest slice inside `examples/collect.mwl`: `join`
(variadic, which exists), `basename({withoutExtension})`, `extension(): ?string`, `SEPARATOR`, and no new
dependency. `docs/agent/loop-goal.md` § *Standing decisions* has the two-legs rule for `SEPARATOR` — a case
asserting a built path must normalize it.

## Backlog

- **`Core\Encoding`, `Core\Hash`, `Core\Uuid`, `Core\Csv`, `Core\Validate`, `Core\Random`, `Core\Out`,
  `Core\Uri::parseQuery`** — the rest of `examples/collect.mwl`; each needs a dependency picked under
  [ADR 0051](../adr/0051-standard-library-tiers.md) § 4 and its three obligations.
- **`Core\ObjectSet`/`ObjectMap`** — spec § 8, and the one item that needs *language* work first:
  `new Core\X<T>()` does not parse. `mwl_runtime::identity` is the comparison they need.
- **`Core\Time\Date`, `Core\Time\TimeOfDay`, `Core\Month`, `DateTime::date`/`timeOfDay`/`withTime`** —
  `time.rs`'s gap 1; the machinery all exists, so each is a registry row and a body.
- **`Core\Str::compare`, `chunk`, `lines`, `graphemes`, `codePoints`, `replaceAll`, `replaceRange`,
  `fold`, `normalize`, `fromCodePoint(s)`** — the rest of § 1. `mwl-stdlib`'s gap 1 is the list.
- **ADR 0069's `overlay`/`overlayDeep`/`underlay`/`appendAll`, plus `Arr::append`/`prepend`** — all
  declare the variadic that exists, so each is a registry row and a body.
- **A shape property read does not lower** — `mwl-ir`'s ADR 0036 § 4 gap: `$e->issues[0]->path` panics
  naming that ADR rather than reading slot 1, so an issue's own fields are unreadable from MWL.
- **A `Core` call that throws leaks a fresh string argument** — `mwl_ir::lower::landing_block`'s own
  *Known gap*: 50 loop iterations of a throwing `Core\Json::decode("…")` inside a `try` lose 50 blocks.
- **The migration-semantics ADR** — [0082](../adr/0082-the-first-party-framework.md) § 7 is the brief, and
  it blocks M16 rather than following it.
- **[0079](../adr/0079-testing-is-a-language-feature.md)'s first slice, after Stage 3** — `#[Test]` parsing
  plus the compile-time table (§ 1) and the generic `Core\Test` assertion roster (§ 4).
- **[0086](../adr/0086-core-cli-terminal-is-a-sink.md)'s M4S slice, after Stage 3** — the `#[Command]` table
  beside `#[Route]`'s, sharing [0061](../adr/0061-compile-time-autoload-and-program-discovery.md) § 3's
  enumeration.
- **[0085](../adr/0085-openapi-is-generated-from-the-route-table.md)'s M4S slice** — the emitter beside the
  same two passes it reads.
- **[0087](../adr/0087-unbalanced-bidi-is-rejected-at-every-boundary.md)'s lexer check** — the only slice of
  the eight ADRs that needs no milestone ahead of it: two counters over spans `mwl-syntax` already walks,
  one diagnostic, and that ADR's *Verification* section is the case list. Small enough to land beside a
  Stage 3 slice rather than instead of one.
