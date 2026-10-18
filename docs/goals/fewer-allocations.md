---
looper: 1
title: The runtime and the standard library allocate only what a result needs
summary: "Every gap record this goal owns is closed: built and its record deleted, or written into its module doc as a stated bound. Markup + Markup is linear in a loop, every bench a stage reaches declares the count it reaches, and the checks below pass."
context: [docs/rules/testing/member-perf-ledger.md]
stages:
  - id: 1
    title: "Markup + Markup appends into the left operand"
    status: open
    context: [crates/nvs-stdlib/src/html.rs, crates/nvs-runtime/src/string.rs]
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
    title: "Core\\BigInt converts text in sub-quadratic time, or caps its digits"
    status: open
    context: [crates/nvs-stdlib/src/bigint.rs]
  - id: 9
    title: "what waits for a measured wall is a stated bound, and the cross-engine suite is gone"
    status: open
    context: [tools/nv/cmd/bench.ts, docs/rules/testing/userland-benchmarks.md, docs/rules/tooling/bench-engine-list-is-data.md]
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
becomes a stated bound in stage 9, naming what it costs and what would close it. Big work waits until a
measured program hits it.

**`Markup + Markup` gets its new `unsafe` code.** The user decided it. `nvs_core_html_markup_concat`
copies both sides into a new string, so `$page = $page + html`…`` over n rows copies the page so far on
every row: a probe ran 4 times the rows in 19 times the time. String `.` is linear in the same loop
because `InstKind::Concat` hands the first operand's reference to `nvs_str_concat_n`, which writes in
place at a refcount of one. A `CoreCall`'s arguments are borrowed, so `Markup` needs two pieces: a
lowering that hands the left operand's reference over only where its holder is re-pointed at the
result, and a helper that appends in place when the object and its string each have one reference.
Until it lands, `Core\Html::join` is the linear form. It spends no memory beyond the doubling a string
`.` already spends. It is a new refcount edge, so the slice owes a valgrind run.

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
- [ ] `crates/nvs-stdlib/src/html.rs`'s module doc says a page built with `Markup + Markup` in a loop is linear.

### 2. the runtime's array reserves, replaces in place and is looked up by a borrowed key

**Does:** closes `member-that-builds-an-array-grows`, `key-argument-is-copied-onto-the`,
`an-objectmap-overwrite-allocates-its-chain-key` and `uint-array-subscript-is-rendered-to`.

- [ ] `NvsArray` has a capacity constructor, and each member that knows its entry count uses it. `crates/nvs-runtime/src/array.rs:896`
- [ ] `key_bytes` borrows the key for the length of the lookup, and each caller is read against the change. `crates/nvs-stdlib/src/arr.rs:2722`
- [ ] A replace-in-place takes a borrowed key, and `Core\ObjectMap::set` over an existing key allocates nothing. `crates/nvs-stdlib/src/objmap.rs:353`
- [ ] A `uint` subscript takes the integer-key path an `int` takes. `crates/nvs-ir/src/lower/expr.rs:2617`
- [ ] Each bench these reach declares its new count, and each record is deleted.

### 3. a string carries its hash

**Does:** a repeat lookup with the same string hashes nothing. Today the array's index is a
`HashMap<NvsStr, usize>` keyed through `Borrow<[u8]>`, so every lookup hashes the key bytes again.
**SipHash stays:** `array.rs`'s § *the index map hashes with std's `RandomState`* is a security
decision, and caching the digest changes nothing about which hasher made it. `Borrow<[u8]>` has to go,
because std has no stable raw-entry API, so the change is contained to `string.rs` and `array.rs`.

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
and takes the next shape of a literal `Core\Str::format` template. A template of plain `%d`/`%s` over
`int` and `string` already compiles to no call (`nvs_types::expr_table::FormatPiece`); one that still
calls builds an `NvsArray` for its variadic tail and parses the template again. A zero-padded `%0Nd`
is the commonest flag left, and a per-placeholder conversion helper would take it and every other flag
out of the call.

- [ ] A written duration is an immortal constant in the compiled unit. `crates/nvs-ir/src/lower/expr.rs:2803`
- [ ] Two identical string literals in one unit share one data object. `crates/nvs-codegen/src/emit.rs:1318`, `crates/nvs-codegen/src/emit.rs:1420`
- [ ] A literal `Core\Str::format` template with `%0Nd` compiles without a call.

### 8. Core\BigInt converts text in sub-quadratic time, or caps its digits

**Does:** closes `bigint-division-and-decimal-text-grow-quadratically` for the members that read or
write decimal text. `parse` often reads text a request sent, so a quadratic `parse` is a
denial-of-service question first and a speed question second.

- [ ] `parse`, `toString` and `format` convert by divide and conquer, so their cost is sub-quadratic in the number's length. Not checked: whether `num-bigint` (`Cargo.toml:452`) already has such a path. `crates/nvs-stdlib/src/bigint.rs:949`, `crates/nvs-stdlib/src/bigint.rs:1214`, `crates/nvs-stdlib/src/bigint.rs:1224`
- [ ] Only if no sub-quadratic conversion fits a medium slice: `parse` and the text members throw past a fixed digit count, as Python does with its 4300-digit default. The user decided that order. The cap is a rule fragment with its own conformance case, and its message follows the prose rules.
- [ ] Each converting member's bench has a `.scale.nvs` sibling that declares the class it reaches.
- [ ] `div`, `mod`, `gcd`, `lcm`, `sqrt` and `powMod` get a sub-quadratic algorithm only where the same slice reaches it. The rest is a stated bound in the module doc, and the record is deleted.

### 9. what waits for a measured wall is a stated bound, and the cross-engine suite is gone

**Does:** every record this goal owns and did not build is a stated bound in its module doc.
`benches/userland/`, its ledger and its rules are deleted. The website's comparison, `bun nv
bench-compare` over `benches/compare/`, is a separate suite and stays.

- [ ] `virtual-dispatch-resolves-by-name-not`, `nothing-inlines` and `a-switch-terminator-lowers-to-a-compare-chain` are stated bounds, each naming its cost and what closes it. So is any record an earlier stage stopped on.
- [ ] `benches/userland/` and `data/perf/userland.ndjson` are deleted. `bun nv bench` keeps `--warm-start` alone, which CI runs (`.github/workflows/ci.yml:310`). `--serve-vs-fpm` and `benches/serve/hello.php` go with the suite, and `benches/serve/echo.nvs` stays for `bun nv bench-load`. `tools/nv/cmd/bench.ts:1`
- [ ] `rule:testing/userland-benchmarks` and `rule:tooling/bench-engine-list-is-data` are retired, and nothing cites them.
- [ ] Every other mention is rewritten or removed: `Cargo.toml:4`, `tools/nv/cmd/audit.ts:174`, `tools/nv/proofs/perf.ts:388`, `docs/setup.md:31`, `docs/agent/commands.md:513`, `docs/examples/README.md:10`, `benches/members/README.md:6`, `benches/serve/hello.nvs:6`, and the crate comments `grep -rn userland crates benches/abi-probe tests` names. A comment that cited a cross-engine figure as its evidence cites its guard test instead.

## Handoff

### State

The goal is new. The work was M12's other half, and M12 is deleted. The eighteen records it owned now
name this goal. An empty array is already a per-thread singleton (`crates/nvs-runtime/src/array.rs:2407`).

### Next

Stage 1, `Markup + Markup`, on the html and string files:
- the lowering hands the left reference over (`crates/nvs-stdlib/src/html.rs:814`)
- the in-place append and its ladder (`crates/nvs-runtime/src/string.rs:371`)

### Backlog

- `nvs lsp` grows close to quadratically with the open document's size. It is tooling, not the request path, and goal `editor-fixes` owns its record. `data/gaps/nvs-lsp/open-document-analysis-grows-quadratically.json`
