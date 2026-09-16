# Handoff

## State

**Goal `unowned-closures`, stage 4 open.** A derived field now carries its constructor parameter's
own default: `nvs_types::derive` reads the constant off the parameter with the evaluator that
reports nothing, `nvs_ir::lower::codec_fields` translates it into the runtime's smaller vocabulary,
and it arrives as `nvs_runtime::CodecField::default` beside `required`. Both doors that read a
derived codec fill an absent key from it — `Core\Json::decodeAs` at
`crates/nvs-stdlib/src/json.rs:1979` and `queryAs<T>`/`streamAs<T>` at
`crates/nvs-stdlib/src/db/row.rs:150`, where a column the result does not carry is no longer an
issue for a field that declares a default. `rule:core-api/required-optional-and-nullable` is
amended: three of its four rows ship. The fourth, `?T $x = null`, is unreachable for a reason
outside this knot — a written `= null` parameter default is refused while checking
(`crates/nvs-types/src/defaults.rs`, `ConstArg::Null`'s own doc) — so the rule's `status` stays
`designed`.

What is left of the knot is one thing, and a constant on a *field* cannot answer it: a constructor
position **no field names**, which a `#[Json\Field(skip: true)]` property that stayed a parameter
leaves behind. Both doors are loud there. Nothing is blocked.

## Next group

**Stage 4: what a decoder with no call site cannot reach** — one file set:
`crates/nvs-stdlib/src/json.rs`, `crates/nvs-stdlib/src/db/row.rs` with `db/mod.rs`, and
`crates/nvs-runtime/src/object.rs`, which the first item widens for both doors again.

- [ ] **A skipped property that stayed a constructor parameter has no carrier** —
      `crates/nvs-stdlib/src/json.rs:151` gap 1's residue and `crates/nvs-stdlib/src/db/mod.rs:268`
      gap 3, still one knot (`rule:core-classes/derive-field-list`). The unfilled position is
      refused at `crates/nvs-stdlib/src/json.rs:1845` and `crates/nvs-stdlib/src/db/row.rs:214`.
      What it wants is a carrier keyed on the *constructor position* rather than on the field, so
      the arity is what it is indexed by: `nvs_types::derive::DerivedCodec::ctor_arity`
      (`crates/nvs-types/src/derive.rs:336`), copied to `nvs_runtime::ClassDesc::ctor_arity`
      (`crates/nvs-runtime/src/object.rs:1363`) by `set_codec`/`set_db_codec`
      (`crates/nvs-runtime/src/object.rs:1753`), which `crates/nvs-codegen/src/lib.rs:1288` calls.
      The db door already refuses such a `queryAs<T>` while compiling —
      `check_row_sites`' arity condition, `crates/nvs-types/src/derive.rs:934` — and the json door
      has no call-site check at all, so decide whether the second carrier lands or whether the json
      door grows that refusal instead.
- [ ] **A hand-written `toJson()` is not consulted** — `crates/nvs-stdlib/src/json.rs:171` gap 2
      (`rule:core-classes/derive-generates-what-is-missing`). The lookup itself is shaped like
      `nvs_runtime::call_render` (`crates/nvs-runtime/src/dispatch.rs:533`) and the row door's twin
      is `hand_written` (`crates/nvs-stdlib/src/db/row.rs:254`), which reaches compiled code through
      `call_static_on`. **The obstacle is the walk, not the lookup**: the encode half is a `serde`
      `Serialize` impl — `serialize_object`, `crates/nvs-stdlib/src/json.rs:758` — and carries no
      `&mut Ctx` to call a compiled method through, while the row door's has one. `ENCODE` is the
      spelling, `crates/nvs-types/src/derive.rs:1440`.
- [ ] **A `#[Test]` result is a producer, so § 22's three output formats are one record rendered** —
      `crates/nvs-render/src/lib.rs:39` gap 1, which shares none of the files above; take it last or
      leave it to a session of its own.

## Backlog

- `rule:core-api/required-optional-and-nullable` flips to `status: shipped` the day a written
  `= null` parameter default is accepted — `docs/rules/core-api.json`.
- A written `= null` parameter default is refused, which is `crates/nvs-types/src/defaults.rs`'s
  own known gap and not this rule's.
- `crates/nvs-stdlib/src/json.rs` gaps 3 and 4 — the descriptor-versus-emitted-code question is
  answered (descriptor), and the encoder's stack bound stands.
- `docs/agent/loop-goal.toml`'s `[context.stage.4]` now names the three rules this stage cites; its
  `modules` still names no `crates/nvs-types/src/defaults.rs` or `crates/nvs-runtime/src/dispatch.rs`,
  which this session paid a fetch each for.
