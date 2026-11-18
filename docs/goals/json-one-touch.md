---
looper: 1
title: "a typed JSON decode touches each property once"
summary: "Reached when Core\\Json::decodeAs and Core\\Request::jsonAs match a declared key on its bytes, check each value against its property's type as it is read, build each object straight from its slots, allocate only the object, its storage and its strings, and report every issue with the path and message it has today."
context: [crates/nvs-stdlib/src/json.rs, crates/nvs-stdlib/src/json/reader.rs, docs/rules/testing/member-perf-ledger.md]
stages:
  - id: 1
    title: a declared key is matched on its bytes, at the place the last key left off
    status: done
  - id: 2
    title: a value is checked when it is read, and an object is built straight from its slots
    status: open
    context: [crates/nvs-runtime/src/object.rs]
  - id: 3
    title: the per-value bookkeeping is paid only where it is used, and the ledger records the counts
    status: open
    context: [benches/members/core/Request/jsonAs.nvs]
  - id: 4
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: key-match
    stage: 1
    name: "a declared key is matched without being validated as text, in one compare when the document keeps the declared order, and an undeclared or repeated key is handled as before"
    run: [cargo, test, --lib, --, a_declared_key_is_matched_on_its_bytes, a_document_in_declared_order_costs_one_compare_per_key, a_key_out_of_order_or_repeated_is_matched_as_before]
    timeout: 1800
    tests:
      - a_declared_key_is_matched_on_its_bytes
      - a_document_in_declared_order_costs_one_compare_per_key
      - a_key_out_of_order_or_repeated_is_matched_as_before
  - id: one-touch
    stage: 2
    name: "a typed decode allocates the object, its storage and its strings and nothing more, and gives the value or the issues the generic decode gives"
    run: [cargo, test, --lib, --, a_typed_decode_allocates_the_object_its_storage_and_its_strings, a_typed_decode_agrees_with_the_generic_decode_on_every_document, a_constructor_is_found_once_for_a_whole_decode]
    timeout: 1800
    tests:
      - a_typed_decode_allocates_the_object_its_storage_and_its_strings
      - a_typed_decode_agrees_with_the_generic_decode_on_every_document
      - a_constructor_is_found_once_for_a_whole_decode
  - id: ledger
    stage: 3
    name: "the JSON members' recorded counts are the ones this goal left, and their size ladders are still linear"
    run: [bun, nv, scaling, --check, json/]
    timeout: 3600
---

# a typed JSON decode touches each property once

## Why

`Core\Request::jsonAs<Order>()` is a third of the comparison's `json-api` request. Measured on
2026-11-17 on a document of 1.7 KB, one object and a list of 20 objects of four properties each, the
decode takes about 16 µs, which is about 185 ns per property.

The text is read once, and no tree of the document is made (`crates/nvs-stdlib/src/json.rs:277`). The
cost is in what happens to each value after it is read:

- **Every value is touched twice.** `fill` (`json.rs:3403`) reads a scalar into a generic value and
  stores it in a slot. At the object's closing brace, `decode_fields` (`:4794`) walks the slots again,
  checks each against its property's type, and moves it into a second vector in constructor order.
- **Every object allocates five times beside its strings.** `Fields::new` makes two vectors (`:4923`),
  `construct` makes the object, its storage and a vector of arguments
  (`crates/nvs-runtime/src/object.rs:5474`, `:5486`).
- **Every object looks its constructor up by name** (`object.rs:5456`).
- **Every key is validated as UTF-8 before it is compared** (`json/reader.rs:557`), and compared with
  each declared key from the first on (`json.rs:3427`).
- **Every value pays bookkeeping it rarely needs:** a `RefCell` borrow for the deadline count
  (`json.rs:2674`), and a call per slot that looks for a generic array to keep (`json.rs:3682`).

## Standing decisions

**A program sees no difference.** Every document gives the value it gives today, or the same issues
with the same paths and messages in the same order (`rule:core-classes/derive-reports-every-field`).
Constructors run in today's order, each once every one of its properties is a value. A class that
wrote its own decoder keeps its path (`json.rs:2867`). The memory ceiling is asked before every
allocation as today, and the deadline is polled every 256 values as today.

**The count law (my call).** Decoding a list of n objects of one class, each with s string
properties, allocates n × (2 + s) times plus a number that does not grow with n: the object, its
storage, and one string per string property. The test takes the difference between n = 100 and
n = 10, so the constant never has to be known. Today the same difference is n × (5 + s).

**Key matching (my call).** A key is compared as bytes. The search starts at the property after the
one matched last and wraps, so a document that keeps the declared order costs one compare per key. No
hash table is built: a class has few properties, and a table per class is memory held for the life of
the unit. A key is validated as text only when it is undeclared, since it is kept. A declared key
with an escape is not validated either: a declared key is text, so equal bytes are text too. A key
that is not text and meets the memory ceiling is validated first, so its error is today's.

**One touch (my call).** `fill` checks a declared scalar against its property's type when it reads
it. A value of the wrong type stores the issue the second pass would have reported, at the same
place in the list of issues. The slots are in constructor order with the receiver's place first, so
`construct` takes the slice and builds no vector. Which slots were filled is a bit set held in the
`Builder`, 64 properties wide, and a vector only for a class with more. The constructor is found once
per contract and kept beside it for the decode.

**Not in this goal.** The reader's grammar and its byte loop, a SIMD reader, the encoder, and the copy
of each string value into a Novis string. That copy is needed: the document's bytes are freed when
the request ends.

**What it spends.** Nothing held. Each decoded object allocates three times fewer than today.

**Limits: what stops this goal.** Counts decide and never the clock. An item gets one attempt. An
item whose change would alter a value, an issue, a path, a message or the order of constructor calls
is not built, and `### Backlog` says why. Each stage gets two sessions, counted in `### State`; at the
second session's end its open items move to `### Backlog` as declined. No item is added to a stage.
No figure of another server's holds this goal open.

**Lanes.** The goal touches `crates/nvs-stdlib/src/json.rs`, `crates/nvs-stdlib/src/json/reader.rs`
and `construct` in `crates/nvs-runtime/src/object.rs`. It shares no file with `request-fixed-cost`
and can run beside it as a side goal. Its stages share `json.rs` and run in order.

## Stages

### 1. a declared key is matched on its bytes, at the place the last key left off

- [x] `crates/nvs-stdlib/src/json/reader.rs:839` `Reader::key` and `:534` `Reader::text`: a key with no
  escape is given to its caller as bytes with no validation. The caller validates it where it keeps
  it (`json.rs:3429`, the undeclared key). Every other caller of `key` keeps today's validated text.
  - [x] `Reader::next_keyed` gives a key as `Keyed::Key`, whose `Key::text` validates it where `next` would.
  - [x] `Reader::next` and its callers are unchanged.
- [x] `crates/nvs-stdlib/src/json.rs:3417` the key event in `fill`: the compare is on bytes, and the
  search starts after the property matched last in this object, kept in the object's `Filling` frame
  (`:3317`).
  - [x] `Filling::Object` keeps `from`, and `declared` searches from it and wraps.
  - [x] Only an undeclared key is validated, and the ceiling's order of errors is kept.
- [x] The three tests the `key-match` check names, in the tests module of `json.rs` beside `:9179`. The
  first two count with a `#[cfg(test)]` counter of validations and of compares. Run once by name.
  - [x] Written beside `a_typed_integer_too_large_for_its_field_is_that_fields_issue`, on `read_ints`.
  - [x] Run once by name: three pass.

### 2. a value is checked when it is read, and an object is built straight from its slots

**Split:** an `Explore` subagent first lists the existing tests that compare the typed decode with the
generic one, and every caller of `nvs_runtime::construct` and of `decode_fields`, as `path:line` and
one line each.

- [ ] `json.rs:3524` the scalar arm of `fill` and `:2551` `scalar_of`: a declared scalar is checked
  against its property's type here, with the conversion `scalar` (`:5701`) makes today. A mismatch
  stores its issue for that slot.
- [ ] `json.rs:3642` `Builder::open` and `:3667` `close`: the slots are in constructor order with the
  receiver's place first, and the filled set is the bit set of § *One touch*.
- [ ] `json.rs:3711` `build_object`: presence, defaults, nullability and the allowed set
  (`:5552` `outside`) are checked over the slots in place, in declared order, so the issues come out
  in today's order. It no longer goes through `Fields` (`:4890`) for an object `fill` read. `Fields`
  and `walk` (`:4851`) stay for `hydrate`'s callers.
- [ ] `crates/nvs-runtime/src/object.rs:5447` `construct`: a second entry that takes the constructor
  already found and a slice whose first place is the receiver's. The existing entry keeps its
  signature and calls it. `Contract` (`json.rs:2866`) finds the constructor once per class.
- [ ] A list of objects (`json.rs:3498`, `:5856` `decode_list`): each element `fill` built is taken as
  it is, with no `Positions` frame per element.
- [ ] The three tests the `one-touch` check names. The second builds documents from a fixed seed over
  every property type, missing and extra keys, wrong types, nulls and nesting, and compares the typed
  decode with `read_polled` and `hydrate` (`:2869`). Run once by name.

### 3. the per-value bookkeeping is paid only where it is used, and the ledger records the counts

- [ ] `json.rs:2674` `stopped`: the count to the next poll is a plain integer in the `Builder`, and the
  shared word is read every 256 values as today.
- [ ] `json.rs:3667` `Builder::close`: `keep_arrays` (`:3682`) runs only for a contract that has a
  property that can hold a generic array.
- [ ] `crates/nvs-stdlib/src/request.rs:3285` and `json.rs:2853`: `check_codec` runs once for a
  `jsonAs` call.
- [ ] `json.rs:277` *`decodeAs` reads the text against the class* and its *What it spends* at `:299`
  are rewritten as a whole with the count law.
- [ ] `bun nv proofs --record-perf --group 'Core\Json'` and `--group 'Core\Request'`, once each. The
  `allocations` line of `benches/members/core/Request/jsonAs.nvs` and of
  `benches/members/core/Json/decodeAs.nvs` is the new count. No count of either member rises.
- [ ] `bun nv scaling --check json/` once. Every ladder is linear.
- [ ] `bun nv bench-compare --profile json-api` once. Write the share of `nvs_core_request_json_as`
  into `### State`.

### 4. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Written on 2026-11-17 from a conversation with the person, who asked why Novis is third on `json-api`
in the comparison. Their calls: write goals for it, and give up nothing else for these cases. Every
other choice in § *Standing decisions* is the authoring session's call.

Stage 1 is done in one session (stage 2 has used none). `fill` reads with `Reader::next_keyed`
(`reader.rs` `Keyed`, `Key`), and `declared` (`json.rs`, beside `fill_slot`) matches a key on its
bytes from `Filling::Object`'s `from`. The `key-match` tests pass, run once by name. No existing
test is expected to go red. Two properties with one key do not compile (`types_e08.rs`), so the
wrapping search never changes which property matches.

The sweep's reds were stale: `env-secret-proofs` has no log, and the nightly `supply chain` run
44073672172 built `2934ebe1ae`, before `chrono` was removed.

### Next

Stage 2. Stage 1 moved the anchors in `json.rs`: today `scalar_of` is `:2551`, `build_object`
`:3762`, `keep_arrays` `:3733`, `check_codec` `:4081` and `decode_list` `:5907`, and `construct` is
`object.rs:5447`. Find the rest with `bun nv peek --locate`. The tests helper `read_ints` reads an
`int` shape and may serve the `one-touch` tests.

### Backlog

