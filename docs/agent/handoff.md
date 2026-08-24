# Next session prompt

Where the work stands right now. This file is **state**, overwritten every session and never appended to.
The traps and recipes that outlive a session are in [playbook.md](playbook.md); the rules that bind every
agent are in [AGENTS.md](../../AGENTS.md); `python tools/brief.py` is the rest of the orientation.

## State

**The live path is unchanged: `Core` breadth for Stage 3.** Six of seven fixtures produce their frozen
output; `examples/collect.mwl` is the first that does not. **No code has changed in the last several
commits** — they are documentation only, and **ten ADRs (0080–0089) are written and none is built.**

**[ADR 0080](../adr/0080-the-audience-mwl-is-built-for.md) is still the one to read first, because it
reorders the others.** MWL is built first for **multi-tenant and regulated platforms**, which ranks the
framework and the dependency story **above new `Core` breadth**. Three consequences bind every later
session: the pitch is isolation and qualifiers rather than speed; **no document may claim PHP
compatibility**; and where two slices compete, the one serving that audience wins.

**[ADR 0089](../adr/0089-convert-is-one-rule-table-with-two-modes.md) is the newest**, and it is the one
that decides how a PHP codebase reaches MWL. `mwl convert` is **one rule table read through two modes**,
not two translators: every rule branch carries a tier — **E** proven identical, **D** a mechanical
destination that may differ, **N** no destination — and the default `--mode=equivalent` emits only E while
`--mode=runnable` also emits D under a `TODO(convert:<id>)`. An E claim is discharged by a differential
case against the PHP oracle or CI refuses it; determinism is a contract with seven named rules; every
non-trivia input byte leaves as code or as commented-out source. Its § 7 picks the PHP front end —
`mago-syntax` leading, `php-ast` the fallback, behind our own facade, dialects 7.0–8.5 — **subject to a
spike that ADR's *Verification* specifies and that nothing has run yet**. M11's enumerated rewrite
catalogue was folded into that table, so [the plan](../implementation-plan.md)'s M11 now points rather
than restates.

**[ADR 0088](../adr/0088-a-sink-is-an-instruction-and-the-default-refuses.md) is the one with a live code
consequence.** It replaces [0024](../adr/0024-taint-tracking-for-injection-sinks.md) § 4's *list* of sinks
with a predicate — *a parameter is a sink when its content becomes an instruction something executes* —
and **flips the default: an unclassified `string`/`bytes` parameter on a `Core` member refuses `tainted`**.
Userland was already fail-closed ([`assign.rs`](../../crates/mwl-types/src/expr/assign.rs)); `mwl-stdlib`'s
registry carries no classification field yet, which is why `Core\Str::format`'s template is not the sink
that ADR now makes it.

The other six, one line each: **[0081](../adr/0081-packages-are-digests-resolution-is-a-maximum.md)**
packages are digests, minimal version selection, per-package capabilities.
**[0082](../adr/0082-the-first-party-framework.md)** the framework splits into `Core` plus the `mwl/web`
package; `Web\Migration` is **blocked** on § 7's open gap.
**[0083](../adr/0083-persistent-connections-are-isolates.md)** a WebSocket/SSE connection is a root isolate
named by file. **[0084](../adr/0084-durable-background-jobs.md)** a job is a `Core\Db` row.
**[0085](../adr/0085-openapi-is-generated-from-the-route-table.md)** OpenAPI is emitted while compiling.
**[0086](../adr/0086-core-cli-terminal-is-a-sink.md)** the terminal is a sink; `Cli\Text` is the only raw
path. **[0087](../adr/0087-unbalanced-bidi-is-rejected-at-every-boundary.md)** an unterminated directional
control is a compile error in source and `�` at both sinks.

**Do not start any of the ten while Stage 3 is open.** They are scheduled, not in flight.

Verified this session: `python tools/check-links.py` clean across 102 files, `python tools/check-migration.py`
structurally clean (291 rows, 31% covered, unchanged). No Rust was touched, so `verify.py` was not re-run.

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
  refuses an unclassified member. Lands with M4S's remaining sections.
- **`Core\Time\Date`/`TimeOfDay`/`Month`, `DateTime::date`/`timeOfDay`/`withTime`** — `time.rs`'s gap 1;
  the machinery exists, so each is a registry row and a body. Same for the rest of § 1 and ADR 0069's
  combination members.
- **A shape property read does not lower** — `mwl-ir`'s ADR 0036 § 4 gap: `$e->issues[0]->path` panics.
  Beside it, `landing_block`'s leak of a fresh string argument when a `Core` call throws.
- **[0087](../adr/0087-unbalanced-bidi-is-rejected-at-every-boundary.md)'s lexer check** — still the only
  slice of the ten needing no milestone ahead of it: two counters over spans `mwl-syntax` already walks.
- **The migration-semantics ADR** — [0082](../adr/0082-the-first-party-framework.md) § 7 is the brief, and
  it blocks M16 rather than following it. Unrelated to [0089](../adr/0089-convert-is-one-rule-table-with-two-modes.md),
  which is source conversion; this one is database schema migration.
