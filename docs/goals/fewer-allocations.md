---
looper: 1
title: The runtime and the standard library allocate only what a result needs
summary: "Every gap record this goal owns is closed: built and its record deleted, or written into its module doc as a stated bound. Markup + Markup is linear in a loop, every bench a stage reaches declares the count it reaches, and the checks below pass."
context: [docs/rules/testing/member-perf-ledger.md, docs/perf/userland-gap.md]
stages:
  - id: 1
    title: "Markup + Markup appends into the left operand"
    status: open
    context: [crates/nvs-stdlib/src/html.rs, crates/nvs-runtime/src/string.rs, docs/perf/performance-pass.md]
  - id: 2
    title: "the runtime's array reserves, replaces in place and is looked up by a borrowed key"
    status: open
    context: [crates/nvs-runtime/src/array.rs, crates/nvs-stdlib/src/arr.rs, crates/nvs-stdlib/src/objmap.rs, crates/nvs-ir/src/lower/expr.rs]
  - id: 3
    title: "a string carries its hash"
    status: open
    context: [crates/nvs-runtime/src/string.rs, crates/nvs-runtime/src/array.rs]
  - id: 4
    title: "a call into a callback allocates nothing the callback does not take"
    status: open
    context: [crates/nvs-runtime/src/callable.rs, crates/nvs-stdlib/src/arr.rs]
  - id: 5
    title: "a member on a warm path allocates only its result"
    status: open
    context: [crates/nvs-stdlib/src/regex.rs, crates/nvs-stdlib/src/metrics.rs, crates/nvs-runtime/src/metrics.rs, crates/nvs-stdlib/src/csv.rs]
  - id: 6
    title: "a member's working memory is the size of its input"
    status: open
    context: [crates/nvs-stdlib/src/xml.rs, crates/nvs-stdlib/src/http/stream.rs, crates/nvs-stdlib/src/json.rs]
  - id: 7
    title: "a constant is built once, when the unit is compiled"
    status: open
    context: [crates/nvs-ir/src/lower/expr.rs, crates/nvs-codegen/src/emit.rs]
  - id: 8
    title: "what waits for a measured wall is a stated bound, and the userland ledger is gone"
    status: open
    context: [docs/perf/userland-gap.md]
checks: []
---

# The runtime and the standard library allocate only what a result needs

## Why

The compiler is not where a Novis program loses time. A static call, a `foreach` step and a property
read are already cheap. What costs time on the request path is the heap: a key copied to be looked up,
an argument list built per callback, a result array grown one entry at a time, a constant rebuilt on
every evaluation. The gap records this goal owns name these, and `Markup + Markup` in a loop is quadratic. This
goal does the small and medium work among them, and writes the rest down as stated bounds until a
measurement shows a program that needs it.

## Standing decisions

**Novis is measured against itself, never against another language.** The evidence for a slice is the
member ledger's counts (`rule:testing/member-perf-ledger`): a bench declares the allocation count its
member reaches, and a scaling ladder declares its growth. No stage quotes a ratio to PHP, Python or Bun,
and no stage is judged by one.

**Small and medium work only.** A slice that turns out to need a change to the helper ABI, the object
layout, the codegen backend or the calling convention every `Core` member shares stops there. Its record
becomes a stated bound in stage 8, naming what it costs and what would close it. Big work waits until a
measured program hits it.

**`Markup + Markup` gets its new `unsafe` code.** The user decided it. The two pieces are the ones
`docs/perf/performance-pass.md` § *Decided* names: a lowering that hands the left operand's reference over only
where its holder is re-pointed at the result, and a helper that appends in place when the object and its
string each have one reference. It is a new refcount edge, so the slice owes a valgrind run.

**Every slice says what it spends,** per `rule:programs/memory-priority`. A cache held between calls is
per thread and bounded, never per request served.

**Trade-offs.** None for a program: no syntax, no message and no result changes. Allocation counts and
time on the request path go down. Memory goes down too, except a per-thread cache that a slice names. The
implementation gains a few owned-or-borrowed paths in the runtime's array and string, each with its guard.

## Stages

### 1. Markup + Markup appends into the left operand

**Does:** `$page = $page + html`…`` over n rows is linear, as string `.=` is.

- [ ] The lowering hands the left operand's reference to the concat helper where its holder is re-pointed at the result, as `InstKind::Concat` does for strings. `crates/nvs-stdlib/src/html.rs:814`, `crates/nvs-runtime/src/string.rs:371`
- [ ] The helper appends into the carrier's text when the object and its string each have one reference, and copies otherwise.
- [ ] A ladder under `benches/scaling/templates/` builds a page with `+` in a loop and declares `expect linear`.
- [ ] A valgrind run over the new edge is clean.
- [ ] `docs/perf/performance-review.md` § *templates* and `docs/perf/performance-pass.md` § *Decided* say the loop is linear.

### 2. the runtime's array reserves, replaces in place and is looked up by a borrowed key

**Does:** closes `member-that-builds-an-array-grows`, `key-argument-is-copied-onto-the`,
`an-objectmap-overwrite-allocates-its-chain-key` and `uint-array-subscript-is-rendered-to`.

- [ ] `NvsArray` has a capacity constructor, and each member that knows its entry count uses it. `crates/nvs-runtime/src/array.rs:896`
- [ ] `key_bytes` borrows the key for the length of the lookup, and each caller is read against the change. `crates/nvs-stdlib/src/arr.rs:2722`
- [ ] A replace-in-place takes a borrowed key, and `Core\ObjectMap::set` over an existing key allocates nothing. `crates/nvs-stdlib/src/objmap.rs:353`
- [ ] A `uint` subscript takes the integer-key path an `int` takes. `crates/nvs-ir/src/lower/expr.rs:2617`
- [ ] Each bench these reach declares its new count, and each record is deleted.

### 3. a string carries its hash

**Does:** a repeat lookup with the same string hashes nothing. `docs/perf/userland-gap.md` § F is the
design. SipHash stays.

- [ ] `StrHeader` caches the digest of its bytes. `crates/nvs-runtime/src/string.rs:169`
- [ ] The array's index keys on a type whose `Hash` writes the cached word, and a lookup from bare bytes computes it the same way.
- [ ] A guard test shows a second lookup with the same string hashes nothing.

### 4. a call into a callback allocates nothing the callback does not take

**Does:** closes `call-through-a-callable-builds-its` and `walk-builds-its-callbacks-key-argument`.

- [ ] `call_callable` builds its argument slots inline for a small count. `smallvec` is already a workspace dependency. `crates/nvs-runtime/src/callable.rs:186`
- [ ] A walk reads the callback's arity before it builds the key, and builds no key for a callback that takes one parameter. `crates/nvs-stdlib/src/arr.rs:4906`
- [ ] `benches/members/lang/expressions/calls.nvs` and `benches/members/core/Arr/find.nvs` declare their new counts.

### 5. a member on a warm path allocates only its result

**Does:** closes `regex-matches-allocates-on-every-call`, `a-metrics-write-allocates-on-every-call` and
`parse-pays-its-whole-setup-on`.

- [ ] Find where `Core\Regex::matches` allocates on a cached pattern, and remove it. `crates/nvs-stdlib/src/regex.rs:848`
- [ ] A `Core\Metrics` write to an existing series looks it up by a borrowed key, and renders its call site only when it stores it. `crates/nvs-runtime/src/metrics.rs:762`
- [ ] `Core\Csv::parse` reuses a reader per dialect, held per thread and reset per call. `crates/nvs-stdlib/src/csv.rs:696`

### 6. a member's working memory is the size of its input

**Does:** closes `an-xml-writer-holds-a-string-per-piece`, `id-in-force-is-copied-into` and
`a-decoded-objects-issue-prefixes-grow-with-depth-squared`.

- [ ] `Core\Xml\Writer` appends to one string it alone references. `crates/nvs-stdlib/src/xml.rs:2859`, `crates/nvs-stdlib/src/xml.rs:3101`
- [ ] An event with no `id` of its own shares the walk's `lastId` string. `crates/nvs-stdlib/src/http/stream.rs:1295`
- [ ] The JSON decoder builds an issue's path from the open frames' keys when it makes the issue.
- [ ] A valgrind run covers the shared `lastId` edge.

### 7. a constant is built once, when the unit is compiled

**Does:** closes `a-written-duration-is-built-at` and `two-identical-string-literals-are-two-data-objects`,
and takes `docs/perf/userland-gap.md` § I's next shape.

- [ ] A written duration is an immortal constant in the compiled unit. `crates/nvs-ir/src/lower/expr.rs:2803`
- [ ] Two identical string literals in one unit share one data object. `crates/nvs-codegen/src/emit.rs:1318`, `crates/nvs-codegen/src/emit.rs:1420`
- [ ] A literal `Core\Str::format` template with `%0Nd` compiles without a call.

### 8. what waits for a measured wall is a stated bound, and the userland ledger is gone

**Does:** every record this goal owns and did not build is a stated bound in its module doc, and
`docs/perf/userland-gap.md` is deleted.

- [ ] `virtual-dispatch-resolves-by-name-not`, `nothing-inlines`, `a-switch-terminator-lowers-to-a-compare-chain` and `bigint-division-and-decimal-text-grow-quadratically` are stated bounds, each naming its cost and what closes it. So is any record an earlier stage stopped on.
- [ ] `docs/perf/userland-gap.md`'s items G and J are stated bounds in `crates/nvs-runtime/src/object.rs` and `crates/nvs-stdlib/src/arr.rs`.
- [ ] Its *ruled out* and *not on this list* facts move to the module docs they describe, without the cross-engine figures.
- [ ] The file is deleted, and nothing links to it.

## Handoff

### State

The goal is new. The work was M12's other half, and M12 is deleted. The eighteen records it owned now
name this goal. Item K of `docs/perf/userland-gap.md` has landed (`crates/nvs-runtime/src/array.rs:2407`),
and its row is stale.

### Next

Stage 1, `Markup + Markup`, on the html and string files:
- the lowering hands the left reference over (`crates/nvs-stdlib/src/html.rs:814`)
- the in-place append and its ladder (`crates/nvs-runtime/src/string.rs:371`)

### Backlog

- `nvs lsp` grows close to quadratically with the open document's size. It is tooling, not the request path. `docs/perf/performance-pass.md`
