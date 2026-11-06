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
    status: open
    context: [benches/members/README.md, tests/hostile/README.md]
  - id: 5
    title: "the goal is verified once, and every failure is fixed"
    status: open
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

- [ ] Three examples each: an import from a request body, an export of database rows to a file, and
  JSON Lines. One of them shows decision 4's rollback. `docs/examples/core/Json/decodeAs/03-reading-a-list-of-orders.nvs:1`
- [ ] A bench for each, whose ladder shows memory flat from 1 MiB to 256 MiB. `benches/members/README.md:1`
- [ ] Attacks: an array that never ends stops at the CPU limit; one element larger than the memory
  limit stops there; nesting past `maxDepth` stops at once; a huge string outside the pointer is not
  kept. `tests/hostile/README.md:1`
- [ ] A section on the JSON guide page that shows both members.
  `website/src/content/docs/guides/simple-programs/json-api.mdx:1`

### 5. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

**Stage 2 is done.** `decodeEach<mixed>` type-checks: `written_class_of`
(`crates/nvs-types/src/expr/args.rs`, `MIXED_MEMBER`) takes `mixed` for `decodeEach` alone and sets
`ResolvedCall::written_mixed`, and `Lowering::written_type_constants` emits two zero words and a
`false` flag. A scratch `nvs run` walked `decodeEach<Order>` and `<mixed>` over a file, over `bytes`
chunks and as JSON Lines. A throwing constructor ends the walk. After a `break`, a second `foreach`
goes on with the next element, as `Core\Csv::rows` does; the rule fragment now says so.

**Stage 3's writer is built and its three tests pass** (`cargo test --lib json::pieces::tests`, run
once by name). It is `crates/nvs-stdlib/src/json/pieces.rs`, whose module doc owns its shape. No
coroutine: `advance()` pulls items with `nvs_runtime::sequence::pull` on the task's stack, encodes
each with `rendered_chunks`, and gives at most 64 KiB of a pending buffer per piece. `Core\Json\Pieces`
gained a third slot, `items`. The `not_built` stubs in `json.rs` are deleted.

**Found: `Core\IO::writeStream` refuses every `Core` walk of `tainted bytes`.**
`Core\IO::writeStream($p, Core\Request::bodyStream())` and `writeStream($p, Core\Json::encodeEach($x))`
both fail with `E0401: expected array<bytes>|Iterable<bytes>|Iterator<bytes>, found Core\Request\BodyStream`.
The parameter is `CoreTy::Iterated(&CoreTy::Blob(Qual::Neutral))` (`crates/nvs-stdlib/src/io.rs:147`),
and the walks' element is `CoreTy::TaintedBytes` (`registry.rs` `ITERABLES`, about line 3547).
Decision 1 names `writeStream` as the way to write pieces, so this goal fixes it.

The sweep's `env-secret-proofs` red is stale: no file in the tree names that check.

Not run, and worth a look in stage 5: the `tsan` leg. `src/tsan.rs` brackets a task's resume with a
fiber switch, and a `suspendable` resume has no bracket.


### Next

**Finish stage 3: `writeStream` takes a walk of `tainted bytes`, then the round-trip test.**

- Find where the checker matches a `Core` iterable class against a `CoreTy::Iterated` parameter
  (an `Explore` over `crates/nvs-types/src/` for `iterable_element` and `Iterated`). The likely cause
  is an exact element match, where a `Neutral` parameter takes every qualifier. Fix it there, so
  `bodyStream()`, a part's `content()` and `encodeEach` all pass. Add a checker test.
- Check it with a scratch `nvs run`: `Core\IO::writeStream($path, Core\Json::encodeEach($items))`,
  then `decodeEach<mixed>` over the file, and a `foreach ... as tainted bytes $piece` with
  `{ lines: true }`. A piece is `tainted bytes`, so `as bytes` does not compile.
- Then the stage's last item, the Rust round-trip test at `crates/nvs-stdlib/src/io.rs:3142`. Set
  stage 3 `done` and start stage 4 with its `**Split:**` line.

Expected red until stage 4: the proofs gate names both members' missing proofs, and a snapshot of
`nvs meta --json` gains two members, two classes, and the `source` and `items` slots.

### Backlog

Two follow-ups on the request path, left by `strings-in-place` and carried by `compare-recorded`.
Neither is this goal's work.

- A `.` chain with an `int` piece lowers as interpolation does, and a `Uint` part writes a `uint`
  hole. `'n ' . $int` still lowers through `helper.int_to_string`, so
  `lang:expressions/string-operators` makes 4 allocations. Anchors: `Lowering::emit_concat`,
  `crates/nvs-ir/src/lower/expr.rs:2356`, `crates/nvs-codegen/src/emit.rs:4035`.
- `Core\Str::format('%d', $n)` alone in a template hole still makes its own string through
  `nvs_int_to_string`, 4.6% of the `html-page` profile. Written as an `Int` part of the template,
  it allocates nothing (`nvs_str_concat_parts`).
