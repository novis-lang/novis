---
looper: 1
title: JSON costs the same per byte at every size up to the limits
summary: "Reached when decoding and encoding JSON costs the same per byte from 1 KiB to the request and memory limits, a document is never held more than its stated number of times, and ladders up to 32 MiB pin both."
context: [crates/nvs-stdlib/src/json.rs, benches/members/README.md, docs/rules/errors/an-allocation-past-the-ceiling-is-refused-in-front-of-itself.md]
stages:
  - id: 1
    title: "ladders by document size measure every JSON member from 1 KiB to 32 MiB"
    status: done
    context: [benches/members/core/Json/decodeAs.scale.nvs, benches/members/core/Json/encode.scale.nvs, benches/members/core/Budget/memoryPeak.nvs]
  - id: 2
    title: "every cliff the ladders found is fixed, and a large response is written in chunks"
    status: open
    context: [crates/nvs-stdlib/src/response.rs, crates/nvs-stdlib/src/request.rs, crates/nvs-runtime/src/budget.rs, crates/nvs-runtime/src/ctx/output.rs]
  - id: 3
    title: "the ladders are bounds: linear growth and a peak per document byte"
    status: open
  - id: 4
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: size-bounds
    stage: 2
    name: a JSON document of any size is held no more often than stated, and allocates the same per byte at every size
    run: [cargo, test, --lib, "json::"]
    timeout: 1800
    tests:
      - a_response_document_of_any_size_peaks_at_its_size_plus_one_chunk
      - a_decoded_body_of_any_size_is_held_once
      - allocations_per_document_byte_do_not_grow_with_the_document
---

# JSON costs the same per byte at every size up to the limits

## Why

Novis is a language for web servers, and reading and writing JSON is most of what an API server does.
The user's bar, 2026-11-05: Novis does not need the fastest JSON reader, but JSON must perform
reasonably at every size. A 20 MiB document should cost about what twenty 1 MiB documents cost, and a
document that fits the limits must not fail because the server holds it three times.

Nothing measures that today. The JSON ladders grow by item count or nesting depth, not by bytes:
`decodeAs.scale.nvs` goes up to depth 512, `encode.scale.nvs` up to 16,384 items, and
`Request/jsonAs.scale.nvs` up to 256. They are linear as far as they go, but the largest is far below
`[limits] request_body`'s 8 MiB default and `[limits] memory`'s 128 MiB.

Known suspects, each fixed only if the ladder shows it:

- `Core\Response::json` builds the whole document in one `ByteWriter`, which grows by doubling. A
  reallocation holds the old and the new block at once, so the peak is up to three times the document
  before the body gets it.
- `Core\Json::encode` copies the finished buffer into a string, so the peak is twice the document.
- The request body is read to its end (`whole_body`, `crates/nvs-stdlib/src/request.rs:3382`). Where
  it grows by doubling, it holds up to twice the body while it reads, even when the length is known.
- `float_roundtrip` costs about twice the time per float parsed (`Cargo.toml:656`). That is a stated
  price, so a document of floats sets its own bar.
- An object with many keys, a very long string, and a string full of escapes or non-ASCII text.

## Standing decisions

1. **"Reasonable" is decided here.** For every member and every shape, the time per document byte at
   the largest size is within 1.5 times the time per byte at 64 KiB. Allocations per document byte do
   not grow with the size. Peak memory per document byte is stated per member, and the tests the
   `size-bounds` check names pin it:
   - `Core\Response::json`: the value, plus the document, plus one chunk.
   - `Core\Json::encode`: the value, plus twice the document.
   - `Core\Request::jsonAs` and `Core\Json::decodeAs`: the body once, plus the objects built, plus a
     scratch that grows with the nesting depth and not with the size.
2. **The shapes:** a wide array of small objects, one object with many keys, long strings (ASCII,
   escape-heavy and non-ASCII), many integers, many floats, and deep nesting. The sizes: 1 KiB,
   64 KiB, 1 MiB, 8 MiB and 32 MiB. `Core\Request::jsonAs` stops at 8 MiB, the default request body
   limit, and the larger rung raises `request_body` in that bench's own `nvs.toml`.
3. **A large `Core\Response::json` is written in 64 KiB chunks.** The encoder writes into a list of
   64 KiB chunks, which become the body's chunks. Nothing is copied or reallocated, and the peak is the
   document plus one chunk. Each chunk asks the budget before it is allocated, as every growth does
   (`rule:errors/an-allocation-past-the-ceiling-is-refused-in-front-of-itself`). A document under one
   chunk still goes through `json-one-pass`'s `write_body_owned`. Safe default, if the body cannot take
   a chunk list: grow the one buffer by 1.5 times instead of 2.
4. **A body with a known length is read into one buffer of that length,** checked against
   `[limits] request_body` before the allocation. A body without a length keeps today's growth.
5. **No new parser in this goal.** serde_json's own time is measured beside each rung: a Rust bench
   parses the same bytes into `serde_json::Value`. If a member meets decision 1 and its own work is
   at most three times serde_json's, it is reasonable. A new parser (SIMD or not) is a separate goal,
   and only if serde_json's own parse is more than half of a member's time at 1 MiB or more and that
   member still misses decision 1. That goal would start with a probe and an adoption bar, as
   `windows-cpu-clock` did.
6. **Out of scope: documents larger than memory.** Reading or writing a JSON document in pieces
   without holding it whole would be a new `Core` API. It is not part of this goal.
7. **Measured by counts first.** Allocations and peak bytes are exact, so the tests and the ladders'
   bounds use them. Time per byte comes from the ladders' growth, which the perf proofs already judge.
8. **Lanes.** It shares `json.rs`, `response.rs` and `budget.rs` with `json-one-pass`, so it runs
   after that goal. It shares no file with `strings-in-place`.
9. **Two instruments, split by what each can see.** The member ladders are each member's
   `.scale.nvs`, whose size is the document's bytes from 1 KiB, over one mixed shape: a list of
   66-byte orders, each with an int, a float, an escaped non-ASCII string and a nested list. They see
   what a member adds above the parse: the `string` copy, the request body, the response body. The
   ramp stops where its counts agree, so it judges growth and never reaches every rung. The per-shape
   figures at the five fixed sizes come from `json::tests::serde_reference_ladder`, which runs every
   decision-2 shape through serde_json's own `Value`, `read` and `written`, and prints time,
   allocations and peak per byte. Thirty ladder files, one per member and shape, were declined: the
   shape changes the parse and the encoder, which the Rust bench measures directly, and not what
   the members add.

## Stages

### 1. ladders by document size measure every JSON member from 1 KiB to 32 MiB

**Does:** finds the cliffs before anything is changed.

**Split:** writing subagents write the ladder benches, one per member, side by side. Their files are
disjoint. A `reader` subagent then reads the recorded figures and returns, per member and shape, the
time per byte, allocations per byte and peak per byte at each size.

- [x] Add a byte-size ladder bench for `Core\Json::decode`, `Core\Json::decodeAs`, `Core\Json::encode`,
  `Core\Request::jsonAs` and `Core\Response::json`, over decision 2's shapes and sizes. Each reads its
  peak with `Core\Budget::memoryPeak`. `benches/members/core/Json/encode.scale.nvs:1`
- [x] Add the Rust bench that parses the same documents with serde_json alone, `#[ignore]`d, printing
  its figures with `--nocapture`. `crates/nvs-stdlib/src/json.rs:1`
- [x] Record the ladders with `bun nv proofs --record-perf --force --id '<feature>'`, and write the
  table the `reader` returns into the commit message. Each result that misses decision 1 becomes an
  item of stage 2, with its anchor. `benches/members/README.md:1`

### 2. every cliff the ladders found is fixed, and a large response is written in chunks

**Does:** brings every member under decision 1. Decision 3 is built whatever the ladder shows,
because the three-times peak follows from how the buffer grows.

- [x] Write `Core\Response::json`'s document in 64 KiB chunks that the body takes, as decision 3 says.
  `rule:errors/an-allocation-past-the-ceiling-is-refused-in-front-of-itself`
  `crates/nvs-stdlib/src/response.rs:1976` `crates/nvs-runtime/src/ctx/output.rs:353`
  - [x] `Ctx::write_body_chunks` keeps the chunks as the body's leading part, and a later write follows them.
  - [x] `Completion::output` is a `Written`: one chunk keeps its one buffer, and a reader that needs
    one buffer joins once (`into_bytes`, `to_bytes`).
  - [x] The server's `Answer::Whole` sends one frame per chunk, with an exact `Content-Length`.
  - [x] An 8 MiB response peaks at the document plus one chunk plus about 4 KiB.
- [x] Read a body with a known length into one buffer of that length, as decision 4 says.
  `crates/nvs-stdlib/src/request.rs:3382`
  - [x] `RequestBody::declared_len`, which the server's `Pull` answers from `Content-Length`.
  - [x] A declared length past `request_body` is refused before a byte is read.
  - [x] A 64 KiB body read peaked at three times its size without the reservation, and once with it.
- [ ] Fix each cliff stage 1 added here, at its cause.
- [x] Encoding an `int` or a `float` allocates once per number (0.17 to 0.30 allocations per byte on
  the `ints` and `floats` shapes, where `ByteWriter::put` allocates nothing per write). Find the
  allocation in the walk and remove it. `crates/nvs-stdlib/src/json.rs:1239`
  `crates/nvs-stdlib/src/json.rs:815`
  - [x] The cause: `Stack::array`'s list test rendered every key with `key_at`.
  - [x] It reads `arr::is_list` now, and an object-shaped array's key reads `slot_key`.
  - [x] Encode allocations per byte are 0.0000 on every shape at 1 MiB and up (`deep` was 1.49).
- [ ] `written`'s buffer peaks at three times the document, the old and the doubled buffer side by
  side, and encode's time per byte grows 1.7 to 2.1 times from 64 KiB to 32 MiB on `wide`, `ascii`,
  `non-ascii` and `deep`. Write `Core\Json::encode` through the same chunk list as decision 3 and
  join it once into the `string`, so the peak is the document twice. `crates/nvs-stdlib/src/json.rs:936`
  `crates/nvs-runtime/src/budget.rs:1402` `crates/nvs-stdlib/src/json.rs:1808`
  - [x] `budget::ChunkWriter`: a first chunk that doubles to 64 KiB, then 64 KiB chunks, each asked.
  - [x] The encoder writes into it, and `Core\Json::encode` copies it once into its `string`.
  - [x] Encode peak per byte is 2.00 at 8 and 32 MiB, down from 3.00.
  - [ ] Encode time per byte still grows from 64 KiB to 32 MiB: `wide` 2.2, `non-ascii` 1.75 and
    `deep` 2.8 on one noisy run. Measure it on a quiet machine before you look for a cause.
- [ ] Decode's time per byte grows 2.05 times on `keys` while serde_json's grows 1.36 times: one
  object's key index is rebuilt as it doubles. Size it once or grow it by less.
  `crates/nvs-stdlib/src/json.rs:1939`
  - [x] Tried: the index bucket keeps each key's digest, so a growth reads no key. It showed no
    clear gain (10.5 against 11.6 ns/B at 8 MiB, 17.7 against 12.7 at 32 MiB, serde_json moving as
    much), and it cost 8 bytes per key, so it was reverted. `crates/nvs-runtime/src/array.rs:312`
  - [ ] Find where the time goes with a profiler before the next change.
- [ ] Decode's time per byte grows 1.75 times on `wide` and 2.46 times on `deep`, and serde_json's
  own parse grows 2.0 and 2.5 times on the same bytes. Measure whether the growth is the allocator's
  and the cache's: run the reference ladder with the values released at the end, not per rung. If
  serde_json's own parse is the cause, decision 5 sends it to its own goal, and the miss is
  recorded here with the figures. `crates/nvs-stdlib/src/json.rs:1913`
- [x] Write the three tests the `size-bounds` check names. Each measures the request's peak and
  allocation count at 64 KiB and at 8 MiB. `crates/nvs-stdlib/src/json.rs:1`
  - [x] `a_response_document_of_any_size_peaks_at_its_size_plus_one_chunk` and
    `allocations_per_document_byte_do_not_grow_with_the_document` in `json::tests`.
  - [x] `a_decoded_body_of_any_size_is_held_once` in `request::tests::json`, beside the body helper.
  - [x] `looper check --name size-bounds` is green.

### 3. the ladders are bounds: linear growth and a peak per document byte

**Does:** keeps a cliff from coming back.

- [ ] Give every ladder `// bench: complexity linear`, and record the ladders again on the fixed code.
  `benches/members/README.md:1`
- [ ] Write the peak per document byte of each member into the doc of its function, as its "What it
  spends" line. `crates/nvs-stdlib/src/json.rs:1758` `crates/nvs-stdlib/src/response.rs:1976`

### 4. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stage 2's memory half is done, and only its time items are open. `Core\Response::json` hands the
encoder's 64 KiB chunks to the body (`Ctx::write_body_chunks`), `Completion::output` carries them as
a `nvs_runtime::host::Written`, and the server sends one frame per chunk. A body that is not empty,
a capture or a child joins them and renders as before. A body with a `Content-Length` is read into
one buffer of that length. The three `size-bounds` tests exist, and the check is green.

The ladder's times move by up to 30% from run to run on this machine, so judge a time ratio over two
runs. Rerun it with `cargo test --profile proof --lib json::tests::serde_reference -- --ignored
--nocapture`. It takes about two minutes once built.

The sweep's `env-secret-proofs` red is stale, again: `looper check --name env-secret-proofs` finds no
such check, no log exists under `.looper/loops/main/checks/`, and no goal or floor names it. The
sweep's list lives in looper's service, outside the tree.

Tests that may go red at the gate, none of them run: a test that pins `size_of::<Ctx>()` or
`Completion`'s size (`Ctx` gained `body_chunks`, three words); a server test that matches
`Answer::Whole` or reads `Answer::bytes()` as a slice (it returns a `Cow` now); any test that reads
`completion.output` as a `Vec<u8>` outside the crates clippy builds.

### Next

**Stage 2, the time items.** Files: `crates/nvs-stdlib/src/json.rs`, `crates/nvs-runtime/src/array.rs`.

- [ ] `crates/nvs-stdlib/src/json.rs:7644` Encode's time per byte from 64 KiB to 32 MiB, over two
  quiet runs of the reference ladder (`wide` 2.2, `non-ascii` 1.75, `deep` 2.8 on one noisy run).
- [ ] Profile the `keys` decode growth before any change. `crates/nvs-stdlib/src/json.rs:1939`
  `crates/nvs-runtime/src/array.rs:312`
- [ ] `crates/nvs-stdlib/src/json.rs:1913` Run the ladder with the values released at the end, to
  see whether the `wide` and `deep` decode growth is the allocator's and the cache's. If serde_json's
  own parse is the cause, record the miss here with the figures (decision 5).
- [ ] `crates/nvs-stdlib/src/json.rs:1724` Then stage 3: `// bench: complexity linear` on every
  ladder, and each member's peak per document byte in its "What it spends" line.
  `crates/nvs-stdlib/src/response.rs:1976`

### Backlog
