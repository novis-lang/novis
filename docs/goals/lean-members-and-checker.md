---
looper: 1
title: "a Core member, a type check and an editor request do no work twice"
summary: "Reached when the listed members allocate only their result, the checker's walks clone no names, and an editor request resolves no path it already resolved."
context: [crates/nvs-stdlib/src/arr.rs, crates/nvs-stdlib/src/str.rs, crates/nvs-types/src/signatures.rs, crates/nvs-lsp/src/completion.rs, docs/rules/testing/member-perf-ledger.md]
stages:
  - id: 1
    title: the array members allocate per result, not per element
    status: done
    context: [crates/nvs-stdlib/src/arr.rs, crates/nvs-stdlib/src/heap.rs, docs/rules/testing/member-perf-ledger.md]
  - id: 2
    title: the text and byte members scan once and copy once
    status: done
    context: [crates/nvs-stdlib/src/str.rs, crates/nvs-stdlib/src/bytes.rs, crates/nvs-stdlib/src/regex.rs, crates/nvs-stdlib/src/csv.rs, crates/nvs-stdlib/src/encoding.rs, crates/nvs-stdlib/src/bigint.rs]
  - id: 3
    title: a stream reads in large steps into one buffer
    status: done
    context: [crates/nvs-stdlib/src/io.rs, crates/nvs-stdlib/src/http/transport.rs, crates/nvs-stdlib/src/net.rs, crates/nvs-stdlib/src/zip.rs, crates/nvs-stdlib/src/compress.rs]
  - id: 4
    title: "the checker's walks borrow their names"
    status: done
    context: [crates/nvs-types/src/signatures.rs, crates/nvs-types/src/expr/calls.rs, crates/nvs-types/src/expr/mod.rs, crates/nvs-hir/src/hierarchy.rs]
  - id: 5
    title: an editor request resolves a path once
    status: done
    context: [crates/nvs-lsp/src/completion.rs, crates/nvs-lsp/src/index.rs, crates/nvs-lsp/src/parse_cache.rs, crates/nvs-lsp/src/document.rs, crates/nvs-lsp/src/server.rs]
  - id: 6
    title: "the goal is verified once, and every failure is fixed"
    status: done
    gate: true
checks:
  - id: bucket-members-lean
    stage: 1
    name: "groupBy and countBy allocate per bucket, not per element"
    run: [cargo, test, --lib, --, a_bucket_member_allocates_per_bucket]
    timeout: 1800
    tests:
      - a_bucket_member_allocates_per_bucket
  - id: completion-resolves-once
    stage: 5
    name: "a completion at a bare name resolves no indexed path again"
    run: [cargo, test, --lib, --, a_completion_resolves_no_indexed_path_again]
    timeout: 1800
    tests:
      - a_completion_resolves_no_indexed_path_again
---

# a Core member, a type check and an editor request do no work twice

## Why

These are small changes, each under an hour by reading, that remove work a call does for every
element, every expression or every request. Each was read in the code. None was timed.

**Members.**

- `Core\Arr::groupBy` and `countBy` allocate two or three times per element for a bucket that
  already exists (`crates/nvs-stdlib/src/arr.rs:2721`, `:4420`). `unique`, `diff`, `intersect` and
  `reverse` build an index key string per list element (`:5514`, `:5994`, `:3820`).
- `Core\Arr::append` and its siblings copy entry by entry into an array with no capacity (`:3527`).
- A comparator sort merges runs that are already in order (`:5058`).
- `Core\Bytes::indexOf` compares a window at every offset (`crates/nvs-stdlib/src/bytes.rs:775`).
  On repeated bytes that is the length of the haystack times the length of the needle.
- `Core\Str::lower` and `upper` map every character through the Unicode tables, ASCII too
  (`crates/nvs-stdlib/src/str.rs:4068`). `trim` and `replace` copy the subject when nothing changed
  (`:2596`, `:3035`). `split` with a `limit` counts every separator first (`:2177`).
- `Core\IO\File::readLine` costs two system calls and an 8 KiB copy per line
  (`crates/nvs-stdlib/src/io.rs:2961`). An HTTP reply body is read 8 KiB at a time and copied three
  times (`crates/nvs-stdlib/src/http/transport.rs:557`).

**The checker.** A `QName` is a `Vec<String>`. Every ancestor walk clones one per step into a
`seen` set, and `resolve_method` clones the whole `MethodSig`
(`crates/nvs-types/src/signatures.rs:2485`, `:2034`). About 25 hooks per resolved call build
`owner.to_string()` (`crates/nvs-types/src/expr/calls.rs:755`). A variable read allocates its name
(`crates/nvs-types/src/expr/mod.rs:362`). `mark_overridden_methods` walks the ancestors once per
method (`crates/nvs-types/src/signatures.rs:1107`).

**The editor.** A completion at a bare name calls `canonicalize` on every indexed file, twice
(`crates/nvs-lsp/src/completion.rs:3371`, `crates/nvs-lsp/src/index.rs:1307`). Each analysis
resolves each graph file's path four or five times (`crates/nvs-lsp/src/parse_cache.rs:179`). A
keystroke in a file with `require` parses its whole graph again in an empty cache
(`crates/nvs-lsp/src/document.rs:401`).

## Standing decisions

**Limits: what stops this goal.**

- **The list is closed.** Only the members and sites named in the stages are touched. A member not
  listed stays as it is, however similar.
- **A member is kept when its ledger count drops.** `rule:testing/member-perf-ledger` re-measures a
  member whose file moved. A member whose allocations and calls did not drop is restored to its
  old code, and its item is ticked as declined with both figures.
- **Only the changed members are re-measured,** in one `bun nv proofs --record-perf` run per stage,
  narrowed to the stage's members. No full ledger run.
- **A change that alters a result is declined at once.** If a conformance case or a member test has
  to change its expected output, the member's edit is discarded. Limits, taint steps and the
  DoS-safe hashers are not touched.
- **An item gets one attempt, and a stage two sessions,** counted in `### State`. At the ceiling the
  open items move to `### Backlog` as declined.
- **No item is added.**

**`memchr` becomes a direct dependency of `nvs-stdlib`.** It is already in the lock file through
`regex`. Its `memmem` search replaces the window compare, inside the same 64 KiB windows and the
same deadline poll. **What it spends:** nothing new in the binary. Stage 2 measured no drop in
`Core\Bytes::indexOf` and restored it, so `memchr` stays out of `nvs-stdlib`'s own list.

**A member returns its argument when the result equals it,** after the stage checks that the taint
and secret state of the returned value is what the copied value had. Safe default if they can
differ: keep the copy for that member.

**`readLine` keeps a read-ahead buffer beside the open file.** Every other member of the handle
drops or serves from the buffer. **What it spends:** up to 8 KiB per open file that has read a
line, for the life of the handle, inside the request's memory limit.

**A reply body is read in 64 KiB steps into a reused buffer.** **What it spends:** 56 KiB more per
client call while its reply is read.

**The checker keeps `QName` as it is.** The walks borrow it. Declined: an interned name type, which
is a change across four crates with no measured program behind it. Declined: checking files on
several threads, for the same reason.

**The editor's paths are resolved when a document opens and when the index records a file.**
Declined: skipping the staleness `stat` of each graph file per request, because a file changed
outside the editor must still be seen.

**Lanes:** stages 1 to 3 touch only `crates/nvs-stdlib/` and run in order. Stage 4 touches
`crates/nvs-types/` and `crates/nvs-hir/`, and stage 5 `crates/nvs-lsp/`. Each of the two can run
as a side goal.

## Stages

### 1. the array members allocate per result, not per element

**Does:** remove the per-element allocations of the bucket and set members, size the appending
members, and skip ordered runs in the merge.

- [x] `crates/nvs-stdlib/src/arr.rs:2721`: `groupBy` looks its bucket up by borrowed bytes and
  builds the key argument only for a callback that takes it (`rule:testing/member-perf-ledger`).
  Allocations 37 → 27 per op.
- [x] `crates/nvs-stdlib/src/arr.rs:4420`: `countBy` overwrites an existing bucket
  (`rule:testing/member-perf-ledger`). Allocations 11 → 8.
- [x] `crates/nvs-stdlib/src/arr.rs:5514`: `unique`, `set_member` at `:5994`, `reverse` at `:3820`
  and `mapKeys` at `:2611` use `slot_key` and `store_at` (`rule:testing/member-perf-ledger`).
  `unique` 28 → 14, `diff` 25.5 → 12.3, `intersect` 25.5 → 12.8, `mapKeys` 18 → 12. Declined:
  `reverse`, whose reversed list keys land in the hash form and are rendered either way.
- [x] `crates/nvs-stdlib/src/arr.rs:3527`: `append`, `prepend` at `:3561`, `replaceRange` at `:3397`
  and `appendAll` at `:5864` reserve their result (`rule:testing/member-perf-ledger`).
  `appendAll` 7 → 6 is kept. Declined and restored: `append` 4 → 4, `prepend` 4 → 4,
  `replaceRange` 2 → 2, calls unchanged.
- [x] `crates/nvs-stdlib/src/arr.rs:5058`: `merge_sort` copies a pair of runs that is already in
  order, and swaps two buffers (`rule:testing/member-perf-ledger`). Declined and restored:
  `sort` allocations 5 → 5 and calls 1 → 1.
- [x] `crates/nvs-stdlib/src/arr.rs:3347`: `slice` starts at its slot when the array has no holes
  (`rule:testing/member-perf-ledger`). Declined and restored: allocations 2 → 2, calls 2 → 2.
- [x] `crates/nvs-stdlib/src/arr.rs:2690`: the test `a_bucket_member_allocates_per_bucket`
  (`rule:testing/perf-two-mechanisms`).

### 2. the text and byte members scan once and copy once

**Does:** search bytes with `memmem`, give the case members an ASCII path, and stop copying a
result that is the argument or is built twice.

- [x] `crates/nvs-stdlib/src/bytes.rs:775`: `scanned` calls `memchr::memmem::find`
  (`rule:testing/member-perf-ledger`). Declined and restored: `indexOf` allocations 0 → 0 and
  calls 1 → 1, `contains` the same. `memchr` is not added.
- [x] `crates/nvs-stdlib/src/str.rs:4068`: `lower`, `upper` at `:4090` and `fold` at `:4137` map an
  ASCII subject by bytes (`rule:testing/member-perf-ledger`). Declined and restored: each 1 → 1
  allocations, calls 2 → 2.
- [x] `crates/nvs-stdlib/src/str.rs:2596`: the trim members and `replace` at `:3035` return the
  argument when nothing changed (`rule:testing/member-perf-ledger`). `trim` 1 → 0.5 is kept, with
  `unchanged` beside it. Declined and restored: `trimStart` 1 → 1, `trimEnd` 1 → 1, `replace`
  1 → 1, calls unchanged.
- [x] `crates/nvs-stdlib/src/str.rs:2177`: `split` stops counting at its `limit`
  (`rule:testing/member-perf-ledger`). Declined and restored: allocations 5 → 5, calls 2 → 2.
- [x] `crates/nvs-stdlib/src/str.rs:3975`: the pad members read the cached length
  (`rule:testing/member-perf-ledger`). Declined and restored: `padStart` 1 → 1, `padEnd` 1 → 1.
- [x] `crates/nvs-stdlib/src/regex.rs:2304`: `replace` and `replaceWith` at `:2446` reserve the
  subject's length and return the argument on no match (`rule:testing/member-perf-ledger`).
  `replace` 6.5 → 5.5 and `replaceWith` 5 → 3 are kept.
- [x] `crates/nvs-stdlib/src/csv.rs:957`: a keyed row retains its header names
  (`rule:testing/member-perf-ledger`). `parse` 21 → 20 is kept, and its bench declares 20.
  `rows` 51 → 51 shares the code.
- [x] `crates/nvs-stdlib/src/encoding.rs:1405`: `encodeText` and `decodeText` at `:1435` share the
  payload for UTF-8, after the same validation (`rule:testing/member-perf-ledger`). Declined and
  restored: `encodeText` 2 → 2, `decodeText` 3 → 3. Both benches use `Windows1252`.
- [x] `crates/nvs-stdlib/src/bigint.rs:2371`: `compareTo`, `neg` at `:2310` and `abs` at `:2318`
  work on the stored bytes (`rule:testing/member-perf-ledger`). `compareTo` 2 → 0, `neg` 4 → 1
  and `abs` 4 → 1 are kept, over `Value::bytes_handle` in `nvs-runtime`.
- [x] `crates/nvs-stdlib/src/identity_store.rs:90`: `chain_key` writes its digits by hand
  (`rule:testing/member-perf-ledger`). Declined and restored: `ObjectMap::get`, `set`, `has` and
  `ObjectSet::add` 0 → 0 allocations.

### 3. a stream reads in large steps into one buffer

**Does:** buffer `readLine`, read reply bodies in larger steps, and reuse the read buffers of
sockets and archives.

- [x] `crates/nvs-stdlib/src/io.rs:2961`: the read-ahead buffer, dropped by every other member of
  the handle (`rule:programs/memory-priority`). This item has one session. If a member of the
  handle cannot keep its position promise, it is declined.
  - [x] The buffer lives beside the file in the runtime's table, and `open_file_mut` gives it back.
  - [x] The test `a_line_read_ahead_leaves_every_other_member_at_the_line_end` passes.
  - [x] Allocations 2 → 1, bytes 76 → 58.
  - [x] `readLine.scale.nvs` passes: `growth: flat`. The buffer keeps its length and an `end`, so
    a fill no longer zeroes a whole block.
- [x] `crates/nvs-stdlib/src/http/transport.rs:557`: 64 KiB steps into a reused buffer, read
  straight into the held body where no framing stands between (`rule:programs/memory-priority`).
  Kept without a figure: every `Core\Http\Client` bench goes through the answer table and never
  reaches `Framed::pull`.
- [x] `crates/nvs-stdlib/src/net.rs:1887`: `Datagram::receive` and `Stream::read` at `:1633` read
  into a buffer they do not zero first (`rule:programs/memory-priority`). Safe default if that
  needs `unsafe`: a reused per-thread buffer. Per-thread buffer, taken out for the call.
  `receive` allocations 7 → 6 and bytes 65753 → 217, `read` 2 → 1.
- [x] `crates/nvs-stdlib/src/zip.rs:1206`: one block per extraction
  (`rule:programs/memory-priority`). `extract` allocations 306 → 305, bytes 297344 → 231808.
- [x] `crates/nvs-stdlib/src/compress.rs:890`: the output reserves a hint capped by the ceiling
  (`rule:programs/memory-priority`). `decompress` 20 → 18 is kept. Declined and restored: the
  encode hint, `compress` 130 → 130.

### 4. the checker's walks borrow their names

**Does:** stop the ancestor walks, the roster hooks and a variable read from allocating names.

- [x] `crates/nvs-types/src/signatures.rs:2485`: one ancestor walk over borrowed names serves
  `:2034`, `:2555`, `:2587` and `crates/nvs-hir/src/hierarchy.rs:879`, and a caller that only reads
  gets a borrowed `MethodSig` (`rule:programs/memory-priority`).
  - [x] `nvs_hir::find_in_ancestors` is the one walk, and `implements_interface` uses it.
  - [x] the property, method, interface-argument and iteration walks use it.
  - [x] `resolve_method_ref` serves the instance call and every caller that only reads.
- [x] `crates/nvs-types/src/expr/calls.rs:755`: the `Core` roster hooks run only for an owner in
  `Core`, and the extension hook at `:830` stays ungated (`rule:security/capability-check-at-the-door`).
  - [x] each hook's owner match was read: only `reject_ungranted_call` and the extension hook
    reach a class outside `Core`, and both stay ungated.
  - [x] `every_registry_class_is_under_core` holds the registry to the gate.
- [x] `crates/nvs-types/src/expr/mod.rs:362`: a variable read keeps the borrowed name
  (`rule:programs/memory-priority`).
- [x] `crates/nvs-types/src/signatures.rs:1107`: `mark_overridden_methods` walks the ancestors once
  per class (`rule:programs/memory-priority`).
- [x] `crates/nvs-types/src/expr/calls.rs:161`: the call label at `:718` and the description at
  `crates/nvs-types/src/check.rs:1618` are built on the error path
  (`rule:programs/memory-priority`).
- [x] `crates/nvs-hir/src/qname.rs:19`: `QName::into_segments`, and the four copies of
  `qname_segments` go (`rule:programs/memory-priority`).

### 5. an editor request resolves a path once

**Does:** keep each document's and each indexed file's resolved path, and parse a program's graph
through the session's cache.

- [x] `crates/nvs-lsp/src/index.rs:1307`: `all_declarations()` iterates the stored files, and
  `every_type` at `crates/nvs-lsp/src/completion.rs:3371` is called once per completion
  (`rule:programs/memory-priority`).
  - [x] `SymbolIndex::all_declarations` walks the stored entries.
  - [x] `in_reach` lists the types once, for its items and `taken_among`.
- [x] `crates/nvs-lsp/src/parse_cache.rs:179`: the resolved key is memoised until the cache
  settles, and `record_graph` at `crates/nvs-lsp/src/document.rs:472` resolves nothing again
  (`rule:programs/memory-priority`).
  - [x] `ParseCache::keys` keeps each path's key, and `retain` clears it when a file leaves.
  - [x] `record_graph` gives the entry the document's stored key and keeps the walk's keys.
- [x] `crates/nvs-lsp/src/document.rs:343`: a document stores its resolved key, and `text_of` at
  `:458` reads one buffer (`rule:programs/memory-priority`).
  - [x] `Document::key` is resolved in `open`, and every reader of a document's key uses it.
  - [x] `text_of` takes a key and copies one buffer, or reads the file.
- [x] `crates/nvs-lsp/src/document.rs:401`: `resurvey` parses through the session's `ParseCache`
  (`rule:programs/memory-priority`). This item has one session. If the lender tests differ, it is
  declined. **Declined:** `nvs_hir::lenders::Survey` loads through its own map at its own ids, and
  a `ParseCache` entry serves only the id its path is pinned at. Routing one through the other
  needs a new `nvs-hir` API, outside this stage's lane.
- [x] `crates/nvs-lsp/src/server.rs:2909`: inlay hints are built for the asked range
  (`rule:programs/memory-priority`).
  - [x] `hints::in_range` skips a statement outside the range and builds no hint outside it.
- [x] `crates/nvs-lsp/src/server.rs:3503`: the test
  `a_completion_resolves_no_indexed_path_again` (`rule:testing/perf-two-mechanisms`).
  - [x] it ran once by name and passed.

### 6. the goal is verified once, and every failure is fixed

- [x] Run `looper verify` over the whole goal.
- [x] Fix every failure it names in one pass, never deleting or loosening a test.
- [x] Run `looper verify` once more, and it is green.

## Handoff

### State

Every stage is done, and the goal is reached. The gate's `looper verify` is green, and so are
`bun nv verify --doc`, `bun nv owners --closes lean-members-and-checker`, `bun nv ci --check`
and both acceptance checks. The gate fixed four things. `a_completion_resolves_no_indexed_path_again`
moved to a test module in `crates/nvs-lsp/src/server.rs`, because `completion.rs` may not build an
index or write a file. `all_declarations` joined the index suite's bookkeeping methods. The 241 stale
perf records were taken again, and three benches now declare their lower allocation counts.
`Core\Arr::hasKey`'s bench runs 4096 operations, so its ramp is not read from noise. The testing
chapter's slug bench output shows the counts the leaner `Core\Str::trim` gives. The decisions on
`QName` and on resolving editor paths are now in `crates/nvs-hir/src/qname.rs` and
`crates/nvs-lsp/src/parse_cache.rs`. Sessions used: stage 1: 1, stage 2: 1, stage 3: 2, stage 4: 1,
stage 5: 1, stage 6: 1.

The sweep's `env-secret-proofs` is still stale state in looper's service. Its log does not
exist, and nothing in the tree defines it.

### Next

- `docs/goals/README.md:1`: retire the goal, then `one-copy-each` is the live goal.

### Backlog

- Declined before the run: building `Core\Xml`'s tree straight from the reader, a hole-based sift
  in `Core\Heap`, and a parsed-key cache in the crypto members. Each is more than a session or has
  a recorded reason against it.
- Not settled: whether `call_callable` at `crates/nvs-runtime/src/callable.rs:193` can skip its
  per-element parameter checks. The checks may be what keeps a callback's arguments typed.
