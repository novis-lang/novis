# Why a userland case is slower than PHP, and what closes it

[`benches/userland/README.md`](../../benches/userland/README.md) owns what a case is.
[`tools/bench.py`](../../tools/bench.py) owns how it is measured.
[ADR 0026](../adr/0026-performance-measurement-methodology.md) owns why the ratio is a secondary
figure and the instruction count is the headline. **This file owns the one thing none of those
does: why a given number is what it is, and which piece of work moves it.**

> **In short:** the compiler is not the problem. A static call is 2.7 ns, a `foreach` step 3.4 ns
> and an object property beats PHP by 3.4× — all faster than the engine MWL is measured against.
> Every case that loses, loses on the **heap**: MWL allocates far more often than PHP. Items A, B
> and C have landed since that was first written — MWL owns its allocator, a string has capacity
> and n-ary concatenation, and neither an integer subscript nor a string literal allocates at all —
> and the median went 0.31× to 0.66× for it. What is left is still allocation *count*, not the
> compiler, and none of the work below is a JIT optimisation.

## This file has a lifetime

It is a **ledger, overwritten in place** — never appended to, never a changelog. Each row below is
work that is not done. When an item lands, its number moves into the module doc that owns the
thing it changed, beside the guard test that holds it, and **its row is deleted from here**
([doc-style.md](../agent/doc-style.md)'s *never quote a measured number outside the file that owns
it* — the numbers are here only because nothing owns them yet). When every row is gone, so is this
file.

## Where the suite stands

Measured 2026-08-26 on the Windows development machine, release against PHP 8.5.9 with opcache and
the tracing JIT, 9 reps, `work` figures (`00-baseline` subtracted). `php/mwl` above 1.00 means MWL
is faster. Every row is one full sweep of the *current* build, not a mix of readings.

| | today | before item A | closed by |
|---|---|---|---|
| median ratio | **0.66×** | 0.31× | A, B, C |
| 04-string-format | 0.20× | 0.07× | E, I |
| 14-word-count | 0.20× | 0.13× | F |
| 12-array-map-filter | 0.36× | 0.18× | D |
| 07-string-normalize | 0.41× | 0.17× | E |
| 09-array-assoc-lookup | 0.42× | 0.25× | F |
| 17-json-encode | 0.44× | 0.21× | E |
| 13-array-contains | 0.50× | 0.12× | — |
| 15-regex-match | 0.51× | 0.25× | — |
| 16-regex-replace | 0.56× | 0.44× | E |
| 08-array-list-build | 0.64× | 0.59× | — |
| 20-method-dispatch | 0.68× | 0.76× | **G** |
| 10-array-sort | 0.69× | 0.68× | J |
| 05-string-replace | 0.71× | 0.32× | E |
| 06-string-split-join | 0.82× | 0.31× | E |
| 18-json-decode | 1.06× | 0.53× | — |
| 03-string-concat | 1.24× † | 0.03× | — |
| 01-arith-loop | 1.48× | 1.33× | — |
| 02-fib-recursive | 2.57× | 2.25× | — |
| 19-object-property | 3.39× | 1.05× | — |
| 11-array-sort-by-field | 3.98× | 3.22× | — |

† `03-string-concat` sits at the suite's noise floor: 3 ms of work behind a 7 ms process start, so
its *ratio* swings between 1.2× and 1.5× from run to run — mostly on PHP's number — while its own
work figure holds at 3.0 ms. § B is where its history is, and it is the one row to read as a work
figure rather than as a ratio.

The second column is the original sweep, taken before item A. The middle column this table used to
carry — the same suite against a 90-line thread-local size-class free list built to price A and
then removed — is gone now that today's column measures the real thing: that prototype predicted a
0.54× median, and what landed measures 0.66×, the difference being B and C landing with it.

**Two cases already win by a wide margin and it is worth knowing why, because neither is a runtime
win.** `19-object-property` wins because a property is a fixed offset into an `MwlObj` where PHP's
is a hash lookup. `11-array-sort-by-field` wins because `Core\Arr::sort`'s `by:` option makes the
sort a Schwartzian transform — 50 000 callback calls, where PHP's `usort` makes one per
*comparison*, about 780 000. That is [ADR 0063](../adr/0063-core-api-conventions.md)'s API shape
paying off, not the engine. The same reading from the other side is the whole of this file: where
the two languages perform the same operations, MWL's cost per operation is higher.

## What one operation costs

MWL figures only. Each is `(case − control) / iterations` over a 2 M-iteration loop, so the loop
and the process start are already subtracted.

| operation | MWL | after A |
|---|---|---|
| loop iteration (two adds and a compare) | 1.5 ns | — |
| static method call | **2.7 ns** | — |
| `foreach` step | **3.4 ns** | — |
| array append `$a[] = $i` (packed) | 15.7 ns | 15.3 ns |
| `Core\Str::length` on a 10-byte string | 14.1 ns | 15.6 ns |
| `$a . $b` | 36.2 ns | **14.8 ns** |
| `$a["beta"]` — a constant string key | 54.3 ns | **21.3 ns** ‡ |
| `$a[$i]` — an integer subscript | 82.4 ns | **27.7 ns** |
| `"x" . $i` | 106.2 ns | **33.2 ns** |

The control is exact: the two operations that allocate nothing did not move, and every one that
allocates fell by half or better.

‡ The constant-key row moved once more when § B's last change landed and a literal stopped
allocating: 26.6 ns to **21.3 ns**, paired against the commit before it, so a literal's own cost in
this shape was **5.3 ns**. The `after A` heading is kept for the other rows rather than a third
column being added for one of them.

**Item 19 has landed, and the `$a[$i]` row is measured rather than projected now.** The A/B is
inside one release binary — the same 20 M subscripts over the same packed array, indexed once by an
`int`, which no longer renders, and once by a `uint`, which still does for the reason
`mwl_ir::lower::Lowering::lower_array_key` states — and the loop is otherwise identical, since
`$i as uint` is a free reinterpret. The two measure **6.3 ns and 28.5 ns** per subscript, each
including the loop's own add and compare. So the rendered path lands exactly on the 27.7 ns above,
and not rendering takes **22.2 ns off every integer subscript**. The suite rows do not move for it:
`08`, `09`, `12` and `13` measure 0.69×, 0.40×, 0.34× and 0.47×, which is where the pooled-allocator
column already put them, because those cases reach their elements through `foreach` and a `string`
key rather than through an integer subscript.

**No PHP column, deliberately.** A micro-case that discards its result is one PHP's tracing JIT may
delete outright — `benches/userland/README.md` § *Why the inputs are chained* is that trap, and it
applies to a micro-case as much as to a suite case. The suite table above is where the comparison
is honest; this table is for attribution.

The floor underneath all of it, measured directly on the same machine: **one `alloc`/`dealloc`
round trip of 32 bytes costs 28.7 ns**, and `i.to_string()` costs 38.9 ns. PHP's Zend MM is a bin
allocator over pre-mapped chunks and charges a small fraction of that.

## Two things ruled out, so they are not re-investigated

- **`catch_unwind` per helper is free.** Measured 0.89 ns inside against 0.91 ns outside on this
  MSVC target. [ADR 0002](../adr/0002-error-propagation.md)'s containment costs nothing on the
  path that does not throw.
- **The uniform calling convention is not the problem.** A static call at 2.7 ns and a `foreach`
  step at 3.4 ns are both faster than the interpreter's. The 16-byte `Value` argument slots are
  not where the time goes.

## The work

Ordered by measured payoff per unit of effort. Each names the file that will own its decision once
it lands, and the guard that will hold it.

### A — MWL owns its allocator

Every allocation goes to the platform heap: `crates/mwl-runtime/src/lib.rs` registers a
`#[global_allocator]` only under `cfg(test)`. [docs/plan/design.md](../plan/design.md)
§ *Per-request isolation* already decided that a request gets its own arena released wholesale at
request end, and [ADR 0004](../adr/0004-memory-for-simplicity.md)'s *Footprint, not traffic*
already makes an allocation on a hot path a priority-3 question. **This is not a new decision — it
is that decision, landing early, in two halves.** The half measured above is a thread-local
size-class cache in front of `System`, which needs no per-request accounting and no `Ctx`. The
per-request ceiling attaches to it at M6, where `mwl_runtime::affordable`'s own doc comment already
says it does.

What it spends, per [ADR 0004](../adr/0004-memory-for-simplicity.md)'s *Say what you spend*: a
bounded per-thread cache of freed blocks — the probe held at most 512 blocks in each of 16 size
classes up to 256 bytes, so ~2 MB per thread, never per request and never growing with requests
served.

One thing to get right: `counting_alloc::Counting` must wrap the new allocator rather than
`System`, or the leak guard measures a path the release build does not take.

*Owner:* `crates/mwl-runtime`'s module doc. *Guard:* an allocation round trip stays in a named cost
class, in `benches/abi-probe/tests/perf_guards.rs`.

### B — a string has capacity, and `.=` appends into it

`StrHeader` carried a refcount and a length and nothing else, and `mwl_str_concat` always builds a
fresh allocation, so `$out .= $piece` copied the whole accumulated string every iteration.
Measured then: 50 000 appends took 238 ms and 100 000 took 1 386 ms — 5.8× for twice the work, the
super-linear shape being the tell. This was the whole of `03-string-concat`, and no allocator fixed
it.

Three changes, one layout revision, and **all three have landed**: the same two runs now take
**15.3 ms and 20.6 ms** — 1.65× for twice the work, against 5.8× — and `03-string-concat` is above
1.00× against PHP where it was 0.03×.

- ~~**Capacity in the header**, and an `mwl_str_append` taking the same *consume one reference,
  return one* protocol `mwl_array_set` already uses — so an append at refcount 1 is in place.~~
  **Landed.** `StrHeader`'s third word is a capacity, `mwl_str_append` doubles when it has to and
  writes in place when it does not, and `mwl_ir::ir::InstKind::StrAppend` carries `.=` on a plain
  `string` local to it with no retain and no release. What it spends — 8 bytes per string
  allocation, and up to twice the payload for a string that has been appended to — is stated in
  `crates/mwl-runtime/src/string.rs`'s module doc § *Capacity, and what it spends*, which is where
  that fact lives rather than here.
- ~~**`.` becomes n-ary.** `InstKind::Concat` is strictly binary, so `"a" . $i . "b" . $i . "c"` is
  four allocations of a growing prefix.~~ **Landed.** That instruction carries a `pieces` vector,
  `Lowering::lower_concat` flattens the `.` spine into it and `lower_interpolated_parts` hands its
  pieces over whole, and `mwl_str_concat_n` sums the total length once and copies each piece once.
  Measured back to back on the same machine, this case's own work went from **3.5 ms to 2.8 ms**
  and its ratio from **1.23× to 1.39×** — the work figure being the firmer of the two, since PHP's
  own number moved between the runs. What is left in the row it builds is the three string
  literals, which is the bullet below.
- ~~**A string literal stops allocating.** `mwl_codegen::emit`'s `emit_const_str` calls
  `mwl_str_new` on every *evaluation*, so `$a["beta"]` inside a loop allocates `"beta"` two million
  times.~~ **Landed.** A whole `StrHeader` goes into the unit's data section in front of the bytes
  and `emit_const_str` materializes its address — no call, no allocation — with the refcount pinned
  at `mwl_runtime::IMMORTAL_REFCOUNT`, which every retain and release compares against and steps
  over. An array literal's keys take the same path. Measured paired against the commit before it:
  `$a["beta"]` costs **26.6 ns → 21.3 ns**, and `04-string-format`, which evaluates three literals
  per iteration 300 000 times, went from **100.0 ms of work to 87.2 ms** (21 reps) — about 14 ns a
  literal where one is an argument that is also released. `05`, `06`, `07`, `15` and `17` each fell
  about 5%; `03-string-concat` did not move at this resolution, its three literals per row being
  small beside the append they feed.

**The non-obvious part was the last one, and it is written down where `MwlStr` is.** An immortal
literal lives in the compiled unit, which is the one thing a request *does* share with another
request — so a literal is reachable from two threads, and `string.rs`'s "no `MwlStr` is ever
reachable from two threads" reasoning behind the plain `Cell` refcount stopped being true as
stated. It stays *sound* because no refcount two threads can reach is ever written, and that
module's docs § *An immortal string, and why the `Cell` survives it* is where the narrowed claim
now lives.

*Owner:* `crates/mwl-runtime/src/string.rs`'s module doc. *Guards:*
`appending_into_spare_capacity_allocates_nothing` holds the append half,
`an_n_ary_concatenation_allocates_one_buffer` the concatenation half and
`an_immortal_string_is_never_written_freed_or_allocated_for` the literal's runtime half — all three
read `counting_alloc::allocated_bytes`, the shape `an_integer_subscript_allocates_no_key` already
uses. The emission half is `mwl-codegen`'s
`a_string_literal_is_one_address_rather_than_an_allocation_per_evaluation`, which compares the
address two evaluations answer with, that crate having no allocation counter to read.

### C — an integer subscript reaches the packed form from compiled code

Already scoped, already the handoff's next group, and already half of loop-goal item 15's measured
claim. `mwl_ir::lower::Lowering::lower_array_key` renders an `int` subscript to a decimal string
through `Helper::IntToString` before `InstKind::ArrayGet`/`ArraySet` reaches codegen, so the
allocation has happened before `mwl_array_get_index` — which exists, and which nothing calls — can
avoid it. `crates/mwl-runtime/src/array.rs`'s module doc § *the ABI was the part that expired* owns
the shape of the change.

*Owner:* already `array.rs`'s module doc. *Guard:* `an_integer_subscript_allocates_no_key`, widened
to reach it through compiled code.

### D — no key is synthesized for a callback that does not want one

`Core\Arr::map`, `filter`, `reduce` and `sort` all call `mwl_array_key_at` per element. On a packed
list that renders a decimal and allocates an `MwlStr` — two allocations — and `call_closure` then
slices the argument list to the closure's declared arity and throws it away. `12-array-map-filter`
burns 400 000 of them per round for nothing. The arity is a field on the closure object and is
readable once before the loop instead of per call.

`Core\Arr::sort` compounds it: it builds a key per element even when `preserveKeys` is `false`, and
the key is then discarded.

*Owner:* `crates/mwl-stdlib/src/arr.rs`'s module doc. *Guard:* a one-parameter callback synthesizes
no key.

### E — a `Core\Str` member writes its result once

Two patterns, both mechanical, both worth fixing before §§ 1–12 grow further, because every new
member copies whichever one is there:

- **`produced(&str)` allocates twice.** A member builds a `String`, then `produced` allocates an
  `MwlStr` and copies it. 56 call sites across `str`, `bytes`, `path`, `regex` and `uri`. Where the
  result length is known — `replace`, `padStart`/`padEnd`, `join` — the member can write straight
  into one `MwlStr`.
- **`text()` re-validates UTF-8 on every string argument.** 56 call sites in `str.rs` alone, each
  an O(n) pass over a string [ADR 0009](../adr/0009-string-and-bytes.md) already guarantees valid —
  the function's own error message says so. `Core\Str::length` then adds two more O(n) passes
  (`is_ascii`, then a scan for `\r`) where `strlen` is O(1); the grapheme unit makes O(n)
  unavoidable, three passes does not.

*Owner:* `crates/mwl-stdlib/src/str.rs`'s module doc. *Guard:* a `Core\Str` member allocates its
result once.

### F — a string carries its hash

`Hashed::index` is a `HashMap<MwlStr, usize>` keyed through `Borrow<[u8]>`, so every lookup
re-hashes the key bytes with SipHash: 13.2 ns for a five-byte key. PHP's `zend_string` carries its
hash, so a repeat lookup with the same string hashes nothing. `14-word-count` does three hashed
lookups per word — `hasKey`, a read, a write — and is the suite's second-worst case after the
allocator lands.

**SipHash itself stays.** `array.rs`'s § *the index map hashes with std's `RandomState`* is
priority 1 over priority 3 and this does not touch it; caching the digest of the bytes changes
nothing about which hasher produced it.

The cost is that `Borrow<[u8]>` has to go — std has no stable raw-entry API, so the map keys on a
type whose `Hash` writes the cached word, and a lookup from bare bytes computes it the same way.
That is contained to `array.rs`, and it is why this is its own item rather than part of B's layout
revision.

*Owner:* `crates/mwl-runtime/src/string.rs` for the field, `array.rs` for the key type.

### G — a virtual call is a slot, not a name search

`mwl_class_method` runs `str::from_utf8` over the method name and then a `binary_search` comparing
strings, on **every** `$obj->method()`. There is no slot index. PHP caches the resolved
`zend_function*` in a run-time cache slot and pays this once.

The method name is statically known at every `InstKind::CallVirtual` site, so the baseline tier can
resolve a **slot index** at compile time and emit a load — which needs one real piece of work, a
vtable layout pass that puts an overridden method at the same index in a subclass's descriptor as
in its base, and a decision about where [ADR 0043](../adr/0043-interface-default-methods-and-delegation-replace-traits.md)'s
default methods sit in it.

**This is not M12's inline cache.** [M12](../plan/m12.md) speculates monomorphic → polymorphic →
megamorphic over a baseline; a slot index is the baseline it speculates *from*, and M12 gets
cheaper for it existing.

Beside it and unrelated to the layout question: `call_closure` and `call_at` each `Vec::with_capacity`
per call, which is a heap allocation on every closure call and every native-to-object dispatch.
`smallvec` is already a workspace dependency.

*Owner:* `crates/mwl-runtime/src/object.rs` for the layout, `dispatch.rs` for the call.
*Guard:* a virtual call contains no name lookup.

### I — `Core\Str::format` allocates once per call, not ten

Per call it builds an `MwlArray` for the variadic tail, walks it back out into a `Vec`, allocates a
`vec![false; n]`, allocates a `String` per conversion and copies a final `String` into an `MwlStr`.
PHP's `sprintf` allocates about one. This member is being reopened anyway for
[ADR 0088](../adr/0088-a-sink-is-an-instruction-and-the-default-refuses.md)'s qualifier
classification — its template is that ADR's sink — so it is cheapest done in the same pass.

### J — `Core\Arr::sort` compares without an indirection

It sorts an index permutation with a `Result`-returning closure over `compare_values`, where PHP
sorts the buckets directly with a specialized comparator. The least bad of the losing cases
(0.66×), and the only one whose fix is a rewrite rather than a removal.

## What is not on this list, and why

- **Statement probes and safepoints are ~80 % of the instructions in the tightest loop** — three
  `ctx` loads and branches around four instructions of work, from `--dump-asm` on a bare `while`.
  It costs 1.5 ns against a native ~0.5 ns, and removing it means trading
  [ADR 0018](../adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)'s guarantee
  that a probe can be switched on for a request already running. That is a decision, not a defect,
  and its natural gate is [ADR 0091](../adr/0091-run-mode-is-two-values-a-ceiling-and-a-list-of-defaults.md)'s
  production mode at M6 — an amendment to 0018 when it gets there, not now.
- **Cranelift emits redundant register moves and does not clean up the block chains.** That is the
  baseline tier's ceiling and [M12](../plan/m12.md) is where it is raised. Nothing above is a
  codegen-quality item; every one of them is a representation or an ABI.
- **`Core\Str::length` is O(n) where `strlen` is O(1).** ADR 0009 makes the grapheme the default
  unit; item E removes two of its three passes and no item removes the third.
