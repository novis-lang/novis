# Handoff

## State

**Stage 0 item 18 is two slices from done, and the loop still short-circuits at Stage 0.** MWL owns
its allocator in every optimized build: `crates/mwl-runtime/src/alloc.rs` is a per-thread
size-class free list — 16 classes of 16 bytes up to 256, 512 blocks each, everything else
forwarded — registered as the `#[global_allocator]` at `crates/mwl-runtime/src/lib.rs:255`. That
module's own doc owns every decision it took, including the one this session made and nothing else
records: **pooling is compiled in only where `debug_assertions` is off**, so both valgrind legs,
which build `-p mwl-cli` in debug, keep full use-after-free fidelity. Nothing about it is restated
here.

`an_allocation_round_trip_stays_in_the_pooled_cost_class` is green and self-relative, the shape
`loop-goal.toml`'s check for item 18 asks for: **1.14 ns against the platform heap's 27.18 ns on
this machine, a ratio of 0.042× under a 0.5× bound.** Items 19–22 are untouched.

Verify is green (**1566** tests, 74 suites, clippy and fmt clean) — +6 over last session, all of
them `alloc`'s own. Conformance **435**, differential **90**, untouched. The end-to-end check a new
global allocator warrants is `./target/release/mwl.exe test tests/`: **519 passed / 6 failed**,
identical to the debug binary's, so the whole corpus already runs on it. No valgrind run — a debug
build does not take the pooled path, by construction.

**`orient.py` did not print `docs/perf/userland-gap.md` § A**, which is item 18's whole argument and
its numbers; `[context]` in `loop-goal.toml` has no field that selects a perf doc at all, and items
19–22 each cite a section of that file. It is the one selector worth adding.

## Next group — item 18's last two slices

One file set and both are small: `crates/mwl-runtime/src/counting_alloc.rs` and
`crates/mwl-runtime/src/lib.rs`. `loop-goal.md` § *Stage 0* item 18 argues them.

- [ ] **`counting_alloc::Counting` wraps `alloc::Pooled`, not `System`** — the four call sites are
      `crates/mwl-runtime/src/counting_alloc.rs:75`, `:91`, `:102` and `:116`. `Counting` is
      registered under `cfg(test)` and `alloc` is compiled under
      `cfg(any(test, not(debug_assertions)))`, so the module is there to name from a test build.
      The counters keep their meaning either way — `Counting` sits *outside* the cache, so a
      recycled block is still one `alloc` and one `dealloc` — but `live_bytes` then measures the
      shape the release build has rather than the platform heap's.
- [ ] **State what the allocator spends in `mwl-runtime`'s own module doc**
      (`crates/mwl-runtime/src/lib.rs:1`), per ADR 0004 § *Say what you spend*: a bounded
      per-thread cache, never per request and never growing with requests served, with the
      `[limits.hard]` ceiling attaching at M6. One paragraph in the crate's `//!` header that
      links `alloc`'s module doc and `docs/plan/design.md` § *Per-request isolation* rather than
      restating either.

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
      Do this **last of all**, after the rest of item 18 as well as the three above: the file is
      append-only, so an entry written before either records a `php_ratio` that is permanently
      about the wrong build.

## Backlog

- `Core\Out::capture` is the one remaining key in `crates/mwl-stdlib/tests/spec-members-outstanding.txt`, and its `Sink` return type waits on ADR 0088's sink work — `docs/spec/01-core-library.md:888`.
- `examples/collect.mwl:47` is Stage 3's last unfrozen fixture and stops on exactly that member — plan `Blocking`.
- `Core\Json::decodeAs<T>` — spec § 6's one gap, `mwl_stdlib::json` gap 2.
- ADR 0088's qualifier classification on every `mwl-stdlib` member row — plan `Open now`.
- `do`/`while` is the one M4 control-flow statement that does not lower — `mwl-ir`'s module doc.
- The bench review's other half, after Stage 3 rather than in front of it: a cached hash on
  `StrHeader`, a virtual call resolved to a slot instead of a name search, and `Core\Arr::sort`
  without its permutation indirection — `docs/perf/userland-gap.md` §§ F, G, J, which also says
  which case each moves and why none is urgent.
