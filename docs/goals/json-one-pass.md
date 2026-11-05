---
looper: 1
title: JSON is read into objects in one pass and written straight to where it goes
summary: "Reached when a typed JSON decode builds its objects while it parses, with no tree in between, and an encode writes its bytes into the destination with no extra copy, every check and message unchanged."
context: [crates/nvs-stdlib/src/json.rs, docs/rules/core-classes/derive-reports-every-field.md, docs/rules/errors/an-allocation-past-the-ceiling-is-refused-in-front-of-itself.md]
stages:
  - id: 1
    title: "a byte writer's write that fits asks nothing, and Response::json hands its document to the body"
    status: open
    context: [crates/nvs-runtime/src/budget.rs, crates/nvs-stdlib/src/response.rs, crates/nvs-runtime/src/ctx/output.rs]
  - id: 2
    title: "a typed decode parses with the context in hand, and an object of scalar fields is built as it is read"
    status: open
    context: [crates/nvs-stdlib/src/request.rs, crates/nvs-runtime/src/ctx/inbound.rs, docs/rules/http-server/buffering-readers-share-the-body-and-streaming-readers-consume-it.md]
  - id: 3
    title: "nested objects and lists are built as they are read, and the Found tree is gone"
    status: open
  - id: 4
    title: "the changed members are measured again, and their allocation counts are declared"
    status: open
    context: [benches/members/README.md, benches/compare/profile.sh]
  - id: 5
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: writer-fast-path
    stage: 1
    name: a byte writer's write that fits its buffer asks the budget nothing, and one that grows still asks first
    run: [cargo, test, --lib, "budget::"]
    timeout: 1800
    tests:
      - a_write_that_fits_the_buffer_asks_the_budget_nothing
      - a_write_that_grows_the_buffer_asks_the_budget_first
  - id: one-pass-decode
    stage: 3
    name: a typed decode builds its objects while it parses, and a failure halfway frees every object it built
    run: [cargo, test, --lib, "json::"]
    timeout: 1800
    tests:
      - a_typed_decode_builds_each_object_when_its_closing_brace_is_read
      - a_failure_after_built_objects_frees_every_one_of_them
      - a_failure_inside_a_list_frees_the_elements_built_before_it
---

# JSON is read into objects in one pass and written straight to where it goes

## Why

A profile of `nvs serve` on one pinned core (`bun nv bench-compare --profile json-api`, 2026-11-05 at
`293a3b9b9`) served the `json-api` case at 18,041 requests a second. Of the serving core:

- `Core\Request::jsonAs` took 12.3%. That splits into the parse (`parse_against`, 8.4%), the build
  (`ReadAs::build` and `decode_object`, about 8%), and freeing the tree in between (`Found`'s drop,
  3.2%).
- `Core\Response::json` took 9.2%. `ByteWriter::put` alone took 2.1%, because serde_json writes each
  `"`, `:` and `,` as its own call, and the call is not inlined across crates.

A typed decode today makes two passes. serde_json reads the text into a `Found` tree
(`crates/nvs-stdlib/src/json.rs:2339`), where every object is a boxed slot array and every value a
counted Novis value. A second walk builds the instances from the tree, and then the tree is freed. The
split exists because `Core\Request::jsonAs` reads text borrowed from the request, and the build needs
`&mut Ctx` on the same carrier (`crates/nvs-stdlib/src/request.rs:3158`).

The member ledger counts per operation today (`data/perf/members.md`): `Core\Json::decodeAs` 7
allocations, `Core\Request::jsonAs` 32, `Core\Http\Response::jsonAs` 6, `Core\Test\Response::jsonAs` 7,
`Core\Json::encode` 18 and `Core\Response::json` 9.

Every typed JSON read shares this code: `Core\Json::decodeAs`, `Core\Request::jsonAs`, the HTTP
client's `jsonAs`, JWT claims and the test client. `ByteWriter` is shared by every `Core` member that
writes a payload: JSON, CSV, compression, JWT, I/O and the output buffer.

## Standing decisions

1. **The class with its own decoder keeps its path.** A class that wrote `DECODE` gets a document
   (`read_marked`, then `hydrate`), because its own code takes a value. Only the tree path changes.
2. **Every check and every message stays exactly as it is:** the depth limit, a repeated key, an
   unknown key, a missing field, a wrong type, the integer range, the budget, and taint on every
   decoded string (`rule:core-classes/derive-reports-every-field`). The existing tests are the
   specification. Their expected output is frozen, and no test's expected message changes.
3. **An object is built when its closing brace is read.** Its keys may come in any order, so its
   field values wait in one slot stack that the whole decode shares. Each object pushes its contract's
   field count onto the stack and pops it once it is built. That replaces one boxed slot array per
   object with one buffer per decode, which grows to depth × fields and is freed when the call
   returns.
4. **Every way out frees what the normal one does.** A failure halfway releases every value on the
   slot stack and every instance already built, before the `ParseError` is thrown. The two tests the
   `one-pass-decode` check names prove it with the request's memory gauge back at its start.
5. **The request body is held as `Arc<[u8]>`** (`crates/nvs-runtime/src/ctx/inbound.rs:315`), so
   `jsonAs` clones the handle and parses with `&mut Ctx` free. The bytes are never copied. It spends
   one `Arc` header per request that holds its body. Safe default, if `Arc` conflicts with redacting
   the recorded copy: keep the two calls, `read_as` and `build`, but let both use the slot stack, so
   the tree still goes.
6. **The parse may run on a spare stack** (`on_spare_stack`, `crates/nvs-stdlib/src/json.rs:2894`). The
   build runs there with it, and the closure takes the context.
7. **`Core\Response::json` hands its buffer to the body.** It writes no `String` in between when the
   body is empty, which is the common case. A new `Ctx::write_body_owned(Vec<u8>)`
   (`crates/nvs-runtime/src/ctx/output.rs:353`) takes the buffer when nothing was written yet, and
   copies it in otherwise. `Core\Json::encode` keeps its one copy into the string.
8. **`ByteWriter::put` gets a fast path** (`crates/nvs-runtime/src/budget.rs:1388`). A write that fits
   the buffer's capacity is `#[inline]` and checks nothing else. Growth goes to an out-of-line function
   that asks the budget first, as today (`rule:errors/an-allocation-past-the-ceiling-is-refused-in-front-of-itself`).
   The writer keeps starting empty, so this spends no memory.
9. **Out of scope:** a SIMD parser, replacing serde_json, and the sort callback in `json-api`'s
   program. Those are a dependency, unsafe code, or too small to matter.
10. **Measured by counts.** The evidence is the member ledger's allocations per operation and the
    profile's shares, before and after, in the commit messages. Requests per second is a secondary note.
11. **Lanes.** Stage 1 touches the encode half of `json.rs` and stages 2 and 3 its decode half, so all
    three run in order in one lane. The goal shares no file with `strings-in-place`, which may run as a
    side goal beside it.

## Stages

### 1. a byte writer's write that fits asks nothing, and Response::json hands its document to the body

**Does:** makes the encode side cheaper for every payload writer, and removes one copy from
`Core\Response::json`.

- [ ] Split `ByteWriter::put` into decision 8's inline fast path and an out-of-line growth path. Write
  the two tests the `writer-fast-path` check names. Rewrite the struct's doc as a whole.
  `rule:errors/an-allocation-past-the-ceiling-is-refused-in-front-of-itself`
  `crates/nvs-runtime/src/budget.rs:1388`
- [ ] Add `Ctx::write_body_owned` beside `write_body`, with its "What it spends" line.
  `crates/nvs-runtime/src/ctx/output.rs:353`
- [ ] Let the encoder return its `Vec<u8>` (`Encodable::written`), and have `Core\Response::json` pass
  it to `write_body_owned`. `crates/nvs-stdlib/src/json.rs:923` `crates/nvs-stdlib/src/response.rs:1976`

### 2. a typed decode parses with the context in hand, and an object of scalar fields is built as it is read

**Does:** the keystone. The parse can build instances, and the simplest contract no longer makes a
tree.

**Split:** an `Explore` subagent lists every caller of `read_as`, `ReadAs::build`, `decode_as` and
`held_body`, as `file:line` with one line each. Six files reach them today: `arr.rs`, `http.rs`,
`json.rs`, `jwt.rs`, `request.rs` and `test.rs` under `crates/nvs-stdlib/src/`.

- [ ] Hold the request body as `Arc<[u8]>`, and give `held_octets` a variant that returns a handle.
  `rule:http-server/buffering-readers-share-the-body-and-streaming-readers-consume-it`
  `crates/nvs-runtime/src/ctx/inbound.rs:315` `crates/nvs-stdlib/src/request.rs:3345`
- [ ] Make `decode_as` one call that parses with the context in hand, and move `jsonAs` and every
  other caller onto it. `read_as` and `ReadAs` go, unless decision 5's fallback applies.
  `crates/nvs-stdlib/src/json.rs:2146` `crates/nvs-stdlib/src/request.rs:3171`
- [ ] Add the slot stack of decision 3, and let `Fill` build an object whose fields are all scalars or
  strings when its closing brace is read. `crates/nvs-stdlib/src/json.rs:2574`
  `crates/nvs-stdlib/src/json.rs:3708`

### 3. nested objects and lists are built as they are read, and the Found tree is gone

**Does:** finishes the one pass, then deletes the tree and its drop.

- [ ] Build a nested object's instance into its parent's slot as it closes.
  `crates/nvs-stdlib/src/json.rs:3897` `crates/nvs-stdlib/src/json.rs:4441`
- [ ] Build a list of objects element by element (`FillList`), and an `array<T>` document the same way
  (`decode_each`). `crates/nvs-stdlib/src/json.rs:2683` `crates/nvs-stdlib/src/json.rs:3634`
- [ ] Release the slot stack and every built instance on each error path, as decision 4 says. Write
  the three tests the `one-pass-decode` check names. `crates/nvs-stdlib/src/json.rs:2900`
- [ ] Delete `Found`, `Read` and `Seen`'s tree arms, and rewrite the module doc's decode section as a
  whole. `crates/nvs-stdlib/src/json.rs:2339`

### 4. the changed members are measured again, and their allocation counts are declared

**Does:** records what the goal bought, in counts, and makes the counts a bound.

- [ ] Run `bun nv proofs --record-perf --force --id '<feature>'` for `Core\Json::decodeAs`,
  `Core\Request::jsonAs`, `Core\Http\Response::jsonAs`, `Core\Test\Response::jsonAs`,
  `Core\Json::encode` and `Core\Response::json`, and for every other figure `bun nv proofs --gate`
  then names as stale. `benches/members/README.md:1`
- [ ] Add a `// bench: allocations N` line to the decodeAs and jsonAs benches, with the new count.
  `benches/members/core/Json/decodeAs.nvs:1` `benches/members/core/Request/jsonAs.nvs:1`
- [ ] Run `bun nv bench-compare --profile json-api`, and put its shares and counts beside the ones in
  *Why* in the commit message. `benches/compare/profile.sh:1`

### 5. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Nothing is built yet. The figures in *Why* were taken on 2026-11-05 at `293a3b9b9`.

### Next

**Stage 1**, files `crates/nvs-runtime/src/budget.rs`, `crates/nvs-runtime/src/ctx/output.rs`,
`crates/nvs-stdlib/src/json.rs` (encode half) and `crates/nvs-stdlib/src/response.rs`. No gate.

- [ ] The fast path and its two tests. `crates/nvs-runtime/src/budget.rs:1388`
  `rule:errors/an-allocation-past-the-ceiling-is-refused-in-front-of-itself`
- [ ] `Ctx::write_body_owned`. `crates/nvs-runtime/src/ctx/output.rs:353`
- [ ] The encoder returns its buffer. `crates/nvs-stdlib/src/json.rs:923`
- [ ] `Core\Response::json` hands it to the body. `crates/nvs-stdlib/src/response.rs:1976`

### Backlog
