# Handoff

## State

**Stage 0 re-opened with five more items, and they run before Stage 3.** All seventeen original
items of [loop-goal.md](loop-goal.md) § *Stage 0* are done and every test named for them passes,
but **items 18–22 joined from a bench review** and none of their guards exists yet, so `loop.py`
short-circuits at Stage 0 again. [docs/perf/userland-gap.md](../perf/userland-gap.md) holds every
number behind them and the attribution per case; `loop-goal.md` § *Stage 0* argues each one.
Neither is restated here.

**Item 15's last named test landed with the ABI it waited on.**
`mwl_array_get_index(array, i64, out)` and `mwl_array_set_index(array, i64, value) -> *mut
ArrayHeader` now sit beside the key-taking pair, answering from `Shape::Packed` with no decimal
rendered and nothing allocated, and synthesizing a key only where the shape is already `Hashed`.
`crates/mwl-runtime/src/array.rs`'s module doc owns the decision and what it spends; nothing about it
is restated here.

**One half of item 15's measured claim is still unbanked, and it is now Stage 0 item 19.** `$a[] = $v`
and `foreach` reach the packed form from compiled code today; `$a[$i]` does not, because
`mwl_ir::lower::Lowering::lower_array_key` normalizes an `int` subscript to a decimal string through
`Helper::IntToString` before `InstKind::ArrayGet`/`ArraySet` ever reaches codegen. The key's
representation at the emit site is therefore already `Ty::Str` and the allocation has already
happened, so routing it is an `mwl-ir` change, not a codegen-local one. It is scoped to `file:line`
below, and it runs **after** item 18 rather than first — see that section.

Verify is green (**1560** tests, 74 suites, clippy and fmt clean) — +2 over last session, the new
guard and one for the allocator counter it needed. Conformance **435**, differential **90**,
untouched: no `.mwlt` case was added or edited, so `mwl test tests/` is unmoved at 519 passed /
6 failed (the PHP-on-Windows oracle set). No valgrind run: the new primitives are not reachable from
compiled code yet, and their refcount protocol is `mwl_array_set`'s unchanged.

**A second writer held this tree at the same time, and `git log` reads oddly because of it.** The
whole `crates/mwl-runtime/src/array.rs` change above is in **`cd6a37c`**, that writer's commit, which
swept it in flight and says so; `3a96655` carries only the three files around it, so its message
describes more than it contains. `docs/adr/0051-standard-library-tiers.md` and
`docs/adr/ground-rules.md` are theirs (`bce6f6f`), and `docs/implementation-plan.md`'s M10 paragraph
rode along in `fda43eb`. Nothing is lost and the tree is internally consistent; history was not
rewritten, because the other writer may already be building on it.

## Next group — item 18, MWL owns its allocator

One file set, and it is small: a new `crates/mwl-runtime/src/alloc.rs`, its registration and module-doc
paragraph in `crates/mwl-runtime/src/lib.rs:242` (where `counting_alloc` is registered today under
`cfg(test)`), and a guard in `benches/abi-probe/tests/perf_guards.rs`. `loop-goal.md` § *Stage 0*
item 18 argues it and `docs/perf/userland-gap.md` § A holds the numbers; neither is restated here.

- [ ] **A thread-local size-class free list in front of `System`**, registered as the
      `#[global_allocator]` for non-test builds. Pure Rust, no dependency: the probe that measured
      0.31× → 0.54× was 16 classes of 16 bytes up to 256, a `Cell`-based intrusive free list per
      class capped at 512 blocks, and everything else forwarded. A `thread_local!` here **must** be
      `const`-initialized and hold no `Drop` type, or the allocator allocates from inside itself —
      `counting_alloc.rs`'s own header says why, and it is the one trap in this slice.
- [ ] **`counting_alloc::Counting` wraps the new allocator, not `System`.** Otherwise the leak guard
      and `allocated_bytes` measure a path the release build never takes, which silently weakens
      every allocation-counting guard in the tree — including item 15's.
- [ ] **State what it spends** in `mwl-runtime`'s module doc, per ADR 0004 § *Say what you spend*:
      a bounded per-thread cache, never per request and never growing with requests served. Say that
      the `[limits.hard]` ceiling attaches here at M6, which `mwl_runtime::affordable`'s own doc
      comment already promises, and link `docs/plan/design.md` § *Per-request isolation* rather than
      restating that this is its early half.
- [ ] **The guard**, `an_allocation_round_trip_stays_in_the_pooled_cost_class` — a cost class, the
      shape every other guard in `perf_guards.rs` uses, not a wall-clock number.

## The group after — item 19, routing `$a[$i]` through the new pair

Kept scoped rather than re-derived: these anchors cost a session to find. Slices 1 to 3 are one file
set: `crates/mwl-ir/src/lower/expr.rs`, `crates/mwl-ir/src/lower/stmt.rs`,
`crates/mwl-ir/src/ir.rs`, `crates/mwl-codegen/src/emit.rs`, `crates/mwl-codegen/src/lib.rs`. Slice 4
shares none of it — take it only on its own. ADR 0007 § 5 is the semantics they must not move; the
shape of the change is stated in `crates/mwl-runtime/src/array.rs`'s module doc § *the ABI was the
part that expired*.

- [ ] **Let `InstKind::ArrayGet`/`ArraySet` carry a `Ty::Int` key.** No new variant: the key value's
      own representation is the discriminant, which is why `mwl_array_get_index` takes an `i64`
      rather than a second instruction. Anchors: `crates/mwl-ir/src/ir.rs:913` (`ArrayGet`), `:953`
      (`ArraySet`) — both doc comments state the "every key is normalized to `Ty::Str`" rule that is
      being widened. **`Ty::Uint` stays on the string path**: a `uint` above `i64::MAX` renders a
      decimal `integer_key` refuses, so it is an ordinary string key today and wrapping it to a
      negative index would move semantics.
- [ ] **Stop rendering an `int` subscript in `mwl-ir`.** `lower_array_key` is
      `crates/mwl-ir/src/lower/expr.rs:1876`; its four callers are `:3273` (an array literal's
      explicit `key =>`, which reaches `ArraySet`), `:3339` (the `Index` read),
      `crates/mwl-ir/src/lower/stmt.rs:563` and `:624`. Check which of those four is `unset`'s —
      `mwl_array_unset` has **no** index-taking form, so that one must keep converting. An
      unconverted key is not refcounted, so the "release the fresh key right after the read" the
      `Index` arm does must go with it, and `Self::aliasing_read`'s second return value is
      meaningless for one.
- [ ] **Dispatch on the key's representation in codegen.** `emit_array_get` is
      `crates/mwl-codegen/src/emit.rs:1837` and `emit_array_write` `:1861` — both already take the
      `Ty` back from `self.value(key)` and discard it. New `RuntimeSig::ArrayGetIndex`/`ArraySetIndex`
      at `:2345`, their `runtime_ref` arms at `:2290`, and the two `Signature`s beside
      `crates/mwl-codegen/src/lib.rs:608`, built next to `array_value_at`'s at `:969` (`ptr, I64,
      ptr` and `ptr, I64, ptr -> ptr`). Symbols are already in the table
      (`crates/mwl-runtime/src/helpers.rs:1198`). Snapshots move: `cargo insta test --accept -p
      mwl-ir`, and read the diff rather than accepting it blind — the `int_to_string` call and its
      release vanishing is the whole proof.
- [ ] **The first `docs/perf/history.ndjson` entry**, with a `php_ratio`, per
      [ADR 0026](../adr/0026-performance-measurement-methodology.md) §§ at `:100` (schema and where
      it is appended) and `:190` (what M3 owes it). Only `docs/perf/userland.ndjson` exists today.
      Do this **last of all**, after item 18 as well as the three above: the file is append-only, so
      an entry written before either records a `php_ratio` that is permanently about the wrong build.

## Backlog

- `Core\Out::capture` is the one remaining key in `crates/mwl-stdlib/tests/spec-members-outstanding.txt`, and its `Sink` return type waits on ADR 0088's sink work — `docs/spec/01-core-library.md:888`.
- `examples/collect.mwl:47` is Stage 3's last unfrozen fixture and stops on exactly that member — plan `Blocking`.
- `Core\Json::decodeAs<T>` — spec § 6's one gap, `mwl_stdlib::json` gap 2.
- ADR 0088's qualifier classification on every `mwl-stdlib` member row — plan `Open now`.
- `do`/`while` is the one M4 control-flow statement that does not lower — `mwl-ir`'s module doc.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
- The bench review's other half, after Stage 3 rather than in front of it: a cached hash on
  `StrHeader`, a virtual call resolved to a slot instead of a name search, and `Core\Arr::sort`
  without its permutation indirection — `docs/perf/userland-gap.md` §§ F, G, J, which also says
  which case each moves and why none is urgent.
