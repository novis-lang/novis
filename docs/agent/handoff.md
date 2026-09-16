# Handoff

## State

**Goal `unowned-closures`, stage 4.** The decoding half of the hand-written codec is closed. A class
that wrote `fromJson` is decoded through that member at every door and at every level of a document:
`Contract::writes_decoder` (`crates/nvs-stdlib/src/json.rs:1493`) picks the door, `hand_written`
(`crates/nvs-stdlib/src/json.rs:1619`) is `nvs_runtime::call_static_on` with the value as it arrived,
and the three places that read an empty field list now take it first — `check_codec`'s `LogicError`,
`decode_nested`'s fatal and `decode_element`'s, the last two rewritten to name both spellings a
declaration has. `decode_object` asks before its object test, so § 6's `mixed` parameter means this
door reads a document the field walk would refuse, and an `array<T>` dispatches once per element.

**The written half wins wherever the class answers the member**, not only where the field list is
empty: `rule:core-classes/derive-generates-what-is-missing` is the derive filling in what the class
does not write, so a class carrying `#[Json\Derive]` as well decodes through its member while the
derived field list stays the encoder's. `nvs_stdlib::db::row` reads an empty mapping as its door
instead, and may keep doing so, because `E0757` refuses an attribute beside a written `fromRow`.
The lookup is `ClassDesc::method`'s flattened chain, which the member's doc states.

`nvs_stdlib::json`'s gap 1 is narrowed to the encoding half, which is the item below. Nothing is
blocked.

## Next group

**Stage 4: the hand-written codec, encoding half** — one file set: `crates/nvs-stdlib/src/json.rs`,
and the case above as the decode-side model.

- [ ] **A hand-written `toJson` is consulted** — `crates/nvs-stdlib/src/json.rs:192` gap 1,
      `rule:core-classes/derive-generates-what-is-missing`. The lookup is
      `Contract::writes_decoder`'s twin against `serialize_object`'s empty-codec refusal
      (`crates/nvs-stdlib/src/json.rs:860`), and the same precedence: a written member beats the
      derived field list. What it costs is not the lookup but the call — `Encodable`
      (`crates/nvs-stdlib/src/json.rs:554`) is a `Copy` `serde::Serialize` holding no `Ctx`, and
      `written` (`crates/nvs-stdlib/src/json.rs:1013`) turns every serde error into a `LogicError`,
      so a `toJson` that throws needs a `*mut Ctx` on the walk and a side channel to carry the
      `Fault::Pending` out past `S::Error` or the thrown class is lost.
- [ ] **Decide the re-entrancy before writing that call** — same anchors. `serialize_object` and
      `serialize_array` descend through a *borrowed* handle (`crate::arr::borrowed`,
      `crates/nvs-stdlib/src/json.rs:790`) that takes no reference, so compiled code running
      mid-walk can mutate or free the array the walk is inside. The decode side has no such
      question: `hand_written` is reached before any borrow is taken. Priority 1 says this is
      settled first and stated, not discovered.
- [ ] **A `#[Test]` result is a producer, so § 22's three output formats are one record rendered** —
      `crates/nvs-cli/src/runner.rs:418`, where the three fan out today. Unchanged for three groups,
      and the one item here that shares no file with the two above.

## Backlog

- `nvs_stdlib::json` gap 2 — the walk is a descriptor loop rather than emitted code; the gap's
  `Decided:` keeps the descriptor, so this is prose to settle, not a rewrite (`json.rs:207`).
- `nvs_stdlib::json` gap 3 — the encoder's real bound is the native stack, not `DEPTH_CEILING`
  (`json.rs:225`).
- Stage 5's `server_connection_bounds_are_read_from_the_server_block` and
  `two_builds_of_one_release_version_key_their_units_apart` are the goal's next red checks; both
  name tests that do not exist yet (`docs/agent/loop-goal.toml:11114`).
