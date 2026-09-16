# Handoff

## State

**Goal `unowned-closures`, stage 4 open.** The codec descriptor's element description now nests:
`nvs_runtime::CodecElement` (`crates/nvs-runtime/src/object.rs:838`) is a chain, one link per level
of `array<…>`, so `array<array<T>>` and `array<{n: int}>` decode at every door that reads a derived
codec — `Core\Json::decodeAs`, `Core\Request::jsonAs` and `Core\Arr::shapeAs` alike. What does **not**
nest is the labels: a chain bottoms out in exactly one non-list node, so the terminal element's class,
enum roster and shape contract ride on `CodecField` as they already did, and `nvs-codegen` resolves
its two pointers unchanged. `json.rs`'s gap 1 is struck and its list renumbered 1–4;
`rule:core-classes/derive-field-list` is amended to say the *type* recurses and to name the row door
as the narrower one. The db door refuses a nested document at the declaration instead
(`nvs_types::derive::check_row_sites`), because a column is one value, with `db::row::hydrated` as the
by-hand backstop.

Nothing is blocked. The two items left in the group are both owned by retired goal `m8-stdlib-depth`
(`python tools/owners.py --check` reports them as its only `retired-owner: 2`).

**The pack was missing three `[context]` fields for this item.** `modules` names no
`crates/nvs-runtime/src/object.rs`, `crates/nvs-types/src/derive.rs`, `crates/nvs-ir/src/lower/mod.rs`
or `crates/nvs-stdlib/src/db/row.rs` — the four files the item's own descriptor decision had to be
made across — and `rules` names neither `core-classes/derive-field-list`, which the item's gap cites
by token, nor `core-classes/db-column-types`. Each cost a fetch.

## Next group

**Stage 4: what a decoder with no call site cannot reach** — one file set:
`crates/nvs-stdlib/src/json.rs`, `crates/nvs-stdlib/src/db/mod.rs` with `db/row.rs`, and
`crates/nvs-runtime/src/object.rs`, which the first item widens for both doors the way this session's
did. `crates/nvs-render/src/lib.rs` is the third item's own file and shares none of them.

- [ ] **A field's default has no call site to emit it from, at either door** —
      `crates/nvs-stdlib/src/json.rs:151` gap 1 and `crates/nvs-stdlib/src/db/mod.rs:268` gap 3, which
      are one knot (`rule:core-api/required-optional-and-nullable`). Decided already: carry the
      constant on `nvs_runtime::CodecField` (`crates/nvs-runtime/src/object.rs:906`) beside
      `required`, as `nvs_runtime::FieldDefault` already carries a property's own — the evaluation is
      `nvs_ir::lower::field_default` (`crates/nvs-ir/src/lower/mod.rs:343`) and `decode_field`
      (`crates/nvs-stdlib/src/json.rs:1958`) is the arm that today refuses an absent optional key.
- [ ] **A hand-written `toJson()` is not consulted** — `crates/nvs-stdlib/src/json.rs:170` gap 2
      (`rule:core-classes/derive-generates-what-is-missing`), a `ClassDesc::method` lookup from
      `Encodable`'s object arm at `crates/nvs-stdlib/src/json.rs:835`. Same descriptor decision as the
      item above, which is why they are one group.
- [ ] **A `#[Test]` result is a producer, so § 22's three output formats are one record rendered** —
      `crates/nvs-render/src/lib.rs:37` gap 1 (`rule:errors/diagnostic-record`).

## Backlog

- `nvs-render`'s gap 2 — a compiler diagnostic is not a producer either; `docs/decisions/0092.md:439`
  schedules it (`crates/nvs-render/src/lib.rs:37`).
- `json.rs` gap 3: both halves walk a field list rather than emitted code — the decision the two items
  above are the cheap half of (`docs/agent/carried-gaps.md` § *Unowned*).
- `json.rs` gap 4: the encoder's real bound is the native stack, not `DEPTH_CEILING`
  (`docs/agent/carried-gaps.md` § *Unowned*).
