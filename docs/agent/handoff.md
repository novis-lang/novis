# Handoff

## State

**Goal `unowned-closures`, stage 4.** The document door now asks both of its questions.
`rule:core-classes/derive-attribute`'s opt-in is answered at the call: a class carrying neither
`#[Json\Derive]` nor a written `fromJson` is refused while compiling — `check_json_participation`
(`crates/nvs-types/src/derive.rs:1097`), `E0821`, at every member that writes a document's class.
`check_json_sites` asks it of the root and the arity condition of the reachable set, and they are two
codes rather than the row door's one because the edits differ: `E0820` names a contract that is there
and a constructor position it leaves unfilled, `E0821` a class with no contract to read.
`nvs_stdlib::json`'s `check_codec` stays under both as the backstop for a descriptor built by hand,
and its four `decodeAs` engine faults are declared unreachable rather than echoed by a case.
`nvs-types`' gap list is now empty.

The conformance case split along the same seam. The decode half is
`tests/conformance/reject/a-class-that-declared-no-json-codec-is-refused-at-the-decode-site.nvst`;
what is left in `core/json-decode-as-reads-only-a-class-that-declared-a-codec.nvst` is the encode
door, which names no class in the call and so has no compile-time door and still answers when it
runs — the `LogicError` branch and a count over a three-class sweep.

**The carve-out made `nvs_stdlib::json`'s gap 1 reachable**, which is the next item: a class that
hand-writes `fromJson` compiles now and then meets `check_codec`'s refusal, because nothing
dispatches to it. That gap's text names both halves as of this session. Nothing is blocked.

## Next group

**Stage 4: the hand-written codec** — one file set: `crates/nvs-stdlib/src/json.rs`,
`crates/nvs-stdlib/src/db/row.rs` as the line-for-line model, and the cases the first item adds.

- [ ] **A hand-written `fromJson` is dispatched to** — `crates/nvs-stdlib/src/json.rs:176` gap 1,
      `rule:core-classes/derive-generates-what-is-missing`. `nvs_runtime::call_static_on` with
      `fromJson` where the contract's field list is empty, which is `nvs_stdlib::db::row`'s
      `hand_written` (`crates/nvs-stdlib/src/db/row.rs:255`) line for line, hung off `check_codec`'s
      own refusal (`crates/nvs-stdlib/src/json.rs:1493`) so that every door already calling it —
      `decode_as` (`crates/nvs-stdlib/src/json.rs:1286`), `Core\Arr::shapeAs`
      (`crates/nvs-stdlib/src/arr.rs:5973`) and the three `Core\Request` doors
      (`crates/nvs-stdlib/src/request.rs:2191`) — takes the second door at once. This closes the hole
      the compile-time carve-out opened, and needs none of the item below.
- [ ] **A hand-written `toJson` is consulted** — the other half of the same gap, and **priced as a
      lookup when it is not one**: `Encodable` (`crates/nvs-stdlib/src/json.rs:548`) is a
      `serde::Serialize` that is `Copy` and holds no `Ctx`, and `written`
      (`crates/nvs-stdlib/src/json.rs:1007`) turns every serde error into a `LogicError`. So a
      `toJson` that *throws* needs a `*mut Ctx` threaded through the walk and a side channel to carry
      the `Fault::Pending` out past `S::Error`, or the thrown class is lost. The gap's `Decided:`
      keeps the descriptor and widens it, so that is what to build; what it did not price is the
      fault path.
- [ ] **A `#[Test]` result is a producer, so § 22's three output formats are one record rendered** —
      `crates/nvs-cli/src/runner.rs:418`, where the three fan out today. Unchanged from the last two
      groups, and the one item here that shares no file with the two above.

## Backlog

- `nvs_stdlib::json` gap 2: the codec walk is a descriptor read by native Rust rather than emitted
  IR — that crate's module doc owns which of the two it stays.
- `nvs_types::defaults`' own gap: a written `= null` parameter default, the one unshipped row of
  `rule:core-api/required-optional-and-nullable`.
- Stage 5's acceptance check is the earliest red one —
  `server_connection_bounds_are_read_from_the_server_block`, an artefact not written yet rather than
  a regression.
