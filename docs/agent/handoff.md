# Next session prompt

Where the work stands right now. This file is **state**, overwritten every session and never appended to.
The traps and recipes that outlive a session are in [playbook.md](playbook.md); the rules that bind every
agent are in [AGENTS.md](../../AGENTS.md); `python tools/brief.py` is the rest of the orientation.

## State

**The live path is unchanged: `Core` breadth for Stage 3.** Six of seven fixtures produce their frozen
output; `examples/collect.mwl` is the first that does not. **Eleven ADRs (0080–0090) are written and none
is built.** The last several commits are documentation; this session touched Rust only to correct four doc
comments that ADR 0090 made untrue, and `verify.py` is green.

**[ADR 0080](../adr/0080-the-audience-mwl-is-built-for.md) is still the one to read first, because it
reorders the others.** MWL is built first for **multi-tenant and regulated platforms**, which ranks the
framework and the dependency story **above new `Core` breadth**. Three consequences bind every later
session: the pitch is isolation and qualifiers rather than speed; **no document may claim PHP
compatibility**; and where two slices compete, the one serving that audience wins.

**[ADR 0090](../adr/0090-one-equality-operator-and-disjoint-types-do-not-compile.md) is the newest, and it
is a language-surface change with a real porting cost.** `==` is the **only** equality operator; `===` and
`!==` do not parse. It never converts, two statically **disjoint** operand types are a compile error
(`"1" == 1`, `string` against `bytes`, a non-nullable type against `null`), and the three rows PHP's two
operators disagreed on each take the strict reading — strings compare as text and never as numbers, arrays
compare ordered and element-wise, objects compare by **identity**. A `mixed` operand is the one runtime
case, and a tag mismatch there is `false`, never a throw. Its fold touched twelve ADRs and both spec files
for the spelling alone; § 7 re-tiers [0089](../adr/0089-convert-is-one-rule-table-with-two-modes.md)'s `==`
rule against it.

**[ADR 0089](../adr/0089-convert-is-one-rule-table-with-two-modes.md) decides how a PHP codebase reaches
MWL.** `mwl convert` is **one rule table read through two modes**, not two translators: every branch
carries a tier — **E** proven identical, **D** a mechanical destination that may differ, **N** none — and
the default `--mode=equivalent` emits only E while `--mode=runnable` also emits D under a
`TODO(convert:<id>)`. An E claim is discharged by a differential case against the PHP oracle or CI refuses
it. Its § 7 picks the PHP front end — `mago-syntax` leading, `php-ast` the fallback, behind our own facade,
dialects 7.0–8.5 — **subject to a spike that ADR's *Verification* specifies and that nothing has run yet**.

**[ADR 0088](../adr/0088-a-sink-is-an-instruction-and-the-default-refuses.md) is the other one with a live
code consequence.** It replaces [0024](../adr/0024-taint-tracking-for-injection-sinks.md) § 4's *list* of
sinks with a predicate — *a parameter is a sink when its content becomes an instruction something executes*
— and **flips the default: an unclassified `string`/`bytes` parameter on a `Core` member refuses
`tainted`**. Userland was already fail-closed
([`assign.rs`](../../crates/mwl-types/src/expr/assign.rs)); `mwl-stdlib`'s registry carries no
classification field yet.

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

**Do not start any of the eleven while Stage 3 is open.** They are scheduled, not in flight.

## Next

**`Core\Path` — spec § 11.** Unchanged and still the cheapest slice inside `examples/collect.mwl`: `join`
(variadic, which exists), `basename({withoutExtension})`, `extension(): ?string`, `SEPARATOR`, and no new
dependency. `docs/agent/loop-goal.md` § *Standing decisions* has the two-legs rule for `SEPARATOR`.

## Backlog

- **ADR 0090's four unbuilt halves** — `mwl-ir`'s gap 19 names three (the lexer must stop producing
  `===`/`!==`; `== null` must take `lower_null_identity`'s tag test, currently keyed on `Identical` alone;
  the string/array/object rows need a helper each) and `mwl_types::locals`' `null_test` doc names the
  narrowing arm. The disjoint-operand refusal of its § 2 is `mwl-types`' and needs a new diagnostic code.
- **`Core\Encoding`, `Hash`, `Uuid`, `Csv`, `Validate`, `Random`, `Out`, `Uri::parseQuery`** — the rest of
  `examples/collect.mwl`; each needs a dependency picked under
  [ADR 0051](../adr/0051-standard-library-tiers.md) § 4 and its three obligations.
- **`Core\ObjectSet`/`ObjectMap`** — spec § 8, and the one item needing *language* work first:
  `new Core\X<T>()` does not parse. `mwl_runtime::identity` is the comparison they need, and is also what
  ADR 0090 § 3's object row lowers to.
- **The registry's qualifier classification** — [0088](../adr/0088-a-sink-is-an-instruction-and-the-default-refuses.md)
  § 2: a per-parameter field on `mwl-stdlib`'s member rows, the fail-closed default, and the test that
  refuses an unclassified member. Lands with M4S's remaining sections.
- **`Core\Time\Date`/`TimeOfDay`/`Month`, `DateTime::date`/`timeOfDay`/`withTime`** — `time.rs`'s gap 1;
  the machinery exists, so each is a registry row and a body. Same for the rest of § 1 and ADR 0069's
  combination members.
- **[0087](../adr/0087-unbalanced-bidi-is-rejected-at-every-boundary.md)'s lexer check** — two counters
  over spans `mwl-syntax` already walks, and it needs no milestone ahead of it.
