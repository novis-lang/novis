# Handoff

## State

**Goal `unowned-closures`, stage 4.** `rule:core-classes/derive-field-list`'s skipped-parameter knot
is closed at both doors. A class whose derived codec fills fewer constructor parameters than the
constructor declares is now refused at the call that asks for an instance out of a document —
`nvs_types::derive::check_json_sites` (`crates/nvs-types/src/derive.rs:1064`), `E0820`, over the
written class and every deriving class its fields reach, since a document is a tree. The run-time
fatals stay under it as the backstop for a class built by hand, as the row door's already did.
`nvs_stdlib::json`'s gap 1 and `nvs_stdlib::db`'s gap 3 are struck (json's remaining gaps renumbered),
and `rule:core-api/required-optional-and-nullable` and `rule:core-classes/derive-field-list` are
amended to state the refusal. The second carrier keyed on the constructor position was the other
option and is **not** built: nothing is stored per object, and the two doors now answer alike.

**A live bug came out of the same knot.** Both deferred site recordings keyed on the method name,
which is not unique on `WRITTEN_CLASS_MEMBERS`, so `Core\Request::queryAs<SomeClass>` was refused with
the row door's `E0806` and `Core\Request::postAs<SomeClass>` escaped
`rule:security/derived-codec-qualifiers`. Both now key on the owner, which is what the shape branch
three lines below already did. Nothing is blocked.

## Next group

**Stage 4: the rest of the document door** — one file set: `crates/nvs-types/src/derive.rs`,
`crates/nvs-stdlib/src/json.rs`, and the one conformance case the first item rewrites.

- [ ] **The missing-`#[Json\Derive]` third moves to the call site** —
      `crates/nvs-types/src/derive.rs:44` gap 1, whose `Decided:` stands
      (`rule:core-classes/derive-attribute`). The condition goes in `check_json_sites`
      (`crates/nvs-types/src/derive.rs:1064`) beside the arity one, with the hand-written-decoder
      carve-out `check_row_sites` already has for `fromRow` (`crates/nvs-types/src/derive.rs:851`,
      the `DB_DECODE` branch) asked of `DECODE` (`crates/nvs-types/src/derive.rs:1596`) instead. It is
      **not** additive the way the arity condition was:
      `tests/conformance/core/json-decode-as-reads-only-a-class-that-declared-a-codec.nvst` asserts
      that the refusal is a `LogicError`, that encode and decode agree per class, and that the codec
      is read before the document — three run-time properties a compile-time refusal deletes rather
      than moves, so rewriting that case is part of the slice, not fallout from it.
- [ ] **A hand-written `toJson()` is not consulted** — `crates/nvs-stdlib/src/json.rs:171` gap 1,
      `rule:core-classes/derive-generates-what-is-missing`. A `ClassDesc::method("toJson")` lookup
      from the native encoder, whose `Decided:` sentence keeps the descriptor and widens it.
- [ ] **A `#[Test]` result is a producer, so § 22's three output formats are one record rendered** —
      `crates/nvs-cli/src/runner.rs:418`, where the three fan out today. Unchanged from the last
      group, and the one item here that shares no file with the two above.

## Backlog

- `crates/nvs-stdlib/src/json.rs:182` gap 2: both codec halves walk a field list rather than emitted
  straight-line code — the answer gap 1 above waits on.
- `crates/nvs-types/src/defaults.rs`: a written `= null` parameter default is refused while checking,
  which is the one unshipped row of `rule:core-api/required-optional-and-nullable`.
- `crates/nvs-stdlib/src/json.rs:195` gap 3: the encoder's real bound is the native stack.
- `Core\Arr::shapeAs` records a json site like every other document door; no case writes a class at
  it, only shapes (`tests/conformance/core/`).
