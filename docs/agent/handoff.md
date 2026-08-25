# Handoff

## State

**Stage 0 is re-opened, and it outranks everything below.** [loop-goal.md](loop-goal.md) § *Stage 0* now
holds items 10 to 17; **16 and 17 are done**, so what is open is **items 10 to 15**, with fourteen named
tests at `stage = "0 catch-up"` in [loop-goal.toml](loop-goal.toml). `loop.py` short-circuits at stage 0,
so **Stage 3 is shut until those six clear** and the group named further down this file waits behind
them. Take item 10 first; they are ordered cheapest first on purpose. Each item points at the ADR section
or module doc that owns its rule rather than restating it, and `[context]` deliberately adds no selector
for any of them — a session takes one item, the six share no files, and the union cost +10,869 tokens of
orientation for five sections nobody reads.

**Items 16 and 17 landed this session, from a second pass over the same PHP-history ground that produced
10–15.** Both were small enough that queueing them would have cost more than doing them, and both are
listed in `loop-goal.md` as done so neither is re-opened:

* **16 — Cranelift's stack probes are on.** `enable_probestack` defaults to *false*, so a frame over the
  4 KiB guard page could step past it: a stack clash rather than a clean crash. Measured at no
  instruction-count change either way (92,237,951 off vs 92,237,800 inline, call-heavy; 55,399,358 vs
  55,399,652 at 200 frames deep), because a probe is emitted only above 4 KiB and no MWL frame is that
  big yet. `Jit::new`'s comment owns the reasoning. **It is not item 14 under another name** — that
  counts depth, this catches one oversized frame, neither covers the other. Whoever takes 14 should read
  that comment before assuming the stack story is finished.
* **17 — one allocation guard.** `mwl_runtime::affordable` is the single seam a count-shaped argument
  passes through, and its own doc comment owns why, including that it is **not** a budget: ADR 0004's
  `[limits.hard]` per-request ceiling attaches there when M6's arena carries it. It replaced four
  hand-written copies and reached the three members that had none — `Core\Arr::fill`/`padStart`/`padEnd`
  through `append_copies`, `Core\Str::padStart`/`padEnd` through `padding_run`. `Arr::fill($n, 0)` was an
  unbounded run for any `uint`; it is now a catchable throw.

**A field slot is bounds-checked in debug builds.** `field_ptr` had nothing between a wrong index and a
read outside the allocation — and, on the write half, a `release()` on whatever it landed on. The bound is
debug-only deliberately: `Classes::define` builds the codegen slot map and the runtime descriptor from one
`ir::Class::fields` list, so they cannot disagree about a *count*, and carrying the check into release
measured at ~1.4% of a field-heavy program. The commit body has the numbers. The whole suite passes with
it on, which is the reassuring result.

Verify is green (1542 tests, 72 suites, clippy and fmt clean). Conformance **433**, differential 89.

`examples/collect.mwl` still exits 1 at `Core\Out::capture`, and that member is genuinely blocked behind
ADR 0088's sink carriers (`Core\Html\Markup`/`Cli\Text`), which spec § 12's own prose makes its return
type. It is M4S work, not a slice to open ahead of the sinks.

## Numbers worth carrying, measured this session

Callgrind under WSL, per [ADR 0026](../adr/0026-performance-measurement-methodology.md) — wall-clock on
this box swung 56% between two runs of the *same* binary, which is what that ADR exists for.

* **Item 15's `php_ratio` gate has its "before" already**: `$a[] = $i` is **2,014.5 Ir/element** against
  PHP 8.5.9's **148.0** on the same machine, at ~110 vs ~16 bytes per element. That is an independent
  instrument agreeing with the nanosecond figures in `array.rs`'s module doc (219.5 ns vs 23.4 ns), and
  `docs/perf/history.ndjson` still does not exist — which that item already names as why nothing caught
  this.
* **For `[limits.hard]`'s eventual default**: `Core\Arr::fill` is 110 B and ~1.2 µs *per entry*;
  `Core\Bytes::fill(1e9)` is 2 GB in 605 ms; `Core\Str::padStart` is ~3 B and ~13 ns per element. None of
  these are in the docs yet and they are the inputs to picking that number.
* **`Core\Bytes::fill` double-buffers** — `produced(&vec![octet; length])` builds a `Vec` and copies it
  into an `MwlStr`, which is the 2 bytes/element peak for a 1-byte result. Halving it is a small refactor
  for whoever next touches that file; not worth a slice of its own.

## Two open exposures, both under a decided mechanism, neither yet built

Named here because they are *not* bugs to fix ad hoc — the decision that covers each already exists:

* **A helper is an unpollable region.** Safepoints are polled by compiled code, so a single long
  `Core` member is deaf to `CPU_LIMIT` and `CANCEL`: ADR 0020 lists CPU time as a limit that a runaway
  cannot currently reach inside a helper loop. A chunked poll measured at **+0.01%** on `Arr::fill`. It
  wants an ADR 0020 line before an implementation, not a quiet patch.
* **No per-request memory budget.** `affordable` is now the seam; the ceiling itself is M6's.

## Next group — Stage 0 items 10 to 15, in order, starting at item 10

[loop-goal.md](loop-goal.md) § *Stage 0* holds each item and the ADR section or module doc that owns its
rule; [loop-goal.toml](loop-goal.toml)'s `stage = "0 catch-up"` blocks hold the tests that close them. One
item is a group. Item 15 is the largest by far — a second array representation with a degrade path,
roughly 300 to 500 lines across `array.rs` and the ABI — and item 10 is close to done already, so do not
size the set from its first member.

## After Stage 0 — M4's three remaining operator/control-flow holes (`mwl-ir` gap 16, `lib.rs:266`)

These are named in the goal's standing decisions as in scope precisely because the corpus cannot be
written around them, and Stage 4's counts (433 of 600) are now the gate's own work. **They resume once
Stage 0 is empty**, not before — `loop.py` will not reach a Stage 3 fixture until then.

**Shared file set:** `crates/mwl-ir/src/ir.rs:1380` (`BinOp`),
`crates/mwl-ir/src/lower/expr.rs:161` (the binary-operator match),
`crates/mwl-codegen/src/emit.rs:874` (`emit_binop`'s representation rows),
`crates/mwl-ir/src/lower/stmt.rs:166` (`StmtKind::While`'s arm) and
`crates/mwl-ir/src/lower/control.rs:102` (`lower_while`).

- [ ] **1. The bitwise and `**` binary operators lower.** `ir::BinOp` stops at the arithmetic,
      equality and ordering rows, so `&`, `|`, `^`, `<<`, `>>` and `**` panic in `lower_expr`. Each
      needs a `BinOp` variant and an `emit_binop` row over `Int | Uint`. This also lands `&=`, `|=`,
      `^=`, `<<=`, `>>=` and `**=` for nothing: `lower_compound_assignment` already rewrites
      `$x op= e` into `$x = $x op e`, so a compound form arrives the moment its binary form does.
- [ ] **2. `$x++`, `$x--`, `++$x` and `--$x` lower.** Same rewrite shape as a compound assignment
      and the same re-evaluation rule — `is_reevaluable_target` is what refuses `f()->count += 1`,
      and an increment inherits it. Watch the postfix/prefix *value* difference, which the compound
      form has no analogue for.
- [ ] **3. `do { … } while (…);` lowers.** The one M4 control-flow statement that does not; every
      terminator it needs exists, and it is `lower_while` with the body block entered before the
      test rather than after.

## Backlog

- `Core\Out::capture` — the last key in `spec-members-outstanding.txt`; blocked on ADR 0088's sinks.
- ADR 0092 (one diagnostic record, three renderings) and 0091 (run modes) — plan § *Open now*.
- A promoted constructor parameter still claims no slot (`mwl_types::layout`'s own module doc § 38),
  so `$obj->x` on one is `E0405`. PHP writes them everywhere; the differential corpus will meet it.
- `Core\Json::decodeAs<T>`'s wider codec-reachable field set — `mwl_stdlib::json`'s gaps.
- Nullsafe assignment target (`$a?->b = v`) panics rather than being diagnosed — `mwl-ir` gap 6.
- `$e->message()` on a caught `Throwable` panics in `mwl-ir`'s `lower_expr` — "an instance method call
  has no resolved target recorded in the typed-expression table". Found while checking item 17's throw by
  hand; unrelated to it, and it makes a `catch` body hard to write in a scratch fixture.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.

## Orientation gaps found this session

**`--where` has no entry for a backend or runtime *policy* topic.** `python tools/brief.py --where
probestack`, `--where stack` and `--where "memory limit"` all return nothing, so three decisions with real
safety weight — stack probes, ADR 0020's stack limit, ADR 0004's `[limits.hard]` — are reachable only by
already knowing which file to open. That is how this session first proposed a CI cron ADR 0068 § 9
explicitly refuses, and a per-call allocation ceiling where ADR 0004 had already settled a per-request
one. Both were caught by reading the ADR afterwards; neither would have been proposed had `--where`
answered. Worth a routing row each in [docs/adr/README.md](../adr/README.md) § *Where to look*.

**`array.rs`'s packed-array decision sits at lines 36–82 of a 130-line module doc.** A session that opens
that file at the struct — which is where every task in it starts — does not see it and can spend real time
re-deriving a settled decision. The `[context]` manifest selecting the file is not the same as selecting
the section.

`[context]` in `loop-goal.toml` still does not select **ADR 0036 § 4** and names no `mwl-ir` module
pattern at all, so neither `mwl-ir/src/lib.rs`'s gap list nor `lower/*` appears in the map. Both are worth
adding before the next group, which lives entirely in `mwl-ir` and `mwl-codegen`.
