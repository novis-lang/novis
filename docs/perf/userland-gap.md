# Why a userland case is slower than PHP, and what closes it

[`benches/userland/README.md`](../../benches/userland/README.md) owns what a case is.
[`bun nv bench`](../../tools/nv/cmd/bench.ts) owns how it is measured.
`rule:testing/perf-two-mechanisms` owns why the ratio is a secondary
figure and the instruction count is the headline. **This file owns the one thing none of those
does: why a given number is what it is, and which piece of work moves it.**

> **In short:** the compiler is not the problem. A static call is 2.7 ns, a `foreach` step 3.4 ns
> and an object property beats PHP by 3.4× — all faster than the engine Novis is measured against.
> Every case that loses, loses on the **heap**: Novis allocates far more often than PHP. Items A, B
> and C have landed since that was first written — Novis owns its allocator, a string has capacity
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

Measured on the Windows development machine, release against PHP 8.5.9 with opcache and
the tracing JIT, 9 reps, `work` figures (`00-baseline` subtracted). `php/nvs` above 1.00 means Novis
is faster. Every row is one full sweep of the *current* build, not a mix of readings.

| | today | before item A | closed by |
|---|---|---|---|
| median ratio | **0.80×** | 0.31× | A, B, C, D, E |
| 04-string-format | 0.20× | 0.07× | I |
| 14-word-count | 0.20× | 0.13× | F |
| 09-array-assoc-lookup | 0.41× | 0.25× | F |
| 17-json-encode | 0.44× | 0.21× | — |
| 13-array-contains | 0.50× | 0.12× | — |
| 15-regex-match | 0.51× | 0.25× | — |
| 12-array-map-filter | 0.52× | 0.18× | — |
| 16-regex-replace | 0.56× | 0.44× | — |
| 08-array-list-build | 0.66× | 0.59× | — |
| 20-method-dispatch | 0.69× | 0.76× | **G** |
| 06-string-split-join | 0.91× ◇ | 0.31× | — |
| 07-string-normalize | 0.91× | 0.17× | E |
| 10-array-sort | 0.94× | 0.68× | J |
| 05-string-replace | 1.00× ◇ | 0.32× | E |
| 18-json-decode | 1.09× | 0.53× | — |
| 01-arith-loop | 1.50× | 1.33× | — |
| 03-string-concat | 1.51× † | 0.03× | — |
| 02-fib-recursive | 2.53× | 2.25× | — |
| 19-object-property | 3.38× | 1.05× | — |
| 11-array-sort-by-field | 5.17× ‡ | 3.22× | — |

† `03-string-concat` sits at the suite's noise floor: 3 ms of work behind a 7 ms process start, so
its *ratio* swings between 1.2× and 1.5× from run to run — mostly on PHP's number — while its own
work figure holds at 3.0 ms. § B is where its history is, and it is the one row to read as a work
figure rather than as a ratio.

‡ `11-array-sort-by-field` is the other row to read loosely, for the opposite reason: 1.2 s of its
1.5 s is PHP's, so its ratio moves with PHP's variance rather than with Novis's. Two full sweeps on
the same build an hour apart read 5.21× and 4.61×; Novis's own work figure moved 234 ms to 263 ms
across them. Read § D's paragraph for what actually changed there.

◇ `05-string-replace` and `06-string-split-join` are the two rows this sweep read low. Both were
re-run on this build against the commit before it, and the A/B is the number to trust: replace
measures 1.05× and 1.07× in two focused runs of the same binary, and split-join's *code did not
change at all* yet its work figure read 87.3 ms on the base build and 93.3 ms here. A release build
that relinks the whole runtime moves code layout, and these two rows carry about ±6% of it. The
median above is the honest statistic; a single row's third digit is not.

**The suite measures four engines** — Novis, PHP 8.5.9, CPython 3.11.2 and Bun 1.4.0 —
because Novis's CLI claim is made against Python and this project does not publish an unmeasured one, and
because a suite that measured only engines Novis beats would stop being evidence
(`rule:tooling/bench-engine-list-is-data`). On the same
9-rep sweep as the table above, medians of the `work` ratio: **PHP 0.80×, Python 2.13×, Bun 0.57×.**
Cold start — `00-baseline`'s *total*, which every `work` figure subtracts away — is **Novis 7.8 ms, Bun
13.4 ms, Python 19.5 ms, PHP 39.2 ms.**

**Bun is the engine to beat and Novis does not currently beat it.** It wins seventeen of the twenty cases,
and four of them by more than 3×: `12-array-map-filter` **0.08×**, `18-json-decode` 0.22×,
`20-method-dispatch` 0.24×, `15-regex-match` 0.29×. Novis wins `05-string-replace` (1.53×) and draws
`01-arith-loop`, `03-string-concat` and `07-string-normalize`. Read that as the standing verdict on this
suite: a mature JIT behind a scripting surface is ahead of Novis on ordinary userland work today, and the
one figure Novis still owns outright is **cold start**, which it wins against every engine here — Bun
included, and while Bun is also transpiling TypeScript on the way in.

Against Python the split is not uniform and is not noise, and **neither half may be quoted without the
other**: Novis loses `12-array-map-filter` (0.45×), `10-array-sort` (0.71×), `13-array-contains` (0.81×)
and `11-array-sort-by-field` (0.85×) — every one a case whose Python loop is really a call into C — and
wins the genuinely interpreted loops by one to two orders of magnitude (`01-arith-loop` 44×,
`19-object-property` 17×).

**No row in this file's ledger is Python or Bun work.** Items A through J are the PHP gap; nothing below
is waiting on either column, and when the last of them lands and this file is deleted, these paragraphs
go with it. If the Bun gap is ever worked deliberately it earns items of its own, and the first place to
look is the same one every PHP row points at — allocation count, not the compiler.

Three rows moved on item D alone, and they are the last three the array members were holding down:
`12-array-map-filter` 0.36× → **0.51×**, `10-array-sort` 0.69× → **0.96×** and
`11-array-sort-by-field` 3.98× → **5.12×**, which is what carried the median from 0.66×.

The second column is the original sweep, taken before item A. The middle column this table used to
carry — the same suite against a 90-line thread-local size-class free list built to price A and
then removed — is gone now that today's column measures the real thing: that prototype predicted a
0.54× median, and what landed measures 0.66×, the difference being B and C landing with it.

**Two cases already win by a wide margin and it is worth knowing why, because neither is a runtime
win.** `19-object-property` wins because a property is a fixed offset into an `NvsObj` where PHP's
is a hash lookup. `11-array-sort-by-field` wins because `Core\Arr::sort`'s `by:` option makes the
sort a Schwartzian transform — 50 000 callback calls, where PHP's `usort` makes one per
*comparison*, about 780 000. That is `rule:core-api/shape-rules`'s API shape
paying off, not the engine. The same reading from the other side is the whole of this file: where
the two languages perform the same operations, Novis's cost per operation is higher.

## What one operation costs

Novis figures only. Each is `(case − control) / iterations` over a 2 M-iteration loop, so the loop
and the process start are already subtracted.

| operation | Novis | after A |
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

The `Core\Str::length` row predates § E's second bullet and has **not** been re-measured against it:
that member now makes one pass over the bytes where it made three, so the figure is an upper bound
rather than a current reading. It is left as measured rather than guessed at, and § E carries the
suite-level A/B that was actually taken.

‡ The constant-key row moved once more when § B's last change landed and a literal stopped
allocating: 26.6 ns to **21.3 ns**, paired against the commit before it, so a literal's own cost in
this shape was **5.3 ns**. The `after A` heading is kept for the other rows rather than a third
column being added for one of them.

**Item 19 has landed, and the `$a[$i]` row is measured rather than projected now.** The A/B is
inside one release binary — the same 20 M subscripts over the same packed array, indexed once by an
`int`, which no longer renders, and once by a `uint`, which still does for the reason
`nvs_ir::lower::Lowering::lower_array_key` states — and the loop is otherwise identical, since
`$i as uint` is a free reinterpret. The two measure **6.3 ns and 28.5 ns** per subscript, each
including the loop's own add and compare. So the rendered path lands exactly on the 27.7 ns above,
and not rendering takes **22.2 ns off every integer subscript**. The suite rows did not move for it:
`08`, `09`, `12` and `13` measured 0.69×, 0.40×, 0.34× and 0.47× on either side of that change,
which is where the pooled-allocator column already put them, because those cases reach their
elements through `foreach` and a `string` key rather than through an integer subscript. `12` moved
later, on item D, which is the other half of the same observation: what those cases pay for a key
is paid on the way *out* of the array rather than on the way in.

**No PHP column, deliberately.** A micro-case that discards its result is one PHP's tracing JIT may
delete outright — `benches/userland/README.md` § *Why the inputs are chained* is that trap, and it
applies to a micro-case as much as to a suite case. The suite table above is where the comparison
is honest; this table is for attribution.

The floor underneath all of it, measured directly on the same machine: **one `alloc`/`dealloc`
round trip of 32 bytes costs 28.7 ns**, and `i.to_string()` costs 38.9 ns. PHP's Zend MM is a bin
allocator over pre-mapped chunks and charges a small fraction of that.

## Two things ruled out, so they are not re-investigated

- **`catch_unwind` per helper is free.** Measured 0.89 ns inside against 0.91 ns outside on this
  MSVC target. `rule:errors/propagation`'s containment costs nothing on the
  path that does not throw.
- **The uniform calling convention is not the problem.** A static call at 2.7 ns and a `foreach`
  step at 3.4 ns are both faster than the interpreter's. The 16-byte `Value` argument slots are
  not where the time goes.

## The work

Ordered by measured payoff per unit of effort. Each names the file that will own its decision once
it lands, and the guard that will hold it.

### A — Novis owns its allocator

Every allocation goes to the platform heap: `crates/nvs-runtime/src/lib.rs` registers a
`#[global_allocator]` only under `cfg(test)`. [docs/plan/design.md](../plan/design.md)
§ *Per-request isolation* already decided that a request gets its own arena released wholesale at
request end, and `rule:programs/memory-priority`'s *Footprint, not traffic*
already makes an allocation on a hot path a priority-3 question. **This is not a new decision — it
is that decision, landing early, in two halves.** The half measured above is a thread-local
size-class cache in front of `System`, which needs no per-request accounting and no `Ctx`. The
per-request ceiling attaches to it at M6, where `nvs_runtime::affordable`'s own doc comment already
says it does.

What it spends, per `rule:programs/memory-priority`'s *Say what you spend*: a
bounded per-thread cache of freed blocks — the probe held at most 512 blocks in each of 16 size
classes up to 256 bytes, so ~2 MB per thread, never per request and never growing with requests
served.

One thing to get right: `counting_alloc::Counting` must wrap the new allocator rather than
`System`, or the leak guard measures a path the release build does not take.

*Owner:* `crates/nvs-runtime`'s module doc. *Guard:* an allocation round trip stays in a named cost
class, in `benches/abi-probe/tests/perf_guards.rs`.

### B — a string has capacity, and `.=` appends into it

`StrHeader` carried a refcount and a length and nothing else, and `nvs_str_concat` always builds a
fresh allocation, so `$out .= $piece` copied the whole accumulated string every iteration.
Measured then: 50 000 appends took 238 ms and 100 000 took 1 386 ms — 5.8× for twice the work, the
super-linear shape being the tell. This was the whole of `03-string-concat`, and no allocator fixed
it.

Three changes, one layout revision, and **all three have landed**: the same two runs now take
**15.3 ms and 20.6 ms** — 1.65× for twice the work, against 5.8× — and `03-string-concat` is above
1.00× against PHP where it was 0.03×.

- ~~**Capacity in the header**, and an `nvs_str_append` taking the same *consume one reference,
  return one* protocol `nvs_array_set` already uses — so an append at refcount 1 is in place.~~
  **Landed.** `StrHeader`'s third word is a capacity, `nvs_str_append` doubles when it has to and
  writes in place when it does not, and `nvs_ir::ir::InstKind::StrAppend` carries `.=` on a plain
  `string` local to it with no retain and no release. What it spends — 8 bytes per string
  allocation, and up to twice the payload for a string that has been appended to — is stated in
  `crates/nvs-runtime/src/string.rs`'s module doc § *Capacity, and what it spends*, which is where
  that fact lives rather than here.
- ~~**`.` becomes n-ary.** `InstKind::Concat` is strictly binary, so `"a" . $i . "b" . $i . "c"` is
  four allocations of a growing prefix.~~ **Landed.** That instruction carries a `pieces` vector,
  `Lowering::lower_concat` flattens the `.` spine into it and `lower_interpolated_parts` hands its
  pieces over whole, and `nvs_str_concat_n` sums the total length once and copies each piece once.
  Measured back to back on the same machine, this case's own work went from **3.5 ms to 2.8 ms**
  and its ratio from **1.23× to 1.39×** — the work figure being the firmer of the two, since PHP's
  own number moved between the runs. What is left in the row it builds is the three string
  literals, which is the bullet below.
- ~~**A string literal stops allocating.** `nvs_codegen::emit`'s `emit_const_str` calls
  `nvs_str_new` on every *evaluation*, so `$a["beta"]` inside a loop allocates `"beta"` two million
  times.~~ **Landed.** A whole `StrHeader` goes into the unit's data section in front of the bytes
  and `emit_const_str` materializes its address — no call, no allocation — with the refcount pinned
  at `nvs_runtime::IMMORTAL_REFCOUNT`, which every retain and release compares against and steps
  over. An array literal's keys take the same path. Measured paired against the commit before it:
  `$a["beta"]` costs **26.6 ns → 21.3 ns**, and `04-string-format`, which evaluates three literals
  per iteration 300 000 times, went from **100.0 ms of work to 87.2 ms** (21 reps) — about 14 ns a
  literal where one is an argument that is also released. `05`, `06`, `07`, `15` and `17` each fell
  about 5%; `03-string-concat` did not move at this resolution, its three literals per row being
  small beside the append they feed.

**The non-obvious part was the last one, and it is written down where `NvsStr` is.** An immortal
literal lives in the compiled unit, which is the one thing a request *does* share with another
request — so a literal is reachable from two threads, and `string.rs`'s "no `NvsStr` is ever
reachable from two threads" reasoning behind the plain `Cell` refcount stopped being true as
stated. It stays *sound* because no refcount two threads can reach is ever written, and that
module's docs § *An immortal string, and why the `Cell` survives it* is where the narrowed claim
now lives.

*Owner:* `crates/nvs-runtime/src/string.rs`'s module doc. *Guards:*
`appending_into_spare_capacity_allocates_nothing` holds the append half,
`an_n_ary_concatenation_allocates_one_buffer` the concatenation half and
`an_immortal_string_is_never_written_freed_or_allocated_for` the literal's runtime half — all three
read `counting_alloc::allocated_bytes`, the shape `an_integer_subscript_allocates_no_key` already
uses. The emission half is `nvs-codegen`'s
`a_string_literal_is_one_address_rather_than_an_allocation_per_evaluation`, which compares the
address two evaluations answer with, that crate having no allocation counter to read.

### C — an integer subscript reaches the packed form from compiled code

Already scoped. `nvs_ir::lower::Lowering::lower_array_key` renders an `int` subscript to a decimal string
through `Helper::IntToString` before `InstKind::ArrayGet`/`ArraySet` reaches codegen, so the
allocation has happened before `nvs_array_get_index` — which exists, and which nothing calls — can
avoid it. `crates/nvs-runtime/src/array.rs`'s module doc § *the ABI was the part that expired* owns
the shape of the change.

*Owner:* already `array.rs`'s module doc. *Guard:* `an_integer_subscript_allocates_no_key`, widened
to reach it through compiled code.

### D — no key is synthesized for a callback that does not want one

~~`Core\Arr::map`, `filter`, `reduce` and `sort` all call `nvs_array_key_at` per element. On a
packed list that renders a decimal and allocates an `NvsStr` — two allocations — and `call_callable`
then slices the argument list to the closure's declared arity and throws it away.
`12-array-map-filter` burns 400 000 of them per round for nothing. The arity is a field on the
closure object and is readable once before the loop instead of per call.~~ ~~`Core\Arr::sort`
compounds it: it builds a key per element even when `preserveKeys` is `false`, and the key is then
discarded.~~ **Landed, both halves.**

All four members read `nvs_runtime::callable_arity` once before their loop and build the key only
where the callback declared a parameter to receive it. That alone would not have paid for `map` and
`filter`, which *preserve* keys and so were going to build one anyway, so the store half changed
too: `nvs_runtime::SlotKey` answers the key in whichever form the subject's own shape already holds
it — the position itself while the array is packed, a reference to the stored string once it is
hashed — and `NvsArray::set_index` writes it back with nothing rendered. A `filter` allocates a key
only where it left a gap, which is exactly where the result stops being a list. `sort` goes one
further: with `preserveKeys` false and no `by` closure asking for one, nothing downstream can
observe a key, so its walk collects none.

Measured against the sweep in *Where the suite stands*, which was taken on the commit before this
one: `12-array-map-filter` **0.36× → 0.51×**, its own work now 268.5 ms; `10-array-sort`
**0.69× → 0.96×**; `11-array-sort-by-field` **3.98× → 5.12×**. Nothing else in the suite moved, and
the median went 0.66× → **0.69×**. That is the *floor* section above collected: a rendered decimal
is 38.9 ns and its allocation round trip 28.7 ns, and this case walked 4.7 M entries.

*Owner:* `crates/nvs-stdlib/src/arr.rs`'s module doc § *A callback that does not want a key is never
handed one*. *Guard:* `a_callback_that_does_not_want_a_key_synthesizes_none` in
`crates/nvs-runtime/src/array.rs`, which reads `counting_alloc::allocated_bytes` over exactly the
walk those members make — with the control the playbook asks for, since the same test measures that
asking for the key as a string *does* allocate.

### E — a `Core\Str` member writes its result once

Two patterns, both mechanical, both worth fixing before §§ 1–12 grow further, because every new
member copies whichever one is there:

- ~~**`produced(&str)` allocates twice.** A member builds a `String`, then `produced` allocates an
  `NvsStr` and copies it. 56 call sites across `str`, `bytes`, `path`, `regex` and `uri`. Where the
  result length is known — `replace`, `padStart`/`padEnd`, `join` — the member can write straight
  into one `NvsStr`.~~ **Landed, for every member but `join`** — see below.
- ~~**`text()` re-validates UTF-8 on every string argument.** 56 call sites in `str.rs` alone, each
  an O(n) pass over a string `rule:types/bytes` already guarantees valid —
  the function's own error message says so. `Core\Str::length` then adds two more O(n) passes
  (`is_ascii`, then a scan for `\r`) where `strlen` is O(1); the grapheme unit makes O(n)
  unavoidable, three passes does not.~~ **Landed, both halves.**

Reading a `string` argument is now a tag check. The unchecked read lives once, behind one `unsafe`
in `nvs_runtime::NvsStr::text_of`, with `Value::as_text` as the safe caller that discharges it —
the tag *is* `rule:types/bytes`'s guarantee, so deriving it again per argument was work whose answer the
runtime already held. A debug build re-validates inside that one reader, which is what keeps the
invariant checked rather than remembered. The second half is `crate::granularity`'s fast-path test:
the `is_ascii` scan and the search for `\r` are one branchless fold over the bytes, so an ASCII
`Core\Str::length` is one pass and then `len`.

Measured as an A/B on one machine — the same suite, 9 reps, this build against the commit before
it, rather than against the sweep above: `05-string-replace` **0.72× → 0.91×** (work 103.1 ms →
81.5 ms), `06-string-split-join` **0.78× → 0.96×**, `07-string-normalize` **0.42× → 0.51×**,
`04-string-format` work 92.9 ms → 82.0 ms. The base half of that pair reproduced the sweep above to
within 0.02× on every one of those rows, which is what makes it an A/B rather than two readings.
The median did not move: those four rows crossed *over*
the median rather than lifting it, so the statistic sat still while a fifth of the suite's work
disappeared — which is the reading to keep, since nothing outside the string rows moved beyond the
run-to-run noise the two footnotes above describe.

The first bullet then landed as `nvs_runtime::NvsStr::build`: a producer is handed a writer over the
allocation the value will be answered from, so the bytes are written there instead of into a
`String` that is then copied in. A member whose length is exact (`repeat`, `padStart`/`padEnd`,
`reverse`) allocates once; `replace` starts the writer at its subject's length and the writer
`realloc`s on the same doubling `String` used, so a guess that falls short costs what it always cost
and never the final copy. Measured as an A/B against the commit before it: `05-string-replace`
**0.91× → 1.05×** (work 80.1 ms → 70.8 ms) and `07-string-normalize` **0.50× → 0.93×** (81.8 ms →
44.5 ms). The suite median went **0.69× → 0.80×**.

**Two things were measured and rejected, and they are the reason this bullet is worth a paragraph
rather than a line.** Buying an *exact* length with a second read is a loss at these sizes: counting
`replace`'s matches first took that row to 0.74×, and walking a cycle of `padEnd`'s padding to
measure what a second walk then wrote took `07-string-normalize` to 0.45×. Both are arithmetic now.
And **`join` is left as it was** — its length costs a walk of the subject's array slots, which is
the expensive half of the member, and all three ways round that measured worse than the `String` it
builds: a writer at a guessed capacity 90.0 ms against the base build's 87.3, a `Vec` of borrowed
pieces 105.1 ms, a measuring walk 92.5 ms. Nothing about the pattern is wrong there; the length is
just not cheap to learn.

*Owner:* `crates/nvs-stdlib/src/str.rs`'s module doc §§ *`string` is valid UTF-8, so this module
never validates* and *A result is written once*, the second of which holds the rejected
alternatives so they are not tried a third time.
*Guard:* `the_fused_scan_agrees_with_the_two_pass_spelling` in
`crates/nvs-stdlib/src/granularity.rs` holds the fold against the spelling it replaced;
`a_bytes_value_is_a_string_allocation_under_a_tag_of_its_own` in `crates/nvs-runtime/src/value.rs`
holds the one thing soundness rests on, that the unchecked reader answers nothing for a `bytes`; and
`a_str_member_allocates_its_result_once` in
`crates/nvs-stdlib/tests/allocation_policy.rs` counts the allocations four members make and holds
each to one.

### F — a string carries its hash

`Hashed::index` is a `HashMap<NvsStr, usize>` keyed through `Borrow<[u8]>`, so every lookup
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

*Owner:* `crates/nvs-runtime/src/string.rs` for the field, `array.rs` for the key type.

### G — a virtual call is a slot, not a name search

`nvs_class_method` runs `str::from_utf8` over the method name and then a `binary_search` comparing
strings, on **every** `$obj->method()`. There is no slot index. PHP caches the resolved
`zend_function*` in a run-time cache slot and pays this once.

The method name is statically known at every `InstKind::CallVirtual` site, so the baseline tier can
resolve a **slot index** at compile time and emit a load — which needs one real piece of work, a
vtable layout pass that puts an overridden method at the same index in a subclass's descriptor as
in its base, and a decision about where `rule:classes/no-traits`'s
default methods sit in it.

**This is not M12's inline cache.** [M12](../plan/m12.md) speculates monomorphic → polymorphic →
megamorphic over a baseline; a slot index is the baseline it speculates *from*, and M12 gets
cheaper for it existing.

Beside it and unrelated to the layout question: `call_callable` and `call_at` each `Vec::with_capacity`
per call, which is a heap allocation on every closure call and every native-to-object dispatch.
`smallvec` is already a workspace dependency.

*Owner:* `crates/nvs-runtime/src/object.rs` for the layout, `dispatch.rs` for the call.
*Guard:* a virtual call contains no name lookup.

### I — `Core\Str::format` allocates once per call, not ten

Per call it builds an `NvsArray` for the variadic tail, walks it back out into a `Vec`, allocates a
`vec![false; n]`, allocates a `String` per conversion and copies a final `String` into an `NvsStr`.
PHP's `sprintf` allocates about one. This member is being reopened anyway for
`rule:security/sink-predicate`'s qualifier
classification — its template is that ADR's sink — so it is cheapest done in the same pass.

### J — `Core\Arr::sort` compares without an indirection

It sorts an index permutation with a `Result`-returning closure over `compare_values`, where PHP
sorts the buckets directly with a specialized comparator. It was the least bad of the losing cases
and it is barely a losing case now — item D took it from 0.69× to **0.96×** without touching the
comparison at all — but it is still the only one on this list whose fix is a rewrite rather than a
removal.

### K — an empty array is a per-thread singleton, not an allocation

`[]` lowers to `InstKind::ArrayNew { entries: [] }` and calls `nvs_array_new`, which boxes an
`ArrayHeader` — **one** allocation and not three, since the packed shape means the entry vector and
the index map are both absent, but one is not zero. Userland produces empty arrays constantly and
most of them are never written into: an early return, a `filter` that matched nothing, a lookup that
missed, a collector on a branch that was not taken. Each is a free-list pop, a header's worth of
stores and the mirror on release, for a container nothing ever reads.

The mechanism already exists in the tree for the other refcounted container —
`crates/nvs-runtime/src/string.rs` § *An immortal string* — and three properties of the array make it
cheaper here than it was there. Every mutator goes through `NvsArray::make_unique`, which separates
whenever the refcount is not 1, so a singleton a thread-local holds one reference to can never be
written through. `nvs_array_eq` compares by content, so the sharing is unobservable to `rule:expressions/one-equality-operator`'s
identity row. And an array is `!Send + !Sync`, so per-thread is per-owner and the count stays
non-atomic. The consequence worth having: **no hot path needs a pointer comparison.** The singleton's
count simply never reaches zero, so `retain`, `release` and the whole teardown path are unchanged and
`nvs_array_new` is the only function that moves.

Deliberately *not* the immortal-header-in-the-data-section arrangement a string literal gets, and the
reason is the `RefCell`: a read takes `table.borrow()`, which **writes** the borrow flag, so a header
shared between threads would be a race on every `count()`. Thread-local is what makes the borrow flag
sound — a `Cell<*mut ArrayHeader>` behind `alloc.rs`'s own const-init thread-local pattern, not a
`static`.

The aggressive variant — null *is* the empty array, so codegen materializes a constant and there is no
call either — is ruled out here rather than left open. `nvs_array_retain`/`release` already no-op on
null, but every read primitive would gain a null arm and `Core\Arr`'s whole surface would inherit a
"did you handle null" invariant. That is AGENTS.md's priority 4 spent across a large surface to save
one branch.

What it spends, per `rule:programs/memory-priority`'s *Say what you spend*: **nothing
per request — it saves.** One block per thread, permanently, against one block per empty array that
stays empty. An empty array that *is* later written pays one `make_unique` separation, which allocates
exactly the header the current path allocates eagerly, plus a failed `== 1` branch — a wash plus a
branch, not a regression. The one non-obvious cost is the leak check: a per-thread block that is never
freed is *still reachable* rather than *definitely lost*, and `tools/leak-check.sh`'s threshold is
what says whether that matters.

**Priced by count rather than by clock**, which is what makes a guard worth having here at all: the
saving is one pooled allocation and its matching free per empty array that stays empty, and zero
against one is exact where a duration would sit in the noise. What one of them is worth is the floor
section's ceiling — a platform-heap round trip is 28.7 ns and the pooled path a fraction of it — so
this stays a tens-of-nanoseconds item and no suite case is waiting on it. It is ranked last for that
reason, and it is on the list because it is small, self-contained and strictly negative on footprint,
not because anything measured asked for it.

*Owner:* `crates/nvs-runtime/src/array.rs`'s module doc § *an empty array is a per-thread singleton*,
the fourth decision beside the packed one. *Guard:* `an_empty_array_allocates_nothing`, reading
`counting_alloc::allocated_bytes` across a run of `nvs_array_new` calls, with
`writing_into_an_empty_array_allocates` as the control the playbook asks for — a measurement that only
ever reads zero passes just as well when it is broken.

## What is not on this list, and why

- **Statement probes and safepoints are ~80 % of the instructions in the tightest loop** — three
  `ctx` loads and branches around four instructions of work, from `--dump-asm` on a bare `while`.
  It costs 1.5 ns against a native ~0.5 ns, and removing it means trading
  `rule:testing/debug-probes`'s guarantee
  that a probe can be switched on for a request already running. That is a decision, not a defect,
  and its natural gate is `rule:config/two-modes-and-the-default-is-production`'s
  production mode at M6 — an amendment to 0018 when it gets there, not now.
- **Cranelift emits redundant register moves and does not clean up the block chains.** That is the
  baseline tier's ceiling and [M12](../plan/m12.md) is where it is raised. Nothing above is a
  codegen-quality item; every one of them is a representation or an ABI.
- **`Core\Str::length` is O(n) where `strlen` is O(1).** `rule:types/bytes` makes the grapheme the default
  unit; item E removes two of its three passes and no item removes the third.
