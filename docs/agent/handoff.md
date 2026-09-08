# Handoff

## State

**Goal 18, stages 1 and 2 are complete; stage 3 has landed the fork, the carrier's runtime end, the
checker's recording half and now the emitting half.** Optional shape fields, `tainted {…}`, guarded
reads, the codec tables' `required` column, `nvs_types::derive::shape_codec`, `nvs_runtime::ShapeCodec`
and its `ClassTable` owner all stand — `rule:types/shape-type`'s optional-field paragraph,
`rule:core-api/required-optional-and-nullable` and [ADR 0157](../decisions/0157.md) § 2 own that half,
and `crates/nvs-ir/src/lib.rs` § *Design choices* owns the fork. No rule changed, so ADR 0159 is still
unopened.

**What landed this session: the carrier is emitted, published and relocated, and nothing ICEs.** A
`WRITTEN_CLASS_MEMBERS` call now carries **three** leading constants, not two — descriptor, list flag,
wire contract — and `nvs_stdlib::registry::WRITTEN_CLASS_MEMBERS`' own docs are that ABI's home. The
third is `ir::InstKind::ShapeCodecConst`, naming an entry of `ir::Program::shape_codecs` by the key
`lower::shape_codec_key` renders from the contract itself, so `{n: int}` and `{n: string}` share one
`$shape{n}` class and get one table each. `nvs-codegen` defines them in `link_codecs`' pass and
publishes `shape_codec_symbol`'s name beside every descriptor's; a written class emits the zero word.
`Lowering::written_type_constants` is the one emitter for all three call paths and registers the shape's
class itself, so a unit that hydrates a shape it never spells still resolves its descriptor.

**The next gap is the decoder, and it is a throw rather than an ICE.**
`Core\Json::decodeAs<{n: int}>("…")` compiles and runs today, and answers
`LogicError: `$shape{n}` has no JSON codec` — `json::check_codec`
(`crates/nvs-stdlib/src/json.rs:1025`) asks the descriptor, which is the one thing a shape's descriptor
cannot answer, and slot 2 is not read by any member yet. The playbook bullet at
`crates/nvs-stdlib/src/arr.rs:shapeAs` now says exactly that.

The failing acceptance check (`examples/input-shapes.nvs`) is stage 5's fixture — an unwritten artefact,
not a regression.

## Next group

**Stage 3: the contract is read, and then a member writes shapes** — one file set:
`crates/nvs-stdlib/src/json.rs`, `crates/nvs-stdlib/src/arr.rs`, `crates/nvs-stdlib/src/registry.rs`.
**In this order** — the coverage gate fails the moment a registry row has no conformance case, so the
member is last, and it has nothing to call until the walk exists.

- [ ] **Hydrate against the contract instead of the descriptor** — `nvs_core_json_decode_as`
      (`crates/nvs-stdlib/src/json.rs:933`) reads slot 2 with `Value::as_shape_codec`, and where it is
      `Some` the walk runs against those fields rather than `desc.codec()`: `check_codec`
      (`crates/nvs-stdlib/src/json.rs:1025`) refuses `$shape{n}` today because a shape's descriptor
      carries no codec, and `hydrate`/`decode_fields`
      (`crates/nvs-stdlib/src/json.rs:1082` and `:1279`) build through `desc.ctor_arity()` and a
      constructor, which a synthesized shape class has neither of — a shape's fields are written slot
      by slot, and `nvs_runtime::ShapeCodec`'s `CodecField::param` **is** its slot
      (`crates/nvs-runtime/src/object.rs:1335`). `rule:core-classes/derive-reports-every-field`'s
      collected report is the failure shape, unchanged; loop-goal.md § *Stage 3* items 2 to 4 are the
      specification.
- [ ] **`Core\Arr::shapeAs<T>` — the row, the card, the roster entry and three cases in one slice** —
      `crates/nvs-stdlib/src/arr.rs:92` is the class, and the five edits are conventions.md's; the
      member calls the walk above rather than keeping one, which is loop-goal.md § *Stage 3* item 2.
      `nvs_stdlib::registry::WRITTEN_CLASS_MEMBERS` (`crates/nvs-stdlib/src/registry.rs:2382`) gains its
      row, and the ABI it joins is that constant's own docs.
- [ ] **`E_TYPE_ARG_NOT_A_CLASS` still says a class is the only answer** —
      `crates/nvs-types/src/expr/args.rs:1560` reports it and `:1479` documents the reading; the door
      now admits a shape, so the wording and the code's `docs/` entry both name two answers. A
      different file from the two above, so take it only if the first two left room.

## Backlog

- `$shape{n}` is an internal label reaching a user-facing throw — `crates/nvs-stdlib/src/json.rs:1035`.
- Registering a shape class from a call site claims `Ty::Tagged` slots, which degrades a same-named
  literal's checked writes in that unit — `crates/nvs-ir/src/lower/mod.rs`, `written_type_constants`.
- Stage 4's `postAs<T>`/`queryAs<T>` and stage 5's `examples/input-shapes.nvs` — loop-goal.md.
- `Core\Db\Connection::queryAs` still refuses an inline shape at run time for the same reason
  `decodeAs` does — `crates/nvs-stdlib/src/db/execute.rs:1161`.
