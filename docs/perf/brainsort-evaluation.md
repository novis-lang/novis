# brainsort under `Core\Arr::sort`'s natural-order rows

**Verdict: adopted, for the natural-order path only, with its block cache off.** `Core\Arr::sort` and
`sortByKey` now run two sorts: [brainsort](https://crates.io/crates/brainsort) as a key sort when no
comparator was given and every compared value sits in one row of the natural ordering, and the
hand-written merge sort for everything that can throw. The permutation is the merge sort's, index for
index, on every row, shape and size tested; the whole verification is green; the member is 1.1x to 2x
faster on the small arrays a request typically sorts and 2x to 5x faster from a thousand entries. The
seam is `crates/nvs-stdlib/src/sort.rs`, and its module doc is the home of the key mapping, the split
and what a call spends. This file is the measurement and the audit.

Everything below was measured against the published crate, `brainsort 0.5.0` from crates.io (the
package's SHA-256 is `706bc94c7071c20393a470befdf96cb86d0b3fcd372fc6894da204b6b85db277`), on the
developer workstation (AMD Ryzen 7 7800X3D, Windows 11), Novis at toolchain 1.97.1, release profile
with the workspace's `overflow-checks = true` applied to the crate as to everything else.

## What moves and what does not

- **The natural-order path moves.** Without a comparator, a subject whose values are all `int`/`uint`,
  all `float`, all `string`, all `bool` or all `null` can never throw, so one pass over the tags settles
  it and the sort becomes a key sort. This covers `sort($list)`, `sort($list, {by: ...})` (the extracted
  keys are values too), `preserveKeys` and both orders, and `sortByKey` without a comparator.
- **The comparator path stays,** and so does every subject the key sort declines: a `decimal`, a
  `bytes`, an `int` beside a `float` (which the merge sort sorts through the ordering's numeric
  widening), an array or an object (which it throws on). brainsort's `sort_by` cannot abort on a
  throwing callback, and a Novis comparator costs a call through a callable per comparison, so the algorithm under
  it is not where the time goes.

## Correctness evidence

All of it lands with the seam:

- **Same permutation, in Rust** (`sort.rs`'s tests): 7 rows × 9 shapes × 30 sizes from 0 to 20 000 ×
  both orders against the merge sort over `compare_values`, plus 500-round random mixes of the integer
  corners (`i64::MIN`/`MAX`, `u64::MAX`, the `int`/`uint` boundary), of the string corners (empty,
  prefixes, a 300-byte shared prefix, high bytes, numerals) and, for the float key, an exhaustive
  pairwise check of 2 022 bit patterns including every NaN and zero sign against
  `f64::total_cmp` over the ordering's own NaN reading. Equality of the permutation proves stability
  as well as order. Mixed and orderless subjects are declined untouched on 400 random layouts, and
  the merge sort throws on each.
- **Agreement through the language**
  (`tests/conformance/core/arr-sort-key-sort-and-comparator-sort-agree-on-every-row-shape-and-size.nvst`):
  the key sort against a comparator spelling the same order, on four rows × five shapes × twelve sizes
  from 0 to 2 500, ascending, descending and with keys preserved, 720 agreements counted; `sortByKey`
  the same way. The case passes under the merge-sort-only binary too, which is what makes it an
  agreement test rather than a fixture.
- **UTF-8** (`tests/conformance/core/arr-sort-orders-utf-8-strings-by-code-point-on-both-of-its-sorts.nvst`):
  a vocabulary spanning one- to four-byte encodings, the boundaries between them, a combining
  sequence beside its precomposed form, pinned in code-point order and held to agreement between the
  two sorts on the vocabulary and on random pairs from it at 33 to 2 500 entries. Byte order is
  code-point order by UTF-8's design, and both sorts read the same bytes.
- **`bytes`** is unchanged, because it has no natural order: `rule:types/bytes` gives it none, the
  ordering throws on two `bytes` values, and `crate::ordering`'s doc says so and names the two-line
  change that would give it a row. The key sort declines the row, so the throw is the answer under
  both binaries; a comparator over `Core\Bytes::compare` sorts it.
- **Against PHP** (`tests/differential/core/arr-sort-at-size-matches-phps-sort-and-rsort.nvst` and
  `arr-sort-orders-utf-8-strings-as-phps-sort-does.nvst`): ints and non-numeric strings at 1 000 and
  5 000, and UTF-8 strings of every encoding length at 3 000, `sort` and `rsort`.
- **The attack** (`tests/hostile/core/Arr/sort/01-every-row-at-size-with-its-corners.nvs`): every
  row at 200 000 entries with its corner values mixed in, both orders, keys preserved, `sortByKey`,
  and a declined subject. Both binaries print the same five lines.
- **`bun nv verify`** green with the seam wired in, and the two userland cases print the
  same checksum as PHP.

## Performance

### Inside the member

`Core\Arr::sort($a)` through the release binary, nanoseconds per call after subtracting the same
program with zero sort rounds, medians of five interleaved runs of each binary
(`merge` is the merge sort, `key` is brainsort):

| row | shape | n | merge ns | key ns | merge / key |
|---|---|---:|---:|---:|---:|
| int | random | 8 | 286 | 188 | 1.52 |
| int | random | 32 | 1 277 | 795 | 1.61 |
| int | random | 100 | 5 014 | 3 483 | 1.44 |
| int | random | 1 000 | 81 973 | 24 149 | 3.39 |
| int | random | 10 000 | 1 354 947 | 419 467 | 3.23 |
| int | random | 100 000 | 17 527 200 | 3 622 637 | 4.84 |
| int | sorted | 8 | 269 | 179 | 1.50 |
| int | sorted | 32 | 1 071 | 542 | 1.98 |
| int | sorted | 100 | 3 899 | 1 588 | 2.46 |
| int | sorted | 1 000 | 45 489 | 13 466 | 3.38 |
| int | sorted | 10 000 | 692 109 | 260 638 | 2.66 |
| int | sorted | 100 000 | 7 455 120 | 2 176 193 | 3.43 |
| int | nearly sorted | 8 | 277 | 347 | 0.80 |
| int | nearly sorted | 32 | 1 424 | 1 080 | 1.32 |
| int | nearly sorted | 100 | 5 549 | 2 276 | 2.44 |
| int | nearly sorted | 1 000 | 60 139 | 20 724 | 2.90 |
| int | nearly sorted | 10 000 | 981 093 | 435 359 | 2.25 |
| int | nearly sorted | 100 000 | 12 079 940 | 4 382 517 | 2.76 |
| float | random | 8 | 414 | 199 | 2.08 |
| float | random | 32 | 1 343 | 1 205 | 1.11 |
| float | random | 100 | 5 054 | 3 679 | 1.37 |
| float | random | 1 000 | 89 385 | 24 027 | 3.72 |
| float | random | 10 000 | 1 518 155 | 467 249 | 3.25 |
| float | random | 100 000 | 19 908 677 | 3 792 443 | 5.25 |
| float | sorted | 8 | 270 | 191 | 1.41 |
| float | sorted | 32 | 1 065 | 545 | 1.95 |
| float | sorted | 100 | 3 998 | 1 589 | 2.52 |
| float | sorted | 1 000 | 44 932 | 11 903 | 3.77 |
| float | sorted | 10 000 | 687 451 | 225 727 | 3.05 |
| float | sorted | 100 000 | 7 786 497 | 2 015 543 | 3.86 |
| float | nearly sorted | 8 | 279 | 186 | 1.50 |
| float | nearly sorted | 32 | 1 104 | 615 | 1.79 |
| float | nearly sorted | 100 | 4 664 | 2 606 | 1.79 |
| float | nearly sorted | 1 000 | 62 906 | 19 026 | 3.31 |
| float | nearly sorted | 10 000 | 1 007 453 | 394 685 | 2.55 |
| float | nearly sorted | 100 000 | 12 709 760 | 4 042 280 | 3.14 |
| string | random | 8 | 305 | 238 | 1.29 |
| string | random | 32 | 1 452 | 1 242 | 1.17 |
| string | random | 100 | 5 377 | 4 483 | 1.20 |
| string | random | 1 000 | 75 294 | 38 972 | 1.93 |
| string | random | 10 000 | 2 059 661 | 669 489 | 3.08 |
| string | random | 100 000 | 28 961 607 | 6 816 380 | 4.25 |
| string | sorted | 8 | 305 | 245 | 1.25 |
| string | sorted | 32 | 1 254 | 722 | 1.74 |
| string | sorted | 100 | 4 760 | 3 438 | 1.38 |
| string | sorted | 1 000 | 56 484 | 30 286 | 1.87 |
| string | sorted | 10 000 | 855 449 | 720 272 | 1.19 |
| string | sorted | 100 000 | 10 798 063 | 7 064 417 | 1.53 |
| string | nearly sorted | 8 | 271 | 207 | 1.31 |
| string | nearly sorted | 32 | 1 261 | 1 018 | 1.24 |
| string | nearly sorted | 100 | 4 977 | 4 353 | 1.14 |
| string | nearly sorted | 1 000 | 70 402 | 39 244 | 1.79 |
| string | nearly sorted | 10 000 | 1 289 059 | 709 058 | 1.82 |
| string | nearly sorted | 100 000 | 17 342 470 | 7 381 907 | 2.35 |

The small end re-measured with nine repetitions and more rounds, the sizes around the crate's
in-place threshold of 32, merge ÷ key:

| row, shape | n=8 | n=16 | n=32 | n=33 | n=48 | n=64 |
|---|---:|---:|---:|---:|---:|---:|
| int random | 1.50 | 1.71 | 1.64 | 0.91 | 0.99 | 1.15 |
| int sorted | 1.44 | 1.80 | 1.98 | 1.70 | 2.18 | 2.22 |
| int nearly sorted | 1.47 | 1.65 | 1.94 | 1.64 | 1.62 | 1.78 |
| float random | 1.46 | 1.64 | 1.64 | 0.95 | 1.00 | 1.12 |
| float sorted | 1.49 | 1.75 | 1.93 | 2.20 | 2.19 | 2.29 |
| float nearly sorted | 1.49 | 1.69 | 2.01 | 1.41 | 1.42 | 1.50 |
| string random | 1.38 | 1.44 | 1.17 | 0.98 | 1.00 | 1.08 |
| string sorted | 1.28 | 1.45 | 1.70 | 1.54 | 1.11 | 1.24 |
| string nearly sorted | 1.44 | 1.32 | 1.19 | 0.92 | 0.95 | 1.02 |

The 8-entry nearly-sorted cell of the first table was noise: it reads 1.47 here. What is not noise is
the band from 33 to about 48 entries on random input, where the key sort costs up to 9 percent more —
180 ns on a 2 µs call — because the crate leaves its in-place insertion sort at 32 and builds records
for a list the merge sort finishes in a microsecond. A seam-side threshold that sent 33 to 48 entries
to the merge sort would give those cells back and take away the 1.4x to 2.2x the same sizes gain on
sorted and nearly sorted input, so none is added; lifting the crate's own small-sort limit to about
48 is the change that would close it, on brainsort's side.

The member's own walk over the subject and the loop that builds the result are the same under both
sorts and are most of a small call, which is why the small-array ratios sit near 1.5 whatever the
algorithm does. Sorted strings gain the least: brainsort's prescan recognises the order but still
reads every key, and a string key is a pointer chase.

### The permutation alone

The two sorts called directly on a `Vec<Value>` from a Rust harness (`cargo test --release`, medians
of 3 to 9 runs, the timer's granularity is 100 ns), so the member's walk and result build are out of
the picture. The ratio merge ÷ key, per row and shape, at the sizes that matter:

| row, shape | n=33 | n=100 | n=1 000 | n=10 000 | n=100 000 | n=1 000 000 |
|---|---:|---:|---:|---:|---:|---:|
| int random | 0.79 | 1.79 | 3.05 | 6.45 | 10.78 | 11.19 |
| int sorted | 8.00 | 12.50 | 24.17 | 35.66 | 43.25 | 46.97 |
| int nearly sorted | 1.33 | 3.44 | 2.75 | 4.56 | 7.08 | 7.46 |
| int few distinct | 1.67 | 3.33 | 3.48 | 5.73 | 10.79 | 12.39 |
| int + uint random | 0.69 | 1.59 | 2.30 | 6.10 | 10.29 | 13.54 |
| float random | 0.85 | 1.79 | 2.80 | 7.38 | 12.04 | 13.32 |
| float sorted | 8.00 | 13.00 | 21.36 | 31.73 | 37.15 | 46.04 |
| float nearly sorted | 8.00 | 12.50 | 4.47 | 4.37 | 8.02 | 8.14 |
| string random | 0.73 | 1.03 | 1.59 | 4.85 | 6.84 | 5.69 |
| string sorted | 2.25 | 1.41 | 2.10 | 1.19 | 1.49 | 1.64 |
| string nearly sorted | 1.00 | 0.95 | 1.95 | 2.98 | 2.98 | 2.58 |
| string few distinct | 1.25 | 1.45 | 3.08 | 6.83 | 5.37 | 7.02 |
| bool random | 1.75 | 2.44 | 7.04 | 14.84 | 15.76 | 12.56 |

In absolute terms a million random ints go from 219 ms to 19.5 ms, a million random strings from 779
ms to 137 ms, a million sorted ints from 61 ms to 1.3 ms. Reversed input, runs and organ pipes sit
between the random and sorted rows. **Where it falls behind:** at exactly 33 entries — the first size
past the crate's in-place insertion sort — a random row costs 0.7x to 0.9x of the merge sort, some
300 to 400 ns, because the records are built and permuted for a list the merge sort finishes in a
microsecond; by 100 entries the key sort is ahead again on every row but sorted and nearly sorted
strings, which stay level until about a thousand. Inside the member that dip is smaller than the
walk around it (the table above), and no cell there is below 1.1x except the one re-measured.

The block cache, measured the same way on random input — `off` is what ships:

| row | n | cache off, ns | cache on, ns | off / on |
|---|---:|---:|---:|---:|
| int | 10 000 | 102 300 | 100 300 | 1.02 |
| int | 100 000 | 1 207 600 | 1 171 700 | 1.03 |
| int | 1 000 000 | 15 838 900 | 11 971 900 | 1.32 |
| float | 10 000 | 111 800 | 109 600 | 1.02 |
| float | 100 000 | 1 217 400 | 1 066 900 | 1.14 |
| float | 1 000 000 | 13 071 700 | 10 750 200 | 1.22 |
| string | 10 000 | 229 500 | 221 000 | 1.04 |
| string | 100 000 | 3 484 000 | 3 151 400 | 1.11 |
| string | 1 000 000 | 141 271 500 | 134 985 600 | 1.05 |

Two to fourteen percent up to a hundred thousand entries and up to a third at a million, all of it
page-fault cost on fresh blocks; it is the price of memory that stays attributable to a request, and
it is paid.

### End to end, against PHP

`bun nv bench 10-array 11-array --engines nvs,php --reps 7`, work milliseconds after the
baseline subtraction, one binary per run:

| case | merge sort | brainsort | PHP 8.5 JIT | PHP / Novis before → after |
|---|---:|---:|---:|---:|
| `10-array-sort` (50 000 ints × 20 rounds) | 163.8 | 35.5 | 139.5 | 0.85x → 3.93x |
| `11-array-sort-by-field` (50 000 rows by `{by:}` × 20) | 221.5 | 91.1 | 1 025 | 4.79x → 11.26x |

The remainder of case 11 is the 50 000 `by` callbacks per round, which no sort touches.

## Memory and isolation

- **brainsort's process-wide block cache is off.** By default the crate keeps freed blocks of 64 KiB
  and more, up to 32 MiB, shared by every thread under a mutex, for the next sort of the process
  (`memory.rs`). That is memory attributable to no request, which `rule:programs/memory-priority`
  calls a hole rather than a trade-off; `sort.rs` sets the limit to zero once, before the first sort,
  through a `std::sync::Once`, and the crate's own statics hold nothing else. The cost of running
  without it is in the *permutation alone* table above. What remains: the allocate and free paths take
  the (uncontended) mutex once per large block before consulting the limit, a two-line change on the
  crate's side to avoid, and not needed.
- **What a sort spends**, per call and transient: one record per element (16 bytes for a 64-bit key,
  24 for the `int`/`uint` mix, 16 for a string), scratch of about half the records, count tables of at
  most 64 KiB, and the crate's `usize` buffer for applying the permutation — about 40 bytes an element,
  against the merge sort's 16. Freed on return. Below 33 entries the crate insertion-sorts in place
  and allocates nothing. Every block goes through Novis's own global allocator like the merge sort's
  scratch did, so the seam changes nothing about attribution; neither sort consults
  `nvs_runtime::affordable` for its scratch, which is a gap they share.
- **Failure containment.** The Rust key closures cannot fail — every value was classified before the sort
  began. Allocation failure inside the crate completes the sort through the standard library's stable
  sort with the same order. A panic inside the crate (an overflow check, say — the workspace enables
  them in release, and the crate's own ASan job runs with them on) is contained to one request by the
  helper boundary's `catch_unwind`, like any runtime panic. A stable sort's permutation is unique, so
  the answer is the same whichever kernel the CPU selects.
- **Concurrency.** The block-cache mutex is the crate's only shared state; with the cache off it is
  held for a few instructions per large block, and only sorts of 4 096 entries and more allocate
  blocks that large.

## The dependency

Pure Rust, no dependencies, MIT, MSRV 1.86, edition 2024. A `build.rs` that declares one `cfg` and
reads nothing. About 260 `unsafe` sites — the AVX2/BMI2 kernels, the record buffers, the in-place
permutation — on a path attacker-controlled data reaches. The crate's own CI (`rust.yml` in its
repository, read at the time of adoption): clippy with warnings denied on every target and feature,
the suite on six platform/toolchain legs including both Windows targets and ARM64, the scalar code
under `--cfg brainsort_no_simd`, AVX2/BMI2 forced at compile time, a 32-bit i686 leg, `no_std`, Miri
under strict provenance over the library and three test suites, AddressSanitizer with debug
assertions and overflow checks over the vector build, a golden-equivalence job against the C++
implementation's work counts, a libFuzzer job on every push and a nightly one with the corpus cached
between runs. `rule:packaging/a-c-dependency-answers-two-questions` is written for C and does not
bind here; the root manifest's row states that record by analogy, and this file is where it is
measured. The attribution notice is regenerated; `cargo deny check licenses` passes.

**What this does not check:** the scalar kernels on this machine (the AVX2 ones ran; the crate's CI
covers the scalar build against the same suites), Miri or valgrind over the seam (it adds no
`unsafe` and no refcount edge of its own), and a CPU without AVX2.

## Where it falls behind, and what could go wrong

- **Sorted or nearly sorted small strings** gain the least, 1.1x to 1.4x, because the merge sort's
  `n` comparisons on sorted input are already cheap and the prescan reads every key either way.
- **A `decimal` row stays on the merge sort.** A 96-bit mantissa with a scale has an order-preserving
  key, but building one is a second implementation of `Decimal::compare`; not worth it until a
  program sorts decimals at size.
- **The float key is Novis's, not brainsort's.** The crate's own `f64` order (`-0.0 == 0.0`, a NaN by
  its sign) differs from the ordering's; the seam maps through `total_cmp`'s bits instead, and the
  pairwise test is what holds that. A future brainsort change to `f64` cannot reach it.
- **Version risk.** 0.5.0 is a week-old publication by one author. Novis pins the exact version in
  `Cargo.lock`; a bump goes through the sweep like any other, and the differential tests are what
  catch a changed permutation.
- **The comparator path is unchanged**, so a program that sorts with `{comparator: ...}` sees none of
  this; `{by: ...}` does, because the extracted keys are values.
