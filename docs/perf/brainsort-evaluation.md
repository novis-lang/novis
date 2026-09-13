# brainsort against `Core\Arr::sort`'s merge sort

**Verdict: worth switching, for the natural-order path only, once the crate is published and with its
block cache turned off.** The order it produces is identical to the merge sort's, index for index, on
every row of the natural ordering; the whole verification is green with it wired in; and the two
userland sort cases run 2.2x to 4.8x faster end to end. Nothing here has landed — this file is the
record of the evaluation, and the branch that carried it is deleted. When it is picked up again, the
section *How to rebuild the seam* is enough to redo the wiring in an hour.

[brainsort](https://github.com/brainfoolong/brainsort) is a stable sort for keys that map to an
ordered integer — numbers, strings, dates, ids — radix where the key allows it, with AVX2/BMI2 chosen
at run time; its Rust port is `rust/brainsort` in that repository, and every number below was measured
against its working copy at commit `a9c1c39`, on the developer workstation (AMD Ryzen 7 7800X3D,
Windows 11), Novis at toolchain 1.97.1, release profile.

## What can move and what cannot

`Core\Arr::sort` and `sortByKey` are one hand-written bottom-up merge sort over an index permutation
(`crates/nvs-stdlib/src/arr.rs`, `merge_sort`), and its doc comment says why: the comparison can
**fail** — a comparator callback can throw, and two values of incomparable types are a throw of the
member's own — and Rust's sorts take an infallible comparator.

That constraint decides the split:

- **The natural-order path can move.** Without a comparator, a subject whose values all sit in one row
  of `nvs_stdlib::ordering::compare_values`'s table — all `int`/`uint`, all `float`, all `string`, all
  `bool`, all `null` — can never throw, so it can be handed to brainsort as a *key* sort. One
  classification pass over the tags decides the row; anything else (a `decimal`, an `int` beside a
  `float`, a `bytes`, an array, an object) stays on the merge sort, which throws exactly as it does
  today. This covers `sort($list)`, `sort($list, {by: ...})` (the extracted keys are values too) and
  `sortByKey` without a comparator — the calls a program actually makes.
- **The comparator path cannot, and does not need to.** brainsort's `sort_by` is the standard
  library's stable sort behind fast paths for ordered input, and it has no way to abort. Stashing the
  first fault and answering `Equal` afterwards is exactly the inconsistent order the standard sort is
  documented to be allowed to panic on. A Novis comparator costs a closure call per comparison, so the
  algorithm underneath it is not where the time goes; a fallible-comparator API would be a brainsort
  feature with little to gain here.

## The key mapping, which is what makes the order identical

The elements brainsort sorts are `u32` indices; the key is computed once per index by
`brainsort::sort_by_key`, wrapped in `brainsort::Desc` for `Core\Order::Desc`. brainsort is stable and
`Desc` reverses the comparison rather than the result, which is the same rule the merge sort applies,
so equal keys keep their input order under both orders.

| Row | Key | Why that key orders as `compare_values` does |
|---|---|---|
| all `int` | `i64` | exact |
| all `uint` | `u64` | exact |
| `int` and `uint` mixed | `i128` | `compare_values` widens both to `i128` |
| all `float` | `i64` | the bits `f64::total_cmp` compares, after every `NaN` is replaced by the one quiet negative `NaN` the ordering uses, so `NaN` sorts below every number and `-0.0` before `0.0` — brainsort's own float order (`-0.0 == 0.0`, `NaN` by sign) is deliberately **not** used |
| all `string` | `&[u8]` | bytewise, shorter prefix first — `"10"` before `"9"` |
| all `bool` | `bool` | `false` before `true` |
| all `null` | identity permutation | every pair is equal |

## Correctness evidence

All of it was a differential test against the merge sort, in one module beside the seam, and all of it
passed:

- **Same permutation** on 6 rows × 7 shapes (random, sorted, reversed, nearly sorted, few distinct,
  runs, organ pipe) × 19 sizes from 0 to 20 000 × both orders. Equality of the permutation, not of the
  sorted values, is what proves stability as well as order.
- **Float corners** (both zeros, both `NaN` signs, both infinities, subnormals, `f64::MAX`/`MIN`),
  **integer corners** (`i64::MIN`, `i64::MAX`, `u64::MAX`, the `int`/`uint` boundary) and **string
  corners** (empty, prefixes, a 300-byte shared prefix, high bytes) in 500 random mixes each.
- **Mixed subjects are declined**, and the merge sort throws on every one of 400 random layouts of one
  string among ints, so declining changes no answer.
- **The whole member** answers the same array under both implementations with `preserveKeys` on and
  off and both orders, at 0, 1, 7, 40 and 1 000 entries.
- **`python tools/verify.py`** with brainsort on by default: build, unit tests, 1 874 conformance
  cases, 276 differential cases and clippy, all green.
- **Userland cases `10-array-sort` and `11-array-sort-by-field`** print the same checksum under
  brainsort as PHP 8.5, Python 3.11 and Bun do (`python tools/bench.py --check`).

## Performance

End to end, `python tools/bench.py 10-array 11-array --engines nvs,php --reps 7`, work milliseconds
after the baseline subtraction, one binary switched by environment variable:

| case | merge sort | brainsort, cache on | brainsort, cache off | PHP 8.5 JIT |
|---|---:|---:|---:|---:|
| `10-array-sort` (50 000 ints × 20 rounds) | 154.7 | 32.3 | 37.0 | 145.9 |
| `11-array-sort-by-field` (50 000 rows by `{by:}` × 20) | 212.3 | 83.8 | 95.4 | 1 040 |

Case 10 goes from 0.94x of PHP to 3.9x. The remainder of case 11 is the 50 000 `by` callbacks per
round, which no sort touches.

Inside the member (`nvs_core_arr_sort` over a list, interleaved medians, cache on), the speed-up
merge ÷ brainsort:

| row, shape | n=8 | n=32 | n=100 | n=1 000 | n=10 000 | n=100 000 |
|---|---:|---:|---:|---:|---:|---:|
| int random | 1.33 | 1.63 | 1.21 | 1.93 | 3.39 | 3.71 |
| int sorted | 1.33 | 1.71 | 2.00 | 3.06 | 2.37 | 3.14 |
| int few distinct | 1.33 | 1.63 | 1.15 | 2.74 | 4.82 | 4.77 |
| str random | 1.00 | 0.94 | 1.09 | 2.79 | 4.03 | 3.67 |
| str sorted | 1.00 | 1.50 | 1.86 | 2.73 | 3.63 | 3.33 |
| float random | 1.33 | 1.40 | 1.33 | 2.94 | 5.14 | 5.02 |
| float sorted | 1.20 | 1.83 | 2.40 | 3.32 | 4.42 | 4.43 |

Small arrays — the request-path norm — are level to slightly faster; from a thousand entries the sort
is 2x to 6x faster; nothing is slower beyond noise (the 0.94 is a 1.5 µs call). On the permutation
alone, without the member's own walk and result build around it, the gap is 7x to 27x at 10 000 and
up; the member-level figures are the honest ones, because the walk that collects the values and the
loop that builds the result are the same under both and now dominate.

## Memory and isolation

- **brainsort keeps a process-wide cache of freed blocks** of 64 KiB and more, up to 32 MiB, shared by
  every thread under a mutex, until `release_memory` is called or the process ends (`memory.rs`). In
  Novis that is cross-request memory nothing attributes to a request, which
  `rule:programs/memory-priority` treats as a hole rather than a trade-off, and a lock on the request
  path. It has to be off: `brainsort::set_memory_cache_limit(0)` once at startup. The cost is in the
  table above — 10 to 15 percent at 100 000 elements, nothing below — and is paid. With the limit at
  zero the allocate and deallocate paths still take the (uncontended) mutex once per large block
  before consulting the limit; checking the limit first is a two-line change on brainsort's side and
  optional.
- **What a sort spends**, computed from brainsort's documented record layout and not measured: about
  40 bytes per element transient during the call — the `u32` index array, a 16-byte record per
  element, scratch of about half the records, the element permutation buffer, and the `usize`
  permutation handed back — against the merge sort's 16 bytes per element. Freed on return; nothing
  outlives the call once the cache is off.
- **A panic inside the crate** is contained to one request by the helper boundary's `catch_unwind`,
  like any other runtime panic.
- **Allocation failure** inside brainsort falls back to the standard library's stable sort with the
  same order; it never aborts.

## The dependency

brainsort's Rust workflow runs clippy with warnings denied, the suite on `no_std` and on a 32-bit
target, Miri under strict provenance over the library and three test suites, AddressSanitizer over the
vector build, and a libFuzzer job with a persisted corpus. The crate has no dependencies. It carries on
the order of two hundred `unsafe` sites, including the AVX2 kernels, on a path attacker-controlled data
reaches — so it sits under `rule:packaging/a-c-dependency-answers-two-questions`'s second question by
analogy, and the answer above is the record to write when it is adopted. At the time of the
evaluation it was **not on crates.io**; Novis pins registry versions, so publication precedes the switch.
Everything the seam uses — `sort_by_key`, `Desc`, the `Key` impls for `i64`, `u64`, `i128`, `bool` and
`&[u8]`, `set_memory_cache_limit` — has been public API since the port's first commit, so no change on
brainsort's side is needed first.

## How to rebuild the seam

The whole integration was six files and one new module, and the module is what matters:

1. A `sort_impl` module in `nvs-stdlib` with `natural_permutation(values: &[Value], descending) ->
   Option<Vec<usize>>` (classify the row in one pass over the tags; map to the key in the table above;
   `None` for a mixed row) and `permutation_by(n, descending, key)` for `sortByKey`'s bytes.
2. In `nvs_core_arr_sort` and `nvs_core_arr_sort_by_key`, where `merge_sort` is called: when
   `comparator.is_none()`, try the fast path first and fall through to `merge_sort` on `None`.
3. The differential tests listed under *Correctness evidence* — keep those; the two timing tables and
   the environment switch were evaluation scaffolding and do not land. `nvs-stdlib` forbids `std::env`
   (`tests/capability.rs`), so any switch belongs in `nvs-cli`'s `main`, and there should be none in
   the landed version.
4. `set_memory_cache_limit(0)` at startup, and the memory statement on the member's doc comment.
5. A decision record naming the dependency's verification evidence, and the attribution notice
   regenerated from the graph.
