# Handoff

## State

**Goal `decided-closures`, stage 4 — the library — is under way.** The JSON encoder no longer
recurses: `Encodable::written` runs a loop over `Stack`, a `Vec` of `Frame`s that is at once the
descent, the ancestor chain a cycle is decided against and the path a message names a value with.
`crates/nvs-stdlib/src/json.rs`'s § *The walk carries its own stack* is the home of what that spends
per encode, and a `const` assertion beside `Frame` keeps the per-frame half of the figure true.

Every byte of a document is still `serde_json`'s: the structure goes through its `Formatter`, so the
`pretty` profile's indentation is byte-for-byte what `to_string_pretty` wrote, and each scalar goes
through a fresh serializer for its escaping and its number formatting. What the walk owns is the
order values are visited in. `Ancestor`, `Step` and the `Serialize` impl are gone; `Standing` retains
off the stack now, and `Stack`'s own `drop` releases what a `Cursor::Returned` holds when a refusal
stops the walk with frames still open.

`json.rs` gap 2 is struck — the ceiling is a catchable throw at every depth on any thread, not an
abort once the native stack runs out. Gap 1 is the module's only remaining one. Stage 4's first
acceptance check is down to a single missing name,
`an_xml_element_answers_its_namespace_uri`; checks 2 and 3 are untouched. `python
tools/owners.py --closes decided-closures` still names the goal's other gaps.

## Next group

**Stage 4: the JSON codec's last gap** — one file set: `crates/nvs-stdlib/src/json.rs`,
`crates/nvs-runtime/src/object.rs` and the rule fragment the sheet says to amend. The module doc
holds the `Decided:` sentence for all three.

- [ ] **`crates/nvs-runtime/src/object.rs:936` — `CodecField` carries the field's declared default.**
      The sheet's answer for gap 1 is "keep the descriptor and widen it", and the first half of that
      widening is a default constant per codec field, compiled in beside the wire key and the slot so
      the decoding walk materializes it rather than failing on an absent key
      (`rule:core-classes/derive-generates-what-is-missing`).
- [ ] **`crates/nvs-stdlib/src/json.rs:239` — read it, and delete gap 1's numbered item.**
      The derived decoder is `hydrate`'s field walk in the same file; the encoding half already reads
      `toJson` off the flattened method table, which is the `ClassDesc` lookup the sheet's other
      clause asks for, so state that rather than building it twice. The item and its
      `— owner: decided-closures` tag go when both halves are true.
- [ ] **`docs/rules/core-classes/derive-generates-what-is-missing.md:1` — amend the rule.**
      It still asks for IR emitted per derived class; what is built, and what the sheet decided to
      keep, is one compile-time descriptor per class read by native Rust. A rule whose answer changed
      is amended by its own process in the same slice and opens no record — goal
      `decided-closures`'s § *Standing decisions*.

## Backlog

- `crates/nvs-stdlib/src/xml.rs:133` — the computed `namespaceUri()` member, the last name stage 4's
  first acceptance check is missing.
- Stage 4 check 2: Zip64, a CRC, `Core\Random\Seeded`, a UUID's bytes, a UNC root, an EBML case —
  `docs/agent/loop-goal.toml:11605`.
- Stage 4 check 3: the regex step budget as a `limits` directive, and a literal CLDR pattern prepared
  at compile time — `docs/agent/loop-goal.toml:11635`.
