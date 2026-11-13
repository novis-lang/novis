---
looper: 1
title: "Core\\Json reads and writes Novis values itself, and the binary carries no engine code nothing calls"
summary: "Reached when Core\\Json decodes and encodes without serde_json, without native recursion and without a stack switch, an integer literal outside int throws at any length, wasmtime builds with only the features the engine uses, and the built-in components are stored compressed and loaded before the first request."
context: [Cargo.toml, docs/rules/packaging/somebody-elses-specification-is-a-crate.md]
stages:
  - id: 1
    title: "a JSON reader with its own stack reads every token, and agrees with serde_json on every saved input"
    status: done
    context: [crates/nvs-stdlib/src/json.rs, crates/nvs-fuzz-checks/src/json.rs, crates/nvs-fuzz-checks/src/tests.rs, docs/rules/core-classes/json-whole-and-stream.md, docs/rules/http-server/engine-depth-is-bounded-per-decoder.md]
  - id: 2
    title: "decode, isValid and Request::json build values from the reader, with one allocation per key"
    status: done
    context: [crates/nvs-stdlib/src/json.rs, crates/nvs-stdlib/src/json/reader.rs, crates/nvs-stdlib/src/request.rs, tests/hostile/README.md]
  - id: 3
    title: "decodeAs and jsonAs fill classes from the reader on the task's own stack"
    status: done
    context: [crates/nvs-stdlib/src/json.rs, crates/nvs-stdlib/src/json/reader.rs, docs/rules/security/derived-codec-qualifiers.md]
  - id: 4
    title: "decodeEach reads through the same reader, and the spare stack leaves the tree"
    status: done
    context: [crates/nvs-stdlib/src/json/each.rs, crates/nvs-stdlib/src/json/reader.rs, crates/nvs-host/src/stack.rs, docs/rules/core-classes/json-whole-and-stream.md]
  - id: 5
    title: "the writer escapes each string in one pass, and Core\\Json names serde_json only in its tests"
    status: done
    context: [crates/nvs-stdlib/src/json.rs, crates/nvs-stdlib/src/json/pieces.rs, docs/rules/security/json-text-is-safe-inside-html.md, benches/members/README.md]
  - id: 6
    title: "wasmtime builds with only the features the engine uses"
    status: done
    context: [crates/nvs-ext/Cargo.toml, crates/nvs-ext/src/call.rs, crates/nvs-ext/src/load.rs, docs/rules/packaging/an-extension-is-a-sandboxed-wasm-component.md]
  - id: 7
    title: "the built-in components are stored compressed, and a served program's are loaded before its first request"
    status: done
    context: [crates/nvs-ext/build.rs, crates/nvs-ext/src/builtin.rs, crates/nvs-ext/src/load.rs, crates/nvs-cli/src/extensions.rs, docs/rules/packaging/the-first-party-components-are-built-in.md]
  - id: 8
    title: "the goal is verified once, and every failure is fixed"
    status: done
    gate: true
checks:
  - id: reader-agrees
    stage: 1
    name: "the reader accepts and refuses what RFC 8259 says, on a small stack, from one text or from chunks, and agrees with serde_json on every saved input"
    run: [cargo, test, --lib, --, the_reader_accepts_and_refuses_what_rfc_8259_says, the_reader_and_the_oracle_agree_on_every_saved_json_seed, the_reader_reads_the_deepest_document_on_a_small_stack, a_document_cut_at_any_byte_reads_the_same_from_chunks, a_lone_surrogate_and_invalid_utf8_are_refused]
    timeout: 1800
    tests:
      - the_reader_accepts_and_refuses_what_rfc_8259_says
      - the_reader_and_the_oracle_agree_on_every_saved_json_seed
      - the_reader_reads_the_deepest_document_on_a_small_stack
      - a_document_cut_at_any_byte_reads_the_same_from_chunks
      - a_lone_surrogate_and_invalid_utf8_are_refused
  - id: one-allocation-per-key
    stage: 2
    name: "an untyped decode allocates once per object key at two sizes, and an integer literal outside int throws at any length"
    run: [cargo, test, --lib, --, an_untyped_decode_allocates_once_per_object_key_at_two_sizes, an_integer_literal_outside_int_throws_at_any_length]
    timeout: 1800
    tests:
      - an_untyped_decode_allocates_once_per_object_key_at_two_sizes
      - an_integer_literal_outside_int_throws_at_any_length
  - id: typed-on-the-task-stack
    stage: 3
    name: "a typed decode reads its deepest document on a small stack, reports an oversized integer as its field's issue, and finds secret values in one read"
    run: [cargo, test, --lib, --, a_typed_decode_reads_its_deepest_document_on_a_small_stack, a_typed_integer_too_large_for_its_field_is_that_fields_issue, secret_spans_allocates_no_more_per_level_at_two_depths]
    timeout: 1800
    tests:
      - a_typed_decode_reads_its_deepest_document_on_a_small_stack
      - a_typed_integer_too_large_for_its_field_is_that_fields_issue
      - secret_spans_allocates_no_more_per_level_at_two_depths
  - id: each-on-the-reader
    stage: 4
    name: "decodeEach reads a token cut across two chunks, and nothing in the tree asks for a spare stack or an unbounded serde_json depth"
    run: [cargo, test, --lib, --, decode_each_reads_a_token_cut_across_two_chunks, nothing_in_the_tree_asks_for_a_spare_stack_or_an_unbounded_depth]
    timeout: 1800
    tests:
      - decode_each_reads_a_token_cut_across_two_chunks
      - nothing_in_the_tree_asks_for_a_spare_stack_or_an_unbounded_depth
  - id: writer-one-pass
    stage: 5
    name: "the writer writes every scalar as serde_json does, scans each string once at two sizes, and Core\\Json names serde_json only in its tests"
    run: [cargo, test, --lib, --, every_scalar_is_written_as_the_oracle_writes_it, a_string_is_scanned_once_per_encode_at_two_sizes, escape_unicode_writes_the_document_once, core_json_names_serde_json_only_in_its_tests]
    timeout: 1800
    tests:
      - every_scalar_is_written_as_the_oracle_writes_it
      - a_string_is_scanned_once_per_encode_at_two_sizes
      - escape_unicode_writes_the_document_once
      - core_json_names_serde_json_only_in_its_tests
  - id: wasmtime-features
    stage: 6
    name: "the wasmtime row names no feature the engine turns off or nothing calls, and a component in text form does not compile"
    run: [cargo, test, --test, wasmtime_features]
    timeout: 1800
    tests:
      - wasmtime_builds_with_no_feature_the_engine_turns_off
      - a_component_in_text_form_does_not_compile
  - id: components-stored-compressed
    stage: 7
    name: "every built-in component expands to the bytes its pin names, is stored at under half its size, and loads from the cache without expanding"
    run: [cargo, test, --test, builtin_stored]
    timeout: 1800
    tests:
      - every_built_in_component_expands_to_the_bytes_its_pin_names
      - every_built_in_component_is_stored_at_under_half_its_size
      - a_built_in_manifest_reads_without_expanding_its_component
      - a_cached_built_in_component_loads_without_expanding
  - id: components-loaded-at-start
    stage: 7
    name: "a served program that calls a built-in component finds it loaded before the first request"
    run: [cargo, test, --test, builtin_warm]
    timeout: 1800
    tests:
      - a_served_program_loads_the_built_in_components_it_calls_before_it_accepts
---

# Core\Json reads and writes Novis values itself, and the binary carries no engine code nothing calls

## Why

`Core\Json` is the one parser every JSON request body crosses. It uses `serde_json`'s tokenizer,
number parser and string escaper, and nothing else of the crate. A remote client chooses its input.

**Native recursion.** `serde_json` takes one native frame per nesting level. So its own limit is
switched off (`crates/nvs-stdlib/src/json.rs:2493`, `:3535`, `crates/nvs-stdlib/src/json/each.rs:493`).
Every text over 64 bytes is then parsed on an 8 MiB spare stack per thread
(`crates/nvs-stdlib/src/json.rs:2424`, `:2443-2449`, `crates/nvs-host/src/stack.rs:207-265`). A typed
decode runs constructors there, so it arms and restores the context's stack bounds on every way out
(`crates/nvs-stdlib/src/json.rs:3478-3519`). Two tests pin the crate's frame size on the debug
profile (`:5983`, `:6095`). The depth counter keeps this safe today. A crate update that fattens a
frame can break it.

**One extra allocation and copy per object key.** An untyped decode builds a `String` for each key,
then a Novis string from it (`crates/nvs-stdlib/src/json.rs:2379-2384`). The ledger records
`Core\Json::decode` at 22.05 allocations per operation (`data/perf/members.md:397`).

**A silent float.** An integer literal from `i64::MAX + 1` to `u64::MAX` throws. A longer one reaches
the visitor as `f64` and decodes as a `float` (`crates/nvs-stdlib/src/json.rs:81-91`, `:2331-2339`).
The module doc names silent precision loss as the bug that refusal exists for.

**Two or three scans per written string.** The crate's escape table reads each byte. `HtmlSafe` then
compares each byte again for the six HTML-safe escapes, which `93d8f777cf` added
(`crates/nvs-stdlib/src/json.rs:1148-1153`, `:1217-1250`). `{ escapeUnicode: true }` is a third pass
over the finished document into a second `String` of its size (`:2044-2064`).

**Re-reading.** `secret_spans` parses the document again at each nesting level through `RawValue`
(`crates/nvs-stdlib/src/json.rs:4037-4047`, `:4098-4111`).

**Engine code nothing calls.** The workspace `wasmtime` row enables every default feature except
`cache` (`Cargo.toml:376-402`). The engine then turns GC, threads, exceptions and the component
model's async off at run time (`crates/nvs-ext/src/call.rs:876-878`, `:900`, `:903`). The features
stay compiled into the sandbox engine, with the crates they bring.

**42.3 MB of plain wasm.** The four built-in components are embedded uncompressed
(`crates/nvs-ext/src/builtin.rs:19-46`). That is a third of the 129.5 MB binary. A component is also
compiled or read from the cache on the first call into it, inside a request
(`crates/nvs-cli/src/extensions.rs:164-194`, `crates/nvs-ext/src/load.rs:697-709`).

No request reaches another through any of this. The JSON half is the security priority and the
latency priority together. The engine half is simplicity and binary size.

## Standing decisions

**`Core\Json` gets its own reader and writer, and `serde_json` leaves its path.** The reader is one
module, `crates/nvs-stdlib/src/json/reader.rs`. It serves `decode`, `decodeAs`, `isValid`,
`decodeEach`, `Core\Request::json`, `jsonAs` and `secret_spans`. **What it spends:** about 750 owned
lines of parser for bytes an attacker sends. It buys a depth that is a loop bound, one allocation per
key and one pass per written string. The security priority and the latency priority both favour it,
and simplicity pays. `rule:packaging/somebody-elses-specification-is-a-crate` gains this as its second
argued exception. Declined: SIMD scanning, a pull-reader crate, and `serde_json`'s
`arbitrary_precision` feature.

**`serde_json` stays everywhere else, and stays the oracle.** It remains a dependency of `nvs-stdlib`
for small documents of a fixed shape: JOSE headers and claims checks (`jwt.rs`, `jwe.rs`), JWK import
(`crypto.rs:2194`), WebAuthn client data (`passkey.rs`), push subscriptions (`webpush.rs:415`) and
queue rows (`queue.rs`). It stays in `nvs-runtime` (`spool.rs`), `nvs-render` (`json.rs`) and
`nvs-types` (`ext_lib.rs`). It stays in `nvs-ext` for the manifest and source sections, and in
`nvs-lsp` for the wire. It stays in `nvs-cli` for packages, the debug service, DAP, the agent
commands, schemas, OpenAPI, meta and telemetry. `wasmtime`, `lsp-types` and `mysql_common` link it
too. Each of those keeps the crate's own depth limit of 128. Inside
`crates/nvs-stdlib/src/json.rs` and `json/` the crate is named only in test modules, as the oracle of
the differential tests. The `unbounded_depth` feature leaves the workspace row. `raw_value` stays for
the LSP and the debug API, and `float_roundtrip` stays so the oracle rounds as the reader does.

**The reader is a pull tokenizer with its own stack, and every consumer is a loop.** It gives one
event per call: a scalar, a key, a container's start or its end. It checks the grammar itself against
a bit per open level, 128 bytes for `DEPTH_CEILING` levels, held inline. It never recurses, so no
caller switches stacks. `DEFAULT_MAX_DEPTH` stays 512 and `DEPTH_CEILING` stays 1024
(`rule:http-server/engine-depth-is-bounded-per-decoder`). Each consumer keeps its open containers in
a `Vec` whose growth is asked from the request's budget first, as every value builder asks
(`budget::affords`, `crates/nvs-stdlib/src/json.rs:2401`). **What it spends:** one frame per open
level on the heap per decode, freed with the call. Safe default: a consumer that cannot become a
loop in its stage stays on `serde_json` until stage 4. The stage 4 test holds the end state.

**The reader reads from one text or from chunks, through one source trait.** A whole document is a
slice. `decodeEach` keeps its coroutine, and its source suspends when a chunk ends. A token may cross
two chunks, so the coroutine is what keeps the reader's state between them. **What it spends:** the
coroutine's stack per walk in flight, as today, now reserved at 256 KiB in place of 8 MiB. Safe
default: 1 MiB if the debug profile needs it. Declined: a reader that resumes without a coroutine. It
needs a second, re-entrant form of every token loop.

**What a program observes does not change, except the integer defect.** Every conformance case,
example, attack and fuzz seed for JSON passes unchanged. The reader keeps `serde_json`'s message text
and its line and column for every syntax error, because `.out` files pin them
(`docs/examples/core/Json/decode/02-text-that-is-not-valid-json.out:3`). It keeps what `-0`, an
exponent and a fraction decode to. Floats parse through `str::parse::<f64>` on the checked slice,
which rounds as `float_roundtrip` does. The differential test is the proof.

**An integer literal outside `int` throws at any length.** This covers a literal with no fraction
and no exponent. It throws the `ParseError` that a literal just past `i64::MAX` throws today. A
negative one says "too small". The message shows at most the first 32 digits, then `...`, so a long literal
cannot fill a log line. A typed decode keeps a `float` field's result, since the program named that
type. A `uint` field takes a literal up to `u64::MAX`. Any other field reports it as that field's
issue. This is loud where the present result is silent. **What it spends:** nothing per number. The
reader counts digits as it reads them.

**Security decides the reader's shape.** The module has no `unsafe` block. Every index is a checked
slice or a `get`. A string with escapes is built in a scratch buffer whose growth is asked from the
budget first. Four inputs are refused with the messages the crate gives: a lone surrogate, an escape
past the text's end, a control byte in a string, and invalid UTF-8 from a byte source
(`rule:types/string-is-utf8`). The hostile suite's JSON
attacks pass unchanged. A new fuzz target replays the saved seeds in `cargo test`, and no fuzz search
gates anything (`rule:testing/a-fuzz-search-gates-nothing`).

**The writer has one escape table and one pass.** The table holds JSON's own escapes and the six of
`rule:security/json-text-is-safe-inside-html`. `{ escapeUnicode: true }` is the same pass with one
more class of byte, so the second `String` goes. Floats are written by the formatter `serde_json`
calls, `zmij`, taken as a direct dependency that is already linked. Integers use the runtime's
decimal writer. The encoder's own stack of frames stays as the reached goal `json-one-pass` left it.
Every byte written is the byte written today. **What it spends:** nothing new, and one compare fewer
per string byte.

**Count guards hold the gain.** Allocations per decoded object key are counted with
`nvs_runtime::budget::allocations` at 1,000 and 16,000 keys. String scans per encode are a debug-build
count the writer bumps, as `crates/nvs-host/src/counts.rs` counts reads, at two sizes. No guard reads
a clock.

**`wasmtime` builds with the features the engine uses, and no other.** Kept: `runtime`, `cranelift`,
`component-model`, `async`, `pooling-allocator`, `std`, and `parallel-compilation`, which keeps a
cold compile of a built-in component on every core. `anyhow` and `wit-parser` stay only if the build
fails without them. `addr2line`, `backtrace` and `demangle` stay only if a named extension test fails
without them. Removed: `gc`, `gc-drc`, `gc-copying`, `gc-null`, `threads`, `stack-switching`,
`component-model-async`, `coredump`, `debug`, `debug-builtins`, `profiling`, `wat` and
`compile-time-builtins`. **What it spends:** nothing in any priority. It removes dead code from the
component whose job is isolation. Safe default: a feature whose removal fails the build or a named
test stays, with its reason in the row's comment.

**The built-in components are stored with Brotli, and expanded only where one is compiled.** Brotli
is the codec in the tree with an encoder and the best ratio. The build script compresses at quality
9, so a clean build pays seconds, not minutes. The pin stays the SHA-256 of the expanded `.nvsx`, so
`env_hash` and every cache key are unchanged. Each component's manifest and source sections are also
embedded plain, so a compile reads them without expanding. A cache hit never expands. A miss expands
into one buffer, checks it against the pin, compiles, stores and frees the buffer. **What it
spends:** on a miss, tens of milliseconds and the expanded size of that one component, at most about
16 MB. The compile beside it takes seconds and more memory than that. The idle server holds no
expanded bytes. The binary loses the 42.3 MB of plain wasm and gains the stored size, which the
session records in the rule. Declined: deflate, which stores more. `ruzstd`, which has no strong
encoder. Components compiled at build time, which are larger than the wasm and fixed to one CPU. A
buffer kept for the process, which adds the expanded size to idle memory.

**`nvs serve` loads the built-in components its program calls before it accepts.** The checker types
every `Novis\` call against a built-in manifest, so the set is known when the compile ends
(`crates/nvs-types/src/ext_lib.rs:208`). **What it spends at start:** nothing for a program that
calls none. Otherwise one cache read per called component, or one expand and compile on a cold cache,
once per binary. That work leaves the first request. A file compiled after start, in development,
still loads on its first call as today. Safe default: if the set needs a new pass over the program,
no start-time load is built. The rule then says the first call pays it. **How the set is known:** the checker's types are gone by the time a unit is compiled, so the
set is read off the IR. `nvs_codegen::Unit::extension_classes` is the sorted classes of the
program's `ExtensionCall`s, read by one scan of the instructions on a compile and on a cache load
alike, at one compare per instruction off the request path. The emitter could collect them with no
scan, but a unit from the artifact cache has no emitter, and one fact gets one mechanism. The
server prints one `loaded the built-in component` line per component, before its `listening on`
lines.

**These crates stay, and no session reopens them.** `hyper`: it parses attacker bytes, and owning
HTTP/1 framing spends security to buy simplicity. `gix`: one shallow fetch, off the request path, over
a connection Novis checks itself. `ring`, the RustCrypto crates, `argon2` and `bcrypt`: primitives
nobody should write. `rustls` and `x509-parser`: the TLS protocol and its certificates. Cranelift: the
compiler backend. `rusqlite`: SQLite is its C source. The three regex crates: engines with their own
guarantees. `flate2`, `brotli` and `ruzstd`: codecs with external specifications. The Unicode table
crates and `encoding_rs`: data the Unicode releases own. The `mysql_common` row decode belongs to goal
`database-doors`, and the `hyper` header changes to goal `request-path-per-core`.

**Lanes:** two lanes. Stages 1 to 5 share `crates/nvs-stdlib/src/json.rs` and run in order. Stages 6
and 7 share `crates/nvs-ext/Cargo.toml` and `crates/nvs-ext/src/load.rs` and run in order. The two
lanes share only the workspace `Cargo.toml`, in different rows, and may run side by side.

## Stages

### 1. a JSON reader with its own stack reads every token, and agrees with serde_json on every saved input

**Does:** add the reader as a new module that nothing calls yet, with its tests, its fuzz target and
the rule changes that allow it.

- [x] `rule:packaging/somebody-elses-specification-is-a-crate` argues the exception: what the owned
  reader costs and buys, as the first standing decision says
  (`docs/rules/packaging/somebody-elses-specification-is-a-crate.md:13`). Then `bun nv rules --check`
  names any stale chapter.
- [x] `rule:core-classes/json-whole-and-stream` says the reader is Novis's own, with its own stack,
  inside the same coroutine. Its two Declined entries that cite "RFC 8259 being a dependency" get the
  present reason (`docs/rules/core-classes/json-whole-and-stream.md:55`, `:84-91`).
- [x] `crates/nvs-stdlib/src/json/reader.rs:1`: the source trait with a slice source, the events, the
  grammar bits for `DEPTH_CEILING` levels, strings with escapes and surrogate pairs, the RFC 8259
  number grammar with a digit count, and the error type with `serde_json`'s message text, line and
  column (`rule:http-server/engine-depth-is-bounded-per-decoder`,
  `crates/nvs-stdlib/src/json.rs:5881`).
- [x] Test `the_reader_accepts_and_refuses_what_rfc_8259_says`: a table of accepted and refused
  texts, each refused one with its message and position (`crates/nvs-stdlib/src/json/reader.rs:1`).
- [x] Test `the_reader_and_the_oracle_agree_on_every_saved_json_seed`: for every file under
  `fuzz/seeds/json/`, the same verdict as `serde_json`, the same error text on a syntax error, and
  equal scalars in order. The oracle's depth limit of 128 and the reader's own refusals are named
  exceptions (`crates/nvs-stdlib/src/json/reader.rs:1`, `rule:testing/a-fuzz-search-gates-nothing`).
- [x] Tests `the_reader_reads_the_deepest_document_on_a_small_stack`, with the helper
  `decoded_on_a_thread` uses (`crates/nvs-stdlib/src/json.rs:5976`),
  `a_document_cut_at_any_byte_reads_the_same_from_chunks` and
  `a_lone_surrogate_and_invalid_utf8_are_refused` (`crates/nvs-stdlib/src/json/reader.rs:1`).
- [x] A fuzz target `json_reader` and its check, which runs the oracle comparison on any bytes. Its
  row in `TARGETS` reads the `json` seeds (`fuzz/fuzz_targets/json_reader.rs:1`,
  `crates/nvs-fuzz-checks/src/json_reader.rs:1`, `crates/nvs-fuzz-checks/src/tests.rs:10`,
  `crates/nvs-fuzz-checks/src/lib.rs:1`). `nvs-fuzz-checks` gains `serde_json`, and
  `nvs_stdlib::fuzz` exports the reader's entry.

### 2. decode, isValid and Request::json build values from the reader, with one allocation per key

**Does:** make the untyped decode a loop over the reader's events, on the caller's own stack, and
close the integer defect.

- [x] `Decode` becomes a loop with a `Vec` of open arrays. A key is built once, as the Novis string
  the array stores (`crates/nvs-stdlib/src/json.rs:2272-2392`, `:2379-2384`,
  `rule:core-classes/json-whole-and-stream`).
- [x] `read`, `read_marked` and `parse` call it in place, and `read_with` goes with the test at
  `:5983`. `IN_PLACE_BYTES` stays for stage 3's two sites. `read` returns the reader's error type
  (`crates/nvs-stdlib/src/json.rs:2415-2449`, `:2484-2514`).
- [x] The callers of `read` take the new error type: `crates/nvs-stdlib/src/test.rs:3411`,
  `crates/nvs-stdlib/src/queue.rs:3466`, `crates/nvs-stdlib/src/request.rs:3182`, and
  `nvs_core_json_decode` and `nvs_core_json_is_valid` (`crates/nvs-stdlib/src/json.rs:2529`).
- [x] The integer decision: a literal outside `int` throws at any length, with the capped digits
  (`crates/nvs-stdlib/src/json.rs:2324-2339`). The module doc's refusal entry says so (`:81-91`).
- [x] Tests `an_untyped_decode_allocates_once_per_object_key_at_two_sizes` and
  `an_integer_literal_outside_int_throws_at_any_length` (`crates/nvs-stdlib/src/json.rs:8298`).
- [x] An attack: a body holds an integer with more digits than any number type, as a value and
  inside a list. Each decode throws, and `isValid` returns `false`
  (`tests/hostile/core/Json/decode/05-an-integer-longer-than-any-number-type.nvs:1`,
  `tests/hostile/core/Json/isValid/01-deep-huge-and-broken-texts.nvs:27`).
- [x] A conformance case for the same literal, positive and negative
  (`tests/conformance/core/json-decode-refuses-an-integer-outside-int-at-any-length.nvst:1`).
- [x] `bun nv proofs --comments` over the new `.nvs` files passes, and the `decode` and `isValid`
  pages still say what happens (`docs/examples/core/Json/decode/about.md:9`,
  `docs/examples/core/Json/isValid/about.md:5`).

### 3. decodeAs and jsonAs fill classes from the reader on the task's own stack

**Does:** rewrite the typed decode's visitors as loops over the reader, so a constructor runs on the
stack the call came in on. Read `secret` spans in one pass.

- [x] `Fill`, `FillList`, `Typed`, `Key` and `Found` read the reader's events. A declared key is
  matched from the borrowed text, as today (`crates/nvs-stdlib/src/json.rs:2723-3077`, `:3251-3459`,
  `rule:security/derived-codec-qualifiers`).
- [x] `read_against` and `parse_against` run in place. `TaskBounds` and `IN_PLACE_BYTES` go, with
  the test at `:6095` (`crates/nvs-stdlib/src/json.rs:2424`, `:3467-3559`).
  - [x] `read_against` runs in place, and `parse_against`, `TaskBounds` and the short-text test go.
  - [x] `IN_PLACE_BYTES` goes once `secret_spans` reads through the reader.
- [x] A typed integer outside its field's type follows the integer decision: `float` converts,
  `uint` takes up to `u64::MAX`, and any other field reports its own issue
  (`crates/nvs-stdlib/src/json.rs:5433`).
- [x] `secret_spans`, `secret_values` and `Entries` read byte ranges from the reader's events in one
  pass (`crates/nvs-stdlib/src/json.rs:4024-4050`, `:4098-4146`, `:4215-4239`).
- [x] `decode_as`'s callers take the new error type: `crates/nvs-stdlib/src/jwt.rs:2177`,
  `crates/nvs-stdlib/src/test.rs:3472`, `crates/nvs-stdlib/src/request.rs:3313`. `decode_as` still
  returns `Fault`, so no caller changed.
- [x] Tests `a_typed_decode_reads_its_deepest_document_on_a_small_stack`,
  `a_typed_integer_too_large_for_its_field_is_that_fields_issue` and
  `secret_spans_allocates_no_more_per_level_at_two_depths` (`crates/nvs-stdlib/src/json.rs:6095`).
  - [x] `a_typed_decode_reads_its_deepest_document_on_a_small_stack` passes.
  - [x] `a_typed_integer_too_large_for_its_field_is_that_fields_issue` passes.
  - [x] `secret_spans_allocates_no_more_per_level_at_two_depths` passes.

### 4. decodeEach reads through the same reader, and the spare stack leaves the tree

**Does:** put the reader behind `decodeEach`'s chunk feed, then delete what only the recursion
needed.

- [x] `Feed` becomes the reader's chunked source. `Toward`, `Skip` and `each_line` become loops over
  events, and a skipped string or key is never copied (`crates/nvs-stdlib/src/json/each.rs:135`,
  `:473-545`, `rule:core-classes/json-whole-and-stream`).
- [x] `crates/nvs-stdlib/src/json/each_probe.rs:1` is deleted with its `mod` line
  (`crates/nvs-stdlib/src/json.rs:5879`) and the `corosensei` dev-dependency it alone needs
  (`crates/nvs-stdlib/Cargo.toml:164`). A test in it that pins what a program observes moves to
  `each.rs`.
- [x] `on_spare_stack`, `on_spare_stack_within`, the `SPARE` slot and their tests go.
  `suspendable` reserves its own 256 KiB constant (`crates/nvs-host/src/stack.rs:175-265`, `:290`,
  `:1010-1050`, `crates/nvs-host/src/lib.rs:174`).
- [x] The workspace row drops `unbounded_depth`, and its comment says what `serde_json` is for now
  (`Cargo.toml:651-672`).
- [x] `rule:core-classes/json-whole-and-stream` states the walk's memory with the smaller stack
  (`docs/rules/core-classes/json-whole-and-stream.md:73`).
- [x] Test `decode_each_reads_a_token_cut_across_two_chunks`: a string, a number, a surrogate pair
  and a keyword, each cut at every byte (`crates/nvs-stdlib/src/json/each.rs:1`).
- [x] Test `nothing_in_the_tree_asks_for_a_spare_stack_or_an_unbounded_depth`: it reads every `.rs`
  file under `crates/` and the workspace `Cargo.toml`, and fails naming the file
  (`crates/nvs-stdlib/src/json.rs:5890`).

### 5. the writer escapes each string in one pass, and Core\Json names serde_json only in its tests

**Does:** replace the crate's serializer and formatter under `Sink` with one writer, then close the
JSON half with a test over the source.

**Split:** an `Explore` subagent lists every `serde_json::` call outside a test module under
`crates/nvs-stdlib/src` and `crates/nvs-runtime/src`, as `file.rs:NN` and what document it reads.
The session checks the list against the second standing decision.

- [x] `Sink` writes structure and scalars itself. One escape function covers JSON's escapes, the six
  HTML-safe ones and, when asked, every non-ASCII character. `HtmlSafe` and `escape_non_ascii` go
  (`crates/nvs-stdlib/src/json.rs:1119-1240`, `:2040-2060`, `:2136-2227`,
  `rule:security/json-text-is-safe-inside-html`).
- [x] The pretty profile and `encodeEach`'s pieces use the same writer
  (`crates/nvs-stdlib/src/json/pieces.rs:1`).
- [x] `zmij` becomes a direct dependency of `nvs-stdlib` at the version the lock file holds
  (`crates/nvs-stdlib/Cargo.toml:105`, `Cargo.toml:671`).
- [x] `rule:security/json-text-is-safe-inside-html` says the six are rows of the one table, and that
  `escapeUnicode` is the same pass (`docs/rules/security/json-text-is-safe-inside-html.md:19`,
  `:46`).
- [x] The module doc is rewritten as a whole: the reader, the writer, the refusals and what each
  spends. The line about a faster parser goes (`crates/nvs-stdlib/src/json.rs:1-56`, `:358-360`).
- [x] Tests `every_scalar_is_written_as_the_oracle_writes_it`, over every scalar in the saved seeds
  and a table of float edges, `a_string_is_scanned_once_per_encode_at_two_sizes` and
  `escape_unicode_writes_the_document_once` (`crates/nvs-stdlib/src/json.rs:8723`).
- [x] Test `core_json_names_serde_json_only_in_its_tests`: it reads `json.rs` and every file under
  `json/`, and fails naming a line outside a test module (`crates/nvs-stdlib/src/json.rs:8723`).
- [x] The six `Core\Json` members are measured again, as `benches/members/README.md:1` says, and
  each record is at or below its last one (`data/perf/members.md:397`,
  `rule:testing/member-perf-ledger`).

### 6. wasmtime builds with only the features the engine uses

**Does:** cut the workspace `wasmtime` row to the kept set, and load components from binary only.

- [x] A search over `crates/nvs-ext/src` and `crates/nvs-cli/src` confirms that nothing calls a
  core-dump, profiling or guest-debug API of `wasmtime` (`crates/nvs-ext/src/call.rs:899`). A feature
  with a caller stays, with the caller named in the row's comment.
- [x] The row holds the kept set of the standing decision, and its comment gives each kept feature
  one reason (`Cargo.toml:366-402`,
  `rule:packaging/an-extension-is-a-sandboxed-wasm-component`). `crates/nvs-ext/Cargo.toml:19-33`
  drops the sentence about wasmtime's defaults.
- [x] `Component::new` becomes `Component::from_binary` (`crates/nvs-ext/src/load.rs:353`, `:362`).
- [x] `cargo build`, then `cargo test --test proposals --test failure --test call --test cache`. A
  feature whose removal fails either is put back, with its reason in the comment
  (`crates/nvs-ext/tests/proposals.rs:50`).
- [x] Tests `wasmtime_builds_with_no_feature_the_engine_turns_off`, which reads the row and
  `PROPOSALS` and fails naming the feature, and `a_component_in_text_form_does_not_compile`
  (`crates/nvs-ext/tests/wasmtime_features.rs:1`, `crates/nvs-ext/src/call.rs:876`).

### 7. the built-in components are stored compressed, and a served program's are loaded before its first request

**Does:** embed each component as Brotli beside its plain sections, expand only on a compile, and
load a served program's components at start.

**Split:** an `Explore` subagent lists, as `file.rs:NN`, where `nvs serve` finishes compiling its
entry and where a checked program's extension classes are known.

- [x] `build.rs` writes each component as `<name>.nvsx.br`, its manifest and source sections plain,
  and the pin of the expanded bytes. `brotli` joins the build dependencies and the dependencies
  (`crates/nvs-ext/build.rs:173-184`, `crates/nvs-ext/Cargo.toml:53`,
  `rule:packaging/the-first-party-components-are-built-in`).
- [x] `builtin.rs` exposes the stored bytes, the plain sections and one `expanded(name)` that checks
  the pin, with a debug-build count of expansions (`crates/nvs-ext/src/builtin.rs:19-53`).
- [x] `BUILTINS`, `builtin_manifests`, `Loader::builtins` and the site at `:532` read the plain
  sections (`crates/nvs-ext/src/load.rs:449-466`, `:532`, `:734-769`).
- [x] `Loader::compile` takes the bytes as a closure, so a cache hit never expands, and
  `Builtin::extension` passes `expanded` (`crates/nvs-ext/src/load.rs:351-366`, `:697-709`).
- [x] The tests that read `IMAGE`, `INTL`, `PDF` or `SPREADSHEET` call `expanded`
  (`crates/nvs-ext/tests/builtin.rs:15`, `crates/nvs-ext/tests/intl.rs:17`,
  `crates/nvs-ext/tests/pdf.rs:21`).
- [x] The four tests of `components-stored-compressed`, each over `Loader::builtins`
  (`crates/nvs-ext/tests/builtin_stored.rs:1`).
- [x] `nvs serve` loads each built-in component its program calls before it accepts, outside any
  request's budget (`crates/nvs-cli/src/extensions.rs:164-194`, `crates/nvs-cli/src/main.rs:1213`,
  `crates/nvs-types/src/ext_lib.rs:208`).
- [x] Test `a_served_program_loads_the_built_in_components_it_calls_before_it_accepts`
  (`crates/nvs-cli/tests/builtin_warm.rs:1`).
- [x] The rule states the stored form, the measured sizes, what a miss and a start spend, and the
  Declined list of the standing decision
  (`docs/rules/packaging/the-first-party-components-are-built-in.md:13`, `:24`, `:62`).

### 8. the goal is verified once, and every failure is fixed

- [x] Run `looper verify` over the whole goal.
- [x] Fix every failure it names in one pass, never deleting or loosening a test.
- [x] Run `looper verify` once more, and it is green.

## Handoff

### State

Every stage is done, and the goal is reached. `looper verify` is green over the whole goal, and
`bun nv verify --doc`, `bun nv owners --closes fitted-dependencies` and `bun nv ci --check` are
green.

The gate fixed six things. `website/src/data/numbers.json`, the rule chapters and
`THIRD-PARTY-LICENSES.txt` were rendered again. `json_as_refuses_each_bad_body_with_the_message_the_array_path_gave`
in `crates/nvs-stdlib/src/request.rs` and two conformance cases under `tests/conformance/core/`
now pin the decided integer behaviour: an untyped decode throws at any length, and a typed `uint`
field reads up to `u64::MAX` and reports a larger literal as its own issue. The
`Core\Json::decode` and `isValid` pages' search questions were rewritten and stamped. The
Postgres test container now runs under an init process (`tests/db/compose.yaml`): an orphaned
`pg_isready` from a timed-out healthcheck made the postmaster at PID 1 restart into crash recovery,
and two `nvs-cli` runner tests failed with SQLSTATE 57P03. The integer decision now lives in
`rule:errors/ambiguous-input-refused`.

### Next

- `docs/goals/README.md:3` the driver retires this goal, and `server-events` is next on the chain.

### Backlog

- `Core\Json::isValid` checks a text without building its value.
- A reader that resumes between chunks would let `decodeEach` drop its coroutine.
