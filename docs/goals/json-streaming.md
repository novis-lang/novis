---
looper: 1
title: JSON documents of any size are read and written one element at a time
summary: "Reached when Core\\Json::decodeEach reads the elements of an array inside a document, or a JSON Lines stream, one at a time from a file or a byte stream, Core\\Json::encodeEach writes one in pieces, memory stays flat at any document size, and both members owe no feature proof."
context: [crates/nvs-stdlib/src/json.rs, docs/rules/core-classes/xml-tree-and-stream.md, docs/rules/core-api/one-paradigm-per-operation.md]
stages:
  - id: 1
    title: "the rule, the two member cards, and the reader chosen by a probe"
    status: done
    context: [crates/nvs-stdlib/src/csv.rs, crates/nvs-stdlib/src/registry.rs, crates/nvs-host/src/stack.rs, deny.toml]
  - id: 2
    title: "Core\\Json::decodeEach reads one element at a time from a file or a byte stream"
    status: done
    context: [crates/nvs-stdlib/src/request.rs, crates/nvs-stdlib/src/io.rs]
  - id: 3
    title: "Core\\Json::encodeEach writes an array or JSON Lines in pieces"
    status: done
    context: [crates/nvs-stdlib/src/response.rs, crates/nvs-stdlib/src/io.rs]
  - id: 4
    title: "both members carry their feature proofs, and the JSON guide shows them"
    status: done
    context: [benches/members/README.md, tests/hostile/README.md]
  - id: 5
    title: "the goal is verified once, and every failure is fixed"
    status: done
    gate: true
checks:
  - id: stream-read
    stage: 2
    name: decodeEach holds one element at a time, follows its pointer, skips what lies outside it, and reads JSON Lines
    run: [cargo, test, --lib, "json::"]
    timeout: 1800
    tests:
      - decode_each_holds_one_element_at_a_time
      - decode_each_reads_the_array_its_pointer_names
      - decode_each_skips_a_value_outside_the_pointer_without_building_it
      - decode_each_throws_where_a_failure_is_found_after_yielded_elements
      - decode_each_reads_json_lines
  - id: stream-write
    stage: 3
    name: encodeEach writes in pieces of at most 64 KiB, writes JSON Lines, and refuses what encode refuses
    run: [cargo, test, --lib, "json::"]
    timeout: 1800
    tests:
      - encode_each_writes_an_array_in_pieces_of_at_most_64_kib
      - encode_each_writes_json_lines
      - encode_each_refuses_a_secret_value_as_encode_does
---

# JSON documents of any size are read and written one element at a time

## Why

The user, 2026-11-05: JSON of any size is a main part of a web language, and Novis wants a streaming
reader and writer. Today every JSON member holds the whole document. `jsonAs` holds the body, which
`[limits] request_body` caps at 8 MiB by default, and `encode` holds the whole text. So an import, an
export or a bulk API larger than the request's memory cannot be written in Novis at all.

The shape is what other platforms converged on, and what Novis already does for CSV: the program
names a large array inside the document, and gets its elements one at a time, each one a complete,
checked value. Memory is the nesting depth plus one element, whatever the document's size.
`Core\Csv::rows` (`crates/nvs-stdlib/src/csv.rs:1366`) reads a file record by record the same way, and
`Core\IO::writeStream` takes an `Iterable<bytes>` (`crates/nvs-stdlib/src/io.rs:3142`).

## Standing decisions

1. **Two members, and no other door.**
   - `Core\Json::decodeEach<T>($source, string $at = '', { lines?: bool, maxDepth?: int }): Iterable<T>`.
     `$at` is a JSON Pointer (RFC 6901) naming an array inside the document, and `''` is the document
     itself. Each element is decoded and checked exactly as `Core\Json::decodeAs<T>` checks a
     document, with the same messages, and every string in it is tainted. `T` is any type `decodeAs`
     takes, and `mixed` reads each element as `Core\Json::decode` would. `{ lines: true }` reads JSON
     Lines instead: one value per line, and `$at` must be `''`.
   - `Core\Json::encodeEach(Iterable<mixed> $items, { lines?: bool }): Iterable<bytes>`. It pulls one
     item at a time and gives back the array's text in pieces of up to 64 KiB, `[` and `,` and `]`
     included. `{ lines: true }` writes one value per line with no brackets. Each item is encoded and
     refused exactly as `Core\Json::encode` encodes and refuses it, a `secret` value included.
   - There is no `Core\Request::jsonEach`. `Core\Json::decodeEach<T>(Core\Request::bodyStream(), '/items')`
     is the one way, as `rule:core-api/one-paradigm-per-operation` says. There is no response member
     either: a program writes `encodeEach`'s pieces to a `Core\Response::stream`, or passes them to
     `Core\IO::writeStream`.
2. **The source.** `$source` is a `Core\IO\File`, which the member reads forward and neither opens nor
   closes, as `Core\Csv::rows` does, or an `Iterable<bytes>` such as `Core\Request::bodyStream()` or a
   request part's `content()`. Safe default, if a `Core` parameter cannot be that union: the parameter
   is `Iterable<bytes>`, and `Core\IO\File` gains `chunks(): Iterable<bytes>`, with its own proofs.
   **Built:** the registry writes the union, so the safe default is not taken. The chunks arm is
   `Iterated(TaintedBytes)`: a `bytes` arm refused `bodyStream()`'s `Iterable<tainted bytes>`, and a
   plain `bytes` chunk widens to `tainted bytes`.
3. **A walk is taken once.** Like `Core\Csv::rows`, the iterable reads forward, and a second `foreach`
   gives nothing.
4. **A failure halfway throws where it is found.** The elements already given to the program stay
   given, and the `ParseError` names the byte offset. The about page says so in plain words, and one
   example shows a transaction that rolls back.
5. **Outside the pointer, nothing is built.** A value on the way to `$at`, or after the array ends, is
   checked for syntax and depth and then skipped, and a skipped string is not kept. The document after
   the array is read to its end, so a document with trailing garbage still fails.
6. **The reader is chosen by stage 1's probe.** In order of preference:
   - **(a)** serde_json, driven inside a coroutine (`nvs_host`'s `corosensei` stacks) that suspends
     after each element and whenever it needs the next chunk. serde_json stays the only JSON parser,
     and every check the typed decoder makes is shared.
   - **(b)** a maintained pull-reader crate that passes `deny.toml` and adds no C.
   - **(c)** a small framer that finds each element's bytes and hands them to the existing decoder.
     That is a hand-written parser of JSON's outer structure, so it needs a named exception to the
     invariant that RFC 8259 is a dependency (`Cargo.toml:640`).

   The bar: memory flat within 64 KiB from a 1 MiB to a 256 MiB document of small elements; time per
   byte within twice `Core\Json::decodeAs` of the same elements at 1 MiB; and every existing decode
   test's message reproduced. The first option that meets the bar is built, and the reasons for the
   others go into the rule's Declined list.
7. **The rule.** A new fragment, `docs/rules/core-classes/json-whole-and-stream.md`, states the two shapes, the
   memory of each, decisions 1 to 5 and the Declined list. JSON earns its own sentence as XML did
   (`rule:core-classes/xml-tree-and-stream`), and that fragment's "the one place in `Core`" is
   corrected to name both.
8. **Declined, and written into the rule:** an event API (start of object, key, value), because
   element by element covers the workloads and is far simpler to use; `Core\Request::jsonEach`, a
   second door; a tree only, because a document larger than memory is a real workload.
9. **Bounds.** Every element is built under the request's memory limit, and the walk under its CPU
   and wall-time limits. A byte stream keeps the bounds its source documents, such as `bodyStream`'s.
   No new setting is added.
10. **Lanes.** It shares `json.rs` with `json-one-pass` and `json-any-size` and runs after both.
    Stage 4's examples, attacks and pages are written by subagents side by side once stages 2 and 3
    have landed.
11. **A skipped string value is not kept.** `Skip` reads every value with `deserialize_any`, and
    serde_json copies a string into its scratch buffer to give a `&str`, so a 64 MiB skipped string
    needs 64 MiB (`tests/hostile/core/Json/decodeEach/04-*`). The fix: `Skip::deserialize` calls
    `deserialize_option`, which peeks past whitespace without consuming. In `visit_some`, the byte
    serde_json peeked is the last one [`Feed`] handed out, because `io::Bytes` reads one byte per
    call. Record it in a `Cell<u8>` beside `handed`. A `"` reads with `deserialize_ignored_any`,
    whose `ignore_str` keeps nothing. `[` and `{` keep `deserialize_any` with `Skip`, so the depth
    check stays. A skipped object's **keys** still pass through serde_json's `MapKey`, which always
    copies a key into scratch. That bound, O(longest key) counted against the request, is stated in
    the rule's *Outside the pointer* paragraph and the module doc. Declined: `deserialize_ignored_any`
    for a whole skipped subtree, because `ignore_value` checks no depth and keeps one byte per open
    level, so it trades the string hole for a nesting one and drops `maxDepth`. Also declined: a
    framer that rewrites skipped values in `Feed`, which is decision 6's option (c).
12. **A `Core` walk parameter is covariant in its element** (`dba33b4041`, `d0fd6cbdaa`).
    `encodeEach(Iterable<mixed>)` refused a generator of `Product`, and `decodeEach`'s walk of
    `tainted bytes` refused an `Iterator<bytes>`. `reads_any_walk` (`crates/nvs-types/src/expr/args.rs`)
    admits a walk whose element is assignable to the declared one, at a `Core` row only, because a
    `Core` member only reads a walk. A method a program declares keeps `Iterator<T>` invariant.

## Stages

### 1. the rule, the two member cards, and the reader chosen by a probe

**Does:** settles the reader with figures, and states the feature before it is built.

**Split:** an `Explore` subagent lists how `Core\Csv::rows` is registered, typed and iterated: its
class, its slots and its iteration trio, as `file:line` with one line each.

- [x] Probe decision 6's options (a) to (c) as `#[ignore]`d Rust tests that print the figures with
  `--nocapture`. Write the figures and the verdict into `json.rs`'s module doc. `crates/nvs-stdlib/src/json.rs:1`
- [x] Write the fragment `json-whole-and-stream.md` beside the XML one, and correct the XML fragment's sentence.
  `docs/rules/core-classes/xml-tree-and-stream.md:1`
- [x] Register `decodeEach` and `encodeEach` with their reference cards, and the iterable class
  `decodeEach` returns, after `Core\Csv::rows`'s shape. `rule:core-api/reference-card`
  `crates/nvs-stdlib/src/csv.rs:326`
  - [x] The two rows, their options and cards, and the classes `Core\Json\Elements` and `Core\Json\Pieces`.
  - [x] The rosters: written-class members, generic classes, iterables, handles, host handles, dispatch.
  - [x] Stub helpers so every registered symbol resolves.
  - [x] `nvs check` types a scratch program's elements, and `ext_world`'s refused list names both classes.

### 2. Core\Json::decodeEach reads one element at a time from a file or a byte stream

**Does:** builds the reader on stage 1's choice, with decisions 2 to 5.

- [x] The reader over a `Core\IO\File` and over an `Iterable<bytes>`, as decision 2 says.
  `docs/rules/core-classes/json-whole-and-stream.md` `crates/nvs-stdlib/src/json.rs:1`
- [x] The pointer walk and the skipping of decision 5. `crates/nvs-stdlib/src/json.rs:1`
- [x] JSON Lines. `crates/nvs-stdlib/src/json.rs:1`
- [x] The five tests the `stream-read` check names. `crates/nvs-stdlib/src/json.rs:1`
- [x] `decodeEach<mixed>` type-checks, and a Novis case walks a typed array and a `mixed` one.
  `crates/nvs-types/src/expr/args.rs:2067`
  - [x] The checker takes `mixed` for `decodeEach` alone; the lowering writes a zero word in slot 0.
  - [x] A scratch `nvs run` walks `decodeEach<Order>` and `decodeEach<mixed>` over a file and over
    `bodyStream()`-shaped chunks, and a constructor that throws ends the walk.

### 3. Core\Json::encodeEach writes an array or JSON Lines in pieces

**Does:** builds the writer, which `Core\IO::writeStream` and a response stream both take.

- [x] The writer, pulling one item at a time into 64 KiB pieces. `crates/nvs-stdlib/src/json/pieces.rs:1`
- [x] The three tests the `stream-write` check names. `crates/nvs-stdlib/src/json/pieces.rs:1`
- [x] `Core\IO::writeStream` takes a `Core` walk of `tainted bytes`, such as `encodeEach`'s pieces
  and `Core\Request::bodyStream()`. `crates/nvs-stdlib/src/io.rs:147`
  - [x] Find where the checker matches a `Core` iterable class against `CoreTy::Iterated`.
  - [x] A `tainted bytes` element passes a `Blob(Qual::Neutral)` parameter; the fix and a checker test.
- [x] A Rust test that writes `encodeEach`'s pieces through `Core\IO::writeStream` and reads them back
  with `decodeEach`. `crates/nvs-stdlib/src/io.rs:3142`

### 4. both members carry their feature proofs, and the JSON guide shows them

**Does:** finishes the feature (`rule:testing/feature-proofs`). `bun nv proofs --id 'Core\Json::decodeEach'`
and `--id 'Core\Json::encodeEach'` print what each owes and where.

**Split:** writing subagents, side by side, one per file set: the two `about.md` pages; the six
examples; the `.nvst` tests; the two benches; the attacks. Every `.nvs` and `about.md` follows
AGENTS.md § *Text an end user reads*, and `bun nv proofs --comments <paths>` is run over them.

- [x] Three examples each: an import from a request body, an export of database rows to a file, and
  JSON Lines. One of them shows decision 4's rollback. `docs/examples/core/Json/decodeAs/03-reading-a-list-of-orders.nvs:1`
- [x] A bench for each, whose ladder shows memory flat from 1 MiB to 256 MiB. `benches/members/README.md:1`
  - [x] the four bench files, each run once with the debug binary
  - [x] `bun nv proofs --record-perf` on a release binary, after the `Skip` fix below
  - [x] `bun nv scaling --budget --write` stores each new bench's share (benches/members/README.md)
- [x] Attacks: an array that never ends stops at the CPU limit; one element larger than the memory
  limit stops there; nesting past `maxDepth` stops at once; a huge string outside the pointer is not
  kept. `tests/hostile/README.md:1`
  - [x] five cases written and committed; four pass
  - [x] `decodeEach/04-a-huge-string-outside-the-pointer.nvs` passes after the `Skip` fix
- [x] A skipped string is not kept: decision 11. `crates/nvs-stdlib/src/json/each.rs:237`
  - [x] `Skip` peeks with `deserialize_option` and reads a string value with `deserialize_ignored_any`
  - [x] a skipped object's keys: the bound decision 11 states, in the rule and the module doc
  - [x] a Rust test: a skipped 8 MiB string keeps the walk's peak within 64 KiB of a walk without it
- [x] A section on the JSON guide page that shows both members.
  `website/src/content/docs/guides/simple-programs/json-api.mdx:1`
- [x] The about pages and seven `.nvst` cases (`tests/conformance/core/json-*-each-*.nvst`).

### 5. the goal is verified once, and every failure is fixed

- [x] Run `looper verify` over the whole goal.
- [x] Fix every failure it names in one pass, never deleting or loosening a test.
- [x] Run `looper verify` once more, and it is green.

## Handoff

### State

**The goal is reached: every stage is done.** The third `looper verify` is green. `bun nv verify
--doc`, `bun nv owners --closes json-streaming` and `bun nv ci --check` are green, and
`looper check --name stream` passes `stream-read` and `stream-write`.

The gate stage fixed: a `Suspendable` parse's resume and unwind now run inside the sanitizer's fiber
bracket (`634f091cbe`). The capability scans no longer stop at a `#[cfg(test)] mod name;`
declaration (`ed20e2a658`). The `init` refusal test opens the folder `nvs init` checks
(`c57a6cb5f6`). Two `reject/` cases expect `tainted bytes|bytes`, because a union prints in intern
order (`bf630798d4`). The rule `core-classes/json-whole-and-stream` is `shipped`, with its guards
named (`10caa99d19`). The website data, eight pages' questions, a roadmap theme for
`setup-in-any-order` and a shorter `what-novis-checks` about page are in `cd68dc232b`.

`bun nv proofs --gate --group 'Core\Json'` owes nothing. `decodeEach`'s perf record says `growth:
unclear` with `size_growth: flat`, which the gate accepts. The sweep's `env-secret-proofs` red is
stale driver state: `looper check --name env-secret-proofs` answers `no check matches`.

The refused DONE's `website-render` red is not this goal's: at `f1d89c9377`, a clean worktree's
`bun nv render --website --check` is current. The tree carries another hand's uncommitted work
(`IntroBlock`s in `tools/nv/renderers/website-core.ts`, `data/proofs/intro-backlog.json`) that
changes the renderer without re-rendering `website/src/data/core.json` and
`website/public/llms-full.txt`. Its owner runs `bun nv render --website` before committing it.

### Next

Nothing. The driver retires the goal, and `setup-in-any-order` is next on the chain. The `tsan` leg
was not run here; it needs Linux nightly (`tools/tsan.sh`), and CI runs it.

### Backlog

- A union in a diagnostic prints its members in intern order (`TypeInterner::make_union`,
  `crates/nvs-types/src/ty.rs:597`), so a registry change elsewhere reorders a message such as
  `tainted bytes|bytes` and breaks every case that pins it. Printing members in a stable order,
  such as by their text, would end that churn. Not this goal's work.

Two follow-ups on the request path, left by `strings-in-place` and carried by `compare-recorded`.
Neither is this goal's work.

- A `.` chain with an `int` piece lowers as interpolation does, and a `Uint` part writes a `uint`
  hole. `'n ' . $int` still lowers through `helper.int_to_string`, so
  `lang:expressions/string-operators` makes 4 allocations. Anchors: `Lowering::emit_concat`,
  `crates/nvs-ir/src/lower/expr.rs:2356`, `crates/nvs-codegen/src/emit.rs:4035`.
- `Core\Str::format('%d', $n)` alone in a template hole still makes its own string through
  `nvs_int_to_string`, 4.6% of the `html-page` profile. Written as an `Int` part of the template,
  it allocates nothing (`nvs_str_concat_parts`).
- `bun nv scaling --budget` is red over 46 benches of other features, and
  `benches/members/novis/Spreadsheet-Sheet/chart.nvs` throws under `--count` ("The sheet `Sales` has
  no data row yet"). Each over bench lowers its N or writes its reason. No gate runs `--budget`, so
  its drift goes unseen until a goal stores a share.
