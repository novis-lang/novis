---
looper: 1
title: The runtime and the standard library allocate only what a result needs
summary: "Every gap record this goal owns is closed: built and its record deleted, or written into its module doc as a stated bound. Markup + Markup is linear in a loop, every bench a stage reaches declares the count it reaches, and the checks below pass."
context: [docs/rules/testing/member-perf-ledger.md]
stages:
  - id: 1
    title: "Markup + Markup appends into the left operand"
    status: done
    context: [crates/nvs-ir/src/lower/operator.rs, crates/nvs-ir/src/lower/stmt.rs, crates/nvs-stdlib/src/html.rs, crates/nvs-runtime/src/string.rs]
  - id: 2
    title: "the runtime's array reserves and is looked up by a borrowed key"
    status: done
    context: [crates/nvs-runtime/src/array.rs, crates/nvs-stdlib/src/arr.rs]
  - id: 10
    title: "an ObjectMap overwrite replaces its value in place"
    status: done
    context: [crates/nvs-runtime/src/array.rs, crates/nvs-stdlib/src/objmap.rs, crates/nvs-stdlib/src/identity_store.rs]
  - id: 11
    title: "a uint subscript takes the integer-key path"
    status: done
    context: [crates/nvs-ir/src/lower/expr.rs]
  - id: 3
    title: "a string carries its hash"
    status: done
    context: [crates/nvs-runtime/src/string.rs, crates/nvs-runtime/src/array.rs]
  - id: 4
    title: "a call into a callback allocates nothing the callback does not take"
    status: done
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
checks:
  - id: markup-ladder
    stage: 1
    name: a page built with Markup + Markup in a loop is linear
    run: [bun, nv, scaling, --check, templates/markup-concat]
    reach: true
  - id: rules
    stage: 9
    name: the rulebook resolves every rule id and citation
    run: [bun, nv, rules, --check]
  - id: owners
    stage: 9
    name: no gap record names this goal
    run: [bun, nv, owners, --closes, fewer-allocations]
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
and no stage is judged by one. A bench's new count is recorded with `bun nv proofs --record-perf
--group '<group>'`, and a ladder is judged with `bun nv scaling --check <path-filter>`.

**Small and medium work only.** A slice that turns out to need a change to the helper ABI, the object
layout, the codegen backend or the calling convention every `Core` member shares stops there. Its record
becomes a stated bound in stage 9, naming what it costs and what would close it. Big work waits until a
measured program hits it.

**`Markup + Markup` gets its new `unsafe` code.** `nvs_core_html_markup_concat` copies both sides into
a new string, so `$page = $page + html`…`` over n rows copies the page so far on every row: a probe ran
4 times the rows in 19 times the time. String `.` is linear in the same loop because `InstKind::Concat`
hands the first operand's reference to `nvs_str_concat_n`, which writes in place at a refcount of one.
A `CoreCall`'s arguments are borrowed, so `Markup` needs two pieces: a lowering that hands the left
operand's reference over only where its holder is re-pointed at the result, and a helper that appends in
place when the object and its string each have one reference. `Core\Cli\Text` shares the lowering
(`ExprInfo::CarrierComposition`), and takes the same change. It spends no memory beyond the doubling a
string `.` already spends. It is a new refcount edge, so the slice runs `MSYS_NO_PATHCONV=1 wsl.exe --
bash /mnt/e/mwl/tools/leak-check.sh <fixture>` over a fixture that reaches it.

**`Core\BigInt` conversion: divide and conquer first, a digit cap only if that does not fit.** No new
dependency: the conversion is written in `bigint.rs` over `num-bigint`'s own multiply and divide, since
a new crate on a path that reads request text is a supply-chain cost the security priority does not
pay for one member. If no sub-quadratic conversion fits a medium slice, `parse` and the text members
throw past 4300 decimal digits, Python's default, because a bounded `parse` is the safe state for text
a request sent.

**Every slice says what it spends,** per `rule:programs/memory-priority`. A cache held between calls is
per thread and bounded, never per request served.

**Lanes.** Stages 5, 6 and 8 share no file with any other stage or with each other, and the three
members of stage 5 are independent of each other: any of them can run first, or as a side goal beside
the chain. The rest run in order on a shared file. Stage 1 before 3 (`crates/nvs-runtime/src/string.rs`).
Stages 2 and 10 before 3 (`crates/nvs-runtime/src/array.rs`). Stage 2 before 4
(`crates/nvs-stdlib/src/arr.rs`). Stage 11 before 7 (`crates/nvs-ir/src/lower/expr.rs`). Stage 9 runs
last, because it states the bound of any record an earlier stage stopped on. No stage waits on another
goal.

**Trade-offs.** None for a program: no syntax, no message and no result changes. Allocation counts and
time on the request path go down. Memory goes down too, except a per-thread cache that a slice names. The
implementation gains a few owned-or-borrowed paths in the runtime's array and string, each with its guard.

## Stages

### 1. Markup + Markup appends into the left operand

**Does:** `$page = $page + html`…`` over n rows is linear, as string `.=` is (`rule:core-classes/html-auto-escape`).

- [x] `$x = $x + …` on a local carrier lowers to the carrier's append helper and re-points the binding at its answer, as `lower_string_self_concat` does for strings. `crates/nvs-ir/src/lower/stmt.rs:lower_carrier_self_append`
- [x] The helper appends into the carrier's text (and a `Cli\Text`'s runs) when the object has one owner, and copies otherwise; the string and array appends do their own copy-on-write. `crates/nvs-stdlib/src/html.rs:nvs_core_html_markup_append`, `crates/nvs-stdlib/src/cli.rs:nvs_core_cli_text_append`
- [x] A ladder builds a page with `+` in a loop and declares `expect linear` (`rule:testing/member-perf-ledger`). `benches/scaling/templates/markup-concat.nvs:1`
- [x] The leak check over the ladder's program is clean. `tools/leak-check.sh:1`
- [x] `html.rs`'s module doc says a page built with `Markup + Markup` in a loop is linear, and what the doubling spends (`rule:programs/memory-priority`). `crates/nvs-stdlib/src/html.rs:73`

### 2. the runtime's array reserves and is looked up by a borrowed key

**Does:** closes `member-that-builds-an-array-grows` and `key-argument-is-copied-onto-the`.

**Split:** an `Explore` subagent lists every caller of `key_bytes` and every member that builds a
result of a known entry count, as `arr.rs:NN` and one line each, before the session edits.

- [x] `NvsArray` has a capacity constructor, and each member that knows its entry count uses it. `crates/nvs-runtime/src/array.rs:899`, `crates/nvs-stdlib/src/arr.rs:3102`
- [x] `key_bytes` borrows the key for the length of the lookup, and each caller is read against the change. `crates/nvs-stdlib/src/arr.rs:2723`
- [x] `core/Arr/keys.nvs`, `core/Arr/filter.nvs` and `core/Arr/hasKey.nvs` declare their new counts (`rule:testing/member-perf-ledger`). `benches/members/core/Arr/keys.nvs:1`, `benches/members/core/Arr/hasKey.nvs:1`
- [x] The two records are deleted. `data/gaps/nvs-stdlib/member-that-builds-an-array-grows.json:1`, `data/gaps/nvs-stdlib/key-argument-is-copied-onto-the.json:1`

### 10. an ObjectMap overwrite replaces its value in place

**Does:** closes `an-objectmap-overwrite-allocates-its-chain-key`.

- [x] `NvsArray` has a replace-in-place that takes a borrowed key, and `Core\ObjectMap::set` over an existing key uses it. `crates/nvs-runtime/src/array.rs:1080`, `crates/nvs-stdlib/src/objmap.rs:364`, `crates/nvs-stdlib/src/identity_store.rs:104`
- [x] `core/ObjectMap/set.nvs` declares 0 allocations for an overwrite (`rule:testing/member-perf-ledger`). `benches/members/core/ObjectMap/set.nvs:1`
- [x] The record is deleted.

### 11. a uint subscript takes the integer-key path

**Does:** closes `uint-array-subscript-is-rendered-to`. A `uint` at or under `i64::MAX` takes the path
an `int` takes, and only a larger one renders (`rule:types/arrays`).

- [x] `lower_array_key` branches on the value's range for a `uint`. `crates/nvs-ir/src/lower/expr.rs:2617`
- [x] The lowering test of a `uint` subscript pins both branches. `crates/nvs-ir/src/lower/tests.rs:1802`
- [x] The record is deleted. `data/gaps/nvs-ir/uint-array-subscript-is-rendered-to.json:1`

### 3. a string carries its hash

**Does:** a repeat lookup with the same string hashes nothing. Today the array's index is a
`HashMap<NvsStr, usize>` keyed through `Borrow<[u8]>`, so every lookup hashes the key bytes again.
**SipHash stays:** `array.rs`'s § *the index map hashes with std's `RandomState`* is a security
decision, and caching the digest changes nothing about which hasher made it. `Borrow<[u8]>` has to go,
because std has no stable raw-entry API, so the change is contained to `string.rs` and `array.rs`.

- [x] `StrHeader` caches the digest of its bytes. `crates/nvs-runtime/src/string.rs:169`
- [x] The array's index keys on a type whose `Hash` writes the cached word, and a lookup from bare bytes computes it the same way. `crates/nvs-runtime/src/array.rs:26`
- [x] A guard test shows a second lookup with the same string hashes nothing. `crates/nvs-runtime/src/array.rs:946`
- [x] `string.rs`'s module doc says what the cached word spends per string (`rule:programs/memory-priority`). `crates/nvs-runtime/src/string.rs:1`

### 4. a call into a callback allocates nothing the callback does not take

**Does:** closes `call-through-a-callable-builds-its` and `walk-builds-its-callbacks-key-argument`.

- [x] `call_callable` builds its argument slots inline for a small count. `smallvec` is already a workspace dependency. `crates/nvs-runtime/src/callable.rs:184`
- [x] A walk reads the callback's arity off `CALLABLE_ARITY_SLOT` before it builds the key, and builds no key for a callback that takes one parameter. `crates/nvs-stdlib/src/arr.rs:4905`
- [x] `lang/expressions/calls.nvs` and `core/Arr/find.nvs` declare their new counts (`rule:testing/member-perf-ledger`). `benches/members/lang/expressions/calls.nvs:1`, `benches/members/core/Arr/find.nvs:1`
- [x] The two records are deleted. `data/gaps/nvs-runtime/call-through-a-callable-builds-its.json:1`, `data/gaps/nvs-stdlib/walk-builds-its-callbacks-key-argument.json:1`

### 5. a member on a warm path allocates only its result

**Does:** closes `regex-matches-allocates-on-every-call`, `a-metrics-write-allocates-on-every-call` and
`parse-pays-its-whole-setup-on`.

**Split:** the three members share no file, so each is its own slice, in any order.

- [ ] Find where `Core\Regex::matches` allocates on a cached pattern, and remove it. `core/Regex/matches.nvs` declares `allocations 0`. `crates/nvs-stdlib/src/regex.rs:1633`, `benches/members/core/Regex/matches.nvs:1`
- [ ] A `Core\Metrics` write to an existing series looks it up by a borrowed key, and renders its call site only when it stores it. `core/Metrics/gauge.nvs` declares its count. `crates/nvs-stdlib/src/metrics.rs:356`, `crates/nvs-runtime/src/metrics.rs:762`, `benches/members/core/Metrics/gauge.nvs:1`
- [ ] `Core\Csv::parse` reuses a reader per dialect, held per thread and reset per call, and its module doc says what the reader spends per thread (`rule:programs/memory-priority`). `core/Csv/parse.nvs` declares its count. `crates/nvs-stdlib/src/csv.rs:702`, `benches/members/core/Csv/parse.nvs:1`
- [ ] The three records are deleted. `data/gaps/nvs-stdlib/regex-matches-allocates-on-every-call.json:1`, `data/gaps/nvs-stdlib/a-metrics-write-allocates-on-every-call.json:1`, `data/gaps/nvs-stdlib/parse-pays-its-whole-setup-on.json:1`

### 6. a member's working memory is the size of its input

**Does:** closes `an-xml-writer-holds-a-string-per-piece`, `id-in-force-is-copied-into` and
`a-decoded-objects-issue-prefixes-grow-with-depth-squared`.

- [ ] `Core\Xml\Writer` appends to one string it alone references. `core/Xml/writer.nvs` declares its count. `crates/nvs-stdlib/src/xml.rs:2993`, `benches/members/core/Xml/writer.nvs:1`
- [ ] An event with no `id` of its own shares the walk's `lastId` string. `crates/nvs-stdlib/src/http/stream.rs:623`, `crates/nvs-stdlib/src/http/stream.rs:1313`
- [ ] The leak check covers the shared `lastId` edge, over a fixture whose events carry no `id`. `tools/leak-check.sh:1`
- [ ] The JSON decoder builds an issue's path from the open frames' keys when it makes the issue. `crates/nvs-stdlib/src/json.rs:3759`
- [ ] The three records are deleted. `data/gaps/nvs-stdlib/an-xml-writer-holds-a-string-per-piece.json:1`, `data/gaps/nvs-stdlib/id-in-force-is-copied-into.json:1`, `data/gaps/nvs-stdlib/a-decoded-objects-issue-prefixes-grow-with-depth-squared.json:1`

### 7. a constant is built once, when the unit is compiled

**Does:** closes `a-written-duration-is-built-at` and `two-identical-string-literals-are-two-data-objects`,
and takes the next shape of a literal `Core\Str::format` template. A template of plain `%d`/`%s` over
`int` and `string` already compiles to no call (`nvs_types::expr_table::FormatPiece`); one that still
calls builds an `NvsArray` for its variadic tail and parses the template again. A zero-padded `%0Nd`
is the commonest flag left, and a per-placeholder conversion helper would take it and every other flag
out of the call.

- [ ] A written duration is an immortal constant in the compiled unit (`rule:types/duration`). `crates/nvs-ir/src/lower/expr.rs:2803`
- [ ] Two identical string literals in one unit share one data object. `crates/nvs-codegen/src/emit.rs:1318`
- [ ] A literal `Core\Str::format` template with `%0Nd` compiles without a call. `crates/nvs-types/src/expr_table.rs:1416`, `crates/nvs-ir/src/lower/expr.rs:2371`
- [ ] The two records are deleted. `data/gaps/nvs-ir/a-written-duration-is-built-at.json:1`, `data/gaps/nvs-codegen/two-identical-string-literals-are-two-data-objects.json:1`

### 8. Core\BigInt converts text in sub-quadratic time, or caps its digits

**Does:** closes `bigint-division-and-decimal-text-grow-quadratically` for the members that read or
write decimal text. `parse` often reads text a request sent, so a quadratic `parse` is a
denial-of-service question first and a speed question second.

**Split:** at stage start, an `Explore` subagent reads the installed `num-bigint` 0.5 source
(`Cargo.toml:452`) and says whether `from_str_radix` and `to_str_radix` for base 10 are already
sub-quadratic, as `file:line` and one line each.

- [ ] `parse`, `toString` and `format` convert by divide and conquer, so their cost is sub-quadratic in the number's length. `crates/nvs-stdlib/src/bigint.rs:957`, `crates/nvs-stdlib/src/bigint.rs:1222`, `crates/nvs-stdlib/src/bigint.rs:1232`
- [ ] Only if no sub-quadratic conversion fits a medium slice: the digit cap of § *Standing decisions*, as a rule fragment with its own conformance case and a message that follows AGENTS.md § *Text an end user reads*. `docs/rules/core-classes/bigint-digit-cap.md:1`
- [ ] `parse`, `toString` and `format` each get a `.scale.nvs` sibling that declares the class it reaches. `benches/members/core/BigInt/parse.scale.nvs:1`, `benches/members/core/BigInt/toString.scale.nvs:1`, `benches/members/core/BigInt/format.scale.nvs:1`
- [ ] `div`, `mod`, `gcd`, `lcm`, `sqrt` and `powMod` get a sub-quadratic algorithm only where the same slice reaches it. The rest is a stated bound in the module doc, and the record is deleted. `crates/nvs-stdlib/src/bigint.rs:1`, `data/gaps/nvs-stdlib/bigint-division-and-decimal-text-grow-quadratically.json:1`

### 9. what waits for a measured wall is a stated bound, and the cross-engine suite is gone

**Does:** every record this goal owns and did not build is a stated bound in its module doc.
`benches/userland/`, its ledger and its rules are deleted. The website's comparison, `bun nv
bench-compare` over `benches/compare/`, is a separate suite and stays.

**Split:** at stage start, an `Explore` subagent re-runs the search for `benches/userland`,
`userland.ndjson`, `userland-benchmarks`, `bench-engine-list` and "userland suite" outside
`data/perf/`, and returns any hit missing from the last item. "userland" alone also means user code,
and those hits stay.

- [ ] `virtual-dispatch-resolves-by-name-not`, `nothing-inlines` and `a-switch-terminator-lowers-to-a-compare-chain` are stated bounds in their module docs, each naming its cost and what closes it, and their records are deleted. So is any record an earlier stage stopped on. `data/gaps/nvs-ir/virtual-dispatch-resolves-by-name-not.json:1`, `data/gaps/nvs-runtime/nothing-inlines.json:1`, `data/gaps/nvs-codegen/a-switch-terminator-lowers-to-a-compare-chain.json:1`
- [ ] `benches/userland/` and `data/perf/userland.ndjson` are deleted. `bun nv bench` keeps `--warm-start` alone, which CI runs (`.github/workflows/ci.yml:333`). `--serve-vs-fpm` and `benches/serve/hello.php` go with the suite, and `benches/serve/echo.nvs` stays for `bun nv bench-load`. `tools/nv/cmd/bench.ts:1`
- [ ] `rule:testing/userland-benchmarks` and `rule:tooling/bench-engine-list-is-data` are retired, and nothing cites them. `docs/rules/testing/userland-benchmarks.md:1`, `docs/rules/tooling/bench-engine-list-is-data.md:1`, `docs/rules/tooling/python-claims.md:21`, `docs/rules/testing/member-perf-ledger.md:68`, `docs/goals/mutation-testing.md:27`
- [ ] Every other mention is rewritten or removed: `Cargo.toml:4`, `Cargo.toml:31`, `CONTRIBUTING.md:169`, `tools/nv/proofs/perf.ts:392`, `tools/nv/test/rules.test.ts:57`, `docs/setup.md:31`, `docs/setup.md:33`, `docs/agent/commands.md:522`, `docs/examples/README.md:10`, `benches/members/README.md:6`, `benches/members/README.md:60`, `benches/serve/hello.nvs:6`, `crates/nvs-codegen/src/emit.rs:3385`, `crates/nvs-runtime/src/lib.rs:175`, `tests/conformance/core/arr-sort-key-sort-and-comparator-sort-agree-on-every-row-shape-and-size.nvst:13`. A comment that cited a cross-engine figure as its evidence cites its guard test instead.

## Handoff

### State

Stages 1, 2, 3, 4, 10 and 11 are done.

`call_callable_with` builds the receiver and the trimmed arguments in a stack array of
`INLINE_SLOTS` (8) values, and only a longer call spills to a `Vec`. No dependency was added:
`smallvec` is only transitive, through `hyper`. It spends 128 bytes of stack per call through a
callable. `lang/expressions/calls.nvs`, `anonymous-functions.nvs` and
`types/callable-classes-object-shapes.nvs` now count 0 allocations. `arr.rs`'s `call_with_key` calls a
callback with `($value)` alone when it declares one parameter, and `find_slot` (`find`, `findKey`,
`any`, `all`) and `countBy` read the arity once per walk. `core/Arr/find.nvs` went from 10
allocations to 1, the callable object itself. Lower counts that the change caused were declared again
in `core/Arr/filter.nvs` (12), `core/Cli/live.nvs` (11) and `core/Cli/progress.nvs` (9).
`core/Fatal/onLimit.nvs`, `onUncaughtThrow.nvs` and `core/Signal/onShutdown.nvs` had declared
`calls 0` since `ece7b500f`, and now declare `calls 1`. Both gap records are deleted, and so is
`arr.rs`'s `Known gaps` block, since no record names that module now.

The nightly run 39306106377 was cancelled because a later dispatch (39307361674) took over its
concurrency group. Nothing failed in it.

A string's header has a fifth word, the SipHash digest an array
index files it under, filled on the first lookup by `KeyRef::digest` and reset by every in-place write
that changes `len`. The keys are one `RandomState` per process (`DIGEST_KEYS`), so a cached word is
right in every array. The index is a `HashMap<Filed, usize>` whose `PassDigest` hasher passes the word
through, and it is looked up as `dyn Lookup`, so a stored key and a `KeyRef` share one map. Growing an
index re-files its keys without hashing them. `NvsStr`'s `Hash` and `Borrow<[u8]>` are gone. The
compiled-code reads (`array::entry`, `nvs_array_has_key`) pass the key's header, and the `&[u8]`
methods of `NvsArray` hash bare bytes on every call. A string literal is immortal and its data is not
writable, so `$a["name"]` still hashes on every lookup: `string.rs` § *The cached digest* states it.
Every string allocation is 8 bytes larger. Codegen reads only the published offsets, so no codegen
change was needed.

The nightly run 39278827562 tested `2dfe330c6`, which is older than the ldap fix `23b0e6611`, so its
failure needed no change.

A `uint` subscript branches on its value in
`Lowering::lower_uint_array_key`: at or under `i64::MAX` it is the same word reinterpreted as an `int`,
and above it `Helper::UintToString` renders it. The two arms join as a `Ty::Tagged` key. Every runtime
read already tells a key's tag apart, and `nvs-codegen`'s `emit_tagged_key_array_set` branches on the
tag for an `ArraySet`. `unset` still renders through `lower_rendered_array_key`. The crate doc § *an
array key is a `string`* says so. `lang/expressions/assignment.nvs` now indexes with the `uint`
itself and counts 0 allocations, where a `uint` index counted 4 for a compound write.
`a-uint-subscript-names-the-entry-its-decimal-names-on-both-sides-of-i64-max.nvst` pins every key
position at 3, `i64::MAX` and `u64::MAX`, and the leak check over the same shapes is clean. The
playbook trap about a `uint` bench index is deleted.

`NvsArray::overwrite` replaces the value of a live entry through a borrowed key, so
`core/ObjectMap/set.nvs` counts 0 allocations.

`NvsArray::with_capacity` reserves a packed vector, and the hash form takes
the room over when the first key is not `"0"`; every `Core\Arr` member that knows its result's entry
count, or an upper bound no larger than an argument it holds, reserves it. `key_bytes` returns
`KeyBytes`, a borrowed string or an integer rendered on the stack, so `hasKey` counts 0 allocations
where it counted 1. `mapKeys`, `groupBy` and `countBy` copy a callback's key out before they release
it. `keys` stays at 2 and `filter` at 16: their benches are too small to grow past the first reserve.
A filter result with a gap pays a few more bytes, since its index is sized to the subject.

Since `ece7b500f` a `Core` call counts as a call, so every `Core\Arr` bench now declares its member's
call and its callbacks'. All 53 `Core\Arr` records, and the `Core\Cli` and `Core\Html` records stage 1
left stale, are measured again. Fourteen gap records name this goal; `bun nv owners --closes
fewer-allocations` lists them, so the `owners` check stays red until stage 9.

The `looper` on PATH has no `verify`; `bun tools/looper/main.ts verify` runs it.

### Next

**Stage 5: a member on a warm path allocates only its result.** The three members share no file, so
each is its own slice, in any order. Start with `Core\Regex::matches`. Closes
`regex-matches-allocates-on-every-call`, `a-metrics-write-allocates-on-every-call` and
`parse-pays-its-whole-setup-on`.

- [ ] Find where `Core\Regex::matches` allocates on a cached pattern, and remove it. `core/Regex/matches.nvs` declares `allocations 0`. `crates/nvs-stdlib/src/regex.rs:1633`, `benches/members/core/Regex/matches.nvs:1`
- [ ] A `Core\Metrics` write to an existing series looks it up by a borrowed key, and renders its call site only when it stores it. `core/Metrics/gauge.nvs` declares its count. `crates/nvs-stdlib/src/metrics.rs:356`, `crates/nvs-runtime/src/metrics.rs:762`, `benches/members/core/Metrics/gauge.nvs:1`
- [ ] `Core\Csv::parse` reuses a reader per dialect, held per thread and reset per call, and its module doc says what the reader spends per thread (`rule:programs/memory-priority`). `core/Csv/parse.nvs` declares its count. `crates/nvs-stdlib/src/csv.rs:702`, `benches/members/core/Csv/parse.nvs:1`
- [ ] The three records are deleted. `data/gaps/nvs-stdlib/regex-matches-allocates-on-every-call.json:1`, `data/gaps/nvs-stdlib/a-metrics-write-allocates-on-every-call.json:1`, `data/gaps/nvs-stdlib/parse-pays-its-whole-setup-on.json:1`

A change in `nvs-runtime` that lowers a count is not seen by a bench whose figure is keyed on another
file. Re-measure every bench that reaches the changed path with `bun nv proofs --record-perf --force
--only <id> ...`.

### Backlog

- A string literal used as an array key hashes on every lookup, because its header is in the unit's
  read-only data. It is a stated bound in `crates/nvs-runtime/src/string.rs` § *The cached digest*.
  Closing it needs a writable digest word per literal, which is a codegen layout change.

- `nvs lsp` grows close to quadratically with the open document's size. It is tooling, not the request path, and goal `editor-fixes` owns its record. `data/gaps/nvs-lsp/open-document-analysis-grows-quadratically.json`
