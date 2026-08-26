# Handoff

## State

**Stage 0 item 18 is done, and the loop moves to item 19.** MWL owns its allocator in every
optimized build — `crates/mwl-runtime/src/alloc.rs` is a per-thread size-class free list, registered
at `crates/mwl-runtime/src/lib.rs:274` — and the test build's byte counters now sit in front of *it*
rather than the platform heap: `counting_alloc::Counting` forwards all four methods to
`alloc::Pooled`. The counters sit **outside** the cache, so a recycled block is still one `alloc`
and one `dealloc` and both counts mean what they meant; that is stated in `counting_alloc`'s own
header, not here. `mwl-runtime`'s crate doc now states what the allocator spends (~2 MB per thread
that touches every class, O(threads), never O(requests served)) per ADR 0004 § *Say what you spend*.

**Every number measured from here is measured against the baseline that ships**, which is why item
18 came first — `docs/perf/history.ndjson` is append-only and still does not exist.

Verify is green (**1566** tests, 74 suites, clippy and fmt clean) — unchanged count: this group
added no test, because the change is which allocator every existing guard already measures. No
valgrind run: a debug build still takes the platform heap by construction, which is the whole reason
`alloc` is `cfg(any(test, not(debug_assertions)))`.

**`orient.py` still does not print `docs/perf/userland-gap.md`** — items 19–22 each cite a section
of it and `[context]` in `loop-goal.toml` has no field that selects a perf doc at all. It is the one
selector worth adding, and item 19's group below needs § A's four rows.

## Next group — item 19, an integer subscript reaches the packed form

One file set across three crates, all of it small: `crates/mwl-codegen/src/emit.rs`,
`crates/mwl-codegen/src/lib.rs`, `crates/mwl-ir/src/lower/expr.rs`. The runtime half already exists
and is **unreferenced** — nothing in `crates/mwl-codegen/src` names `mwl_array_get_index` at all.
[loop-goal.md](loop-goal.md) item 19 and `crates/mwl-runtime/src/array.rs`'s module doc
§ *the ABI was the part that expired* are the specification.

- [ ] **Stop rendering a decimal for an integer subscript.** `Lowering::lower_array_key` at
      `crates/mwl-ir/src/lower/expr.rs:1876` renders the key to an `MwlStr` before codegen can
      decline to, which is where the 82.4 ns goes. Read it first: if `InstKind::ArrayGet`'s key
      operand cannot carry an unrendered integer, that is an IR shape question — decide it under the
      goal's standing decisions, record it in `mwl-ir`'s module doc, and do not widen it silently.
- [ ] **Emit the integer pair from codegen.** `emit_array_get` at
      `crates/mwl-codegen/src/emit.rs:1837` and `ArraySet`'s call at `:1824`; add the two
      `RuntimeSig` rows beside `emit.rs:2340` and `:2285`, declared at
      `crates/mwl-codegen/src/lib.rs:604` the way the key-taking pair is. The runtime side is
      `crates/mwl-runtime/src/array.rs:1129` (`mwl_array_get_index`), `:1212`
      (`mwl_array_set_index`) and `:252` (`packed_index`).
- [ ] **The first `docs/perf/history.ndjson` entry**, with a `php_ratio`, per
      [ADR 0026](../adr/0026-performance-measurement-methodology.md) — now sound, because item 18
      landed first. The four rows to re-measure are the ones in `array.rs`'s own module doc table
      (`$a[] = $v`, `$a[$i]`, `$a['name']`, `foreach`) against the PHP 8.5.9 oracle on this machine;
      the table's "unpacked" column is the before.

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
