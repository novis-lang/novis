# Next session prompt

Where the work stands right now. This file is **state**, overwritten every session and never appended to.
The traps and recipes that outlive a session are in [playbook.md](playbook.md); the rules that bind every
agent are in [AGENTS.md](../../AGENTS.md); `python tools/brief.py` is the rest of the orientation.

## State

**The live path is unchanged: `Core` breadth for Stage 3.** Six of seven fixtures produce their frozen
output; `examples/collect.mwl` is the first that does not. **No code has changed in the last several
commits** — they are documentation only, and **nine ADRs (0080–0088) are written and none is built.**

**[ADR 0080](../adr/0080-the-audience-mwl-is-built-for.md) is the one to read first, because it reorders
the others.** MWL is built first for **multi-tenant and regulated platforms**, which ranks the framework and
the dependency story **above new `Core` breadth**. Three consequences bind every later session: the pitch is
isolation and qualifiers rather than speed; **no document may claim PHP compatibility**; and where two
slices compete, the one serving that audience wins.

**[ADR 0088](../adr/0088-a-sink-is-an-instruction-and-the-default-refuses.md) is the newest and the one
with a live code consequence.** It replaces [0024](../adr/0024-taint-tracking-for-injection-sinks.md) § 4's
*list* of sinks with a predicate — *a parameter is a sink when its content becomes an instruction something
executes, rather than data something returns or frames* — and **flips the default: an unclassified
`string`/`bytes` parameter on a `Core` member refuses `tainted`**. Userland was already fail-closed
([`assign.rs`](../../crates/mwl-types/src/expr/assign.rs)); `mwl-stdlib`'s registry was the whole exposure,
and it carries no classification field yet, which is why `Core\Str::format`'s template is not the sink that
ADR now makes it. Also fixed there: which sink `echo` binds to in every context (the terminal sink is the
default; HTML is attached only by an HTTP request), and five typed `Core\Response` body members replacing
`write`, so a JSON body is no longer an `echo` the auto-escape sink corrupts.

The other seven, one line each: **[0081](../adr/0081-packages-are-digests-resolution-is-a-maximum.md)**
packages are digests, minimal version selection, per-package capabilities, no package code runs before
yours. **[0082](../adr/0082-the-first-party-framework.md)** the framework splits by 0051's six tests into
`Core` plus the `mwl/web` package; `Web\Migration` is **blocked** on § 7's open gap.
**[0083](../adr/0083-persistent-connections-are-isolates.md)** a WebSocket/SSE connection is a root isolate
named by file. **[0084](../adr/0084-durable-background-jobs.md)** a job is a `Core\Db` row, so `push`
commits with your transaction. **[0085](../adr/0085-openapi-is-generated-from-the-route-table.md)** OpenAPI
is emitted while compiling. **[0086](../adr/0086-core-cli-terminal-is-a-sink.md)** the terminal is a sink
substituting control bytes with visible glyphs regardless of qualifier; `Cli\Text` is the only raw path.
**[0087](../adr/0087-unbalanced-bidi-is-rejected-at-every-boundary.md)** an unterminated directional control
is a compile error in source and `�` at both sinks — one predicate, three callers.

**Do not start any of the nine while Stage 3 is open.** They are scheduled, not in flight.

Verified this session: `python tools/check-links.py` clean across 101 files, `python tools/verify.py` 4/4
green, 1291 tests. No Rust was touched.

## Next

**`Core\Path` — spec § 11.** Unchanged and still the cheapest slice inside `examples/collect.mwl`: `join`
(variadic, which exists), `basename({withoutExtension})`, `extension(): ?string`, `SEPARATOR`, and no new
dependency. `docs/agent/loop-goal.md` § *Standing decisions* has the two-legs rule for `SEPARATOR`.

## Backlog

- **`Core\Encoding`, `Hash`, `Uuid`, `Csv`, `Validate`, `Random`, `Out`, `Uri::parseQuery`** — the rest of
  `examples/collect.mwl`; each needs a dependency picked under
  [ADR 0051](../adr/0051-standard-library-tiers.md) § 4 and its three obligations.
- **`Core\ObjectSet`/`ObjectMap`** — spec § 8, and the one item needing *language* work first:
  `new Core\X<T>()` does not parse. `mwl_runtime::identity` is the comparison they need.
- **The registry's qualifier classification** — [0088](../adr/0088-a-sink-is-an-instruction-and-the-default-refuses.md)
  § 2: a per-parameter field on `mwl-stdlib`'s member rows, the fail-closed default, and the test that
  refuses an unclassified member. Lands with M4S's remaining sections, not as its own slice.
- **`Core\Time\Date`/`TimeOfDay`/`Month`, `DateTime::date`/`timeOfDay`/`withTime`** — `time.rs`'s gap 1;
  the machinery exists, so each is a registry row and a body. Same for the rest of § 1
  (`mwl-stdlib`'s gap 1) and ADR 0069's combination members.
- **A shape property read does not lower** — `mwl-ir`'s ADR 0036 § 4 gap: `$e->issues[0]->path` panics, so
  an issue's own fields are unreadable from MWL. Beside it, `landing_block`'s leak of a fresh string
  argument when a `Core` call throws.
- **[0087](../adr/0087-unbalanced-bidi-is-rejected-at-every-boundary.md)'s lexer check** — the only slice of
  the nine needing no milestone ahead of it: two counters over spans `mwl-syntax` already walks, one
  diagnostic, and that ADR's *Verification* is the case list. Small enough to land beside a Stage 3 slice.
- **The migration-semantics ADR** — [0082](../adr/0082-the-first-party-framework.md) § 7 is the brief, and
  it blocks M16 rather than following it.
