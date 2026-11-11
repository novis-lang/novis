---
looper: 1
title: "a derived JSON codec reads inherited properties, allowed-value sets and meta"
summary: "Reached when #[Core\\Json\\Derive] encodes and decodes inherited properties parents first, accepts a set of allowed values, nvs meta --json carries each derived class's JSON schema, and the derive feature owes no proof."
context: [docs/rules/core-classes/derive-property-list.md, crates/nvs-types/src/derive.rs]
stages:
  - id: 1
    title: a derived class's fields include what its parents declare
    status: open
    context: [docs/rules/core-classes/keep-key-order-freezes-a-derived-classs-keys.md, crates/nvs-ir/src/lower/mod.rs, crates/nvs-diagnostics/src/code/types_e08.rs, tests/conformance/core/json-derive-encodes-declared-fields.nvst, tests/conformance/reject/a-json-derive-field-needs-a-matching-constructor-parameter.nvst]
  - id: 2
    title: a set of allowed values is codec-reachable
    status: open
    context: [docs/rules/types/single-value-types.md, crates/nvs-runtime/src/object.rs, crates/nvs-stdlib/src/json.rs, crates/nvs-stdlib/src/db/row.rs, crates/nvs-cli/src/openapi.rs, crates/nvs-diagnostics/src/code/types_e07.rs]
  - id: 3
    title: nvs meta --json carries each derived class's JSON schema
    status: open
    context: [docs/rules/tooling/meta-json-takes-a-program.md, crates/nvs-cli/src/meta.rs, crates/nvs-cli/src/openapi.rs, crates/nvs-cli/tests/meta.rs]
  - id: 4
    title: the derive feature has its proofs
    status: open
    context: [docs/examples/lang/attributes/core-json-derive-and-core-json-field-a-class-with-a-json-codec/about.md, docs/examples/lang/attributes/core-json-derive-and-core-json-field-a-class-with-a-json-codec/details.md, docs/reference/lang/90-attributes.md, tests/hostile/lang/attributes/core-json-derive-and-core-json-field-a-class-with-a-json-codec/01-documents-a-client-sent-to-break-the-decoder.nvs]
  - id: 5
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: inherited-fields-encode-and-decode
    stage: 1
    name: "a derived subclass encodes and decodes its parents' properties first, and a missing constructor parameter for one is refused at the attribute"
    run: [bun, nv, suite, tests/conformance, --case, tests/conformance/core/json-derive-encodes-inherited-fields-parents-first.nvst, --case, tests/conformance/reject/a-json-derive-subclass-needs-a-parameter-for-each-inherited-field.nvst]
    timeout: 900
  - id: allowed-sets-encode-and-decode
    stage: 2
    name: "a property typed as a set of allowed values encodes as its value, a decode refuses a value outside the set, and a union of two kinds stays E0756 with a help that names mixed"
    run: [bun, nv, suite, tests/conformance, --case, tests/conformance/core/json-derive-checks-an-allowed-value-set.nvst, --case, tests/conformance/reject/a-json-derive-refuses-a-union-of-two-kinds.nvst]
    timeout: 900
  - id: allowed-sets-in-a-row-and-a-schema
    stage: 2
    name: "a row decode checks an allowed-value set, and the OpenAPI schema lists the set as an enum"
    run: [cargo, test, --lib, --test, openapi, --, a_row_decode_checks_an_allowed_value_set, an_allowed_value_set_is_an_enum_in_the_schema]
    timeout: 1800
    tests:
      - a_row_decode_checks_an_allowed_value_set
      - an_allowed_value_set_is_an_enum_in_the_schema
  - id: meta-carries-the-codec
    stage: 3
    name: "nvs meta --json <entry> gives each derived class a json schema with its inherited fields, renamed keys, skipped fields left out and allowed sets as enums"
    run: [cargo, test, --test, meta, --, a_derived_class_carries_its_json_schema]
    timeout: 1800
    tests:
      - a_derived_class_carries_its_json_schema
  - id: derive-proofs-complete
    stage: 4
    name: "the JSON derive feature owes no feature proof"
    run: [bun, nv, proofs, --id, "lang:attributes/core-json-derive-and-core-json-field-a-class-with-a-json-codec"]
    timeout: 600
---

# a derived JSON codec reads inherited properties, allowed-value sets and meta

## Why

A framework that builds forms and tables in the browser from a document the server writes wants each
part of that document to be a `#[Core\Json\Derive]` class, and wants its TypeScript types generated
from `nvs meta --json`. Three gaps stop it, all checked on `main` at 4724df76ac:

- **A derived subclass drops its parents' keys in silence.** `rule:core-classes/derive-property-list`
  makes the fields "the class's own declared properties", and `field_order` walks one class's
  members (`crates/nvs-types/src/derive.rs:2609`). A `Child extends Base` with both derived encodes
  `{"limit":3}`, and `Base`'s `name` is gone. Decoding `Child` is `E0820` with a help line about
  `skip: true` that has nothing to do with it (`crates/nvs-types/src/derive.rs:1777`). Each kind of
  form field is a subclass of a family base, so today every one of them writes its codec by hand.
- **A set of allowed values is `E0756`.** `rule:types/single-value-types` makes `"a"|"b"` a first-class
  type, and `json_reachable` admits a single value (`crates/nvs-types/src/derive.rs:2273`) but not a
  union of them (`:2286`). An enum is the only closed set a codec reads today, and it travels as its
  backing integer, so a browser comparing `kind === "text"` cannot use one.
- **`nvs meta --json <entry>` says nothing about a codec.** A generator re-reads `#[Json\Derive]` and
  `#[Json\Field]` from source text and has to re-implement renames, skips and inheritance.

`mixed` is already codec-reachable (`crates/nvs-types/src/derive.rs:1221`, `:2272`): it compiles,
encodes and decodes nested data. The `E0756` help lists the reachable types and leaves it out
(`crates/nvs-types/src/derive.rs:965`), so a reader concludes it is not allowed. Stage 2 fixes the help.

## Standing decisions

**A derived class's fields are every instance property in its class chain, the root parent's first.**
Each class contributes its fields in its own `field_order`, and the chain is walked from the root
down. A parent counts whether or not it carries `#[Json\Derive]` itself: one rule, no per-parent
opt-in. A parent's own `#[Json\Field]` on its property applies, so a parent can rename or skip a
field for every subclass. Every rule of `rule:core-classes/derive-property-list` holds for an
inherited field as for an own one: it must be a constructor parameter of the subclass with the same
name and type, its type must be codec-reachable, and its key must be unique across the whole chain
(`E0873` naming the first). A refusal for an inherited field is reported at the subclass's attribute,
with a secondary label on the parent's property, because the subclass is the class being derived.
Declined: only parents carrying the attribute count, which is a second rule to learn and still drops
keys in silence for a parent without it. Declined: refusing a derive on a subclass whose parent has
properties, which forces a hand-written codec for every class family.
**What it spends:** a longer compile-time walk per derived class, nothing per object and nothing per
request. The codec's field list grows by the inherited fields, which it should have carried anyway.

**`field_order` keeps its signature.** It still lists one class's members, because `nvs_lsp::order`
asks what one class's fields would be after its own members move. A new function in
`crates/nvs-types/src/derive.rs` walks the chain and calls it per class. `#[Json\KeepKeyOrder]` and
`nvs code order` change nothing: they order a class's own members, and inherited fields always come
first. The rule `rule:core-classes/keep-key-order-freezes-a-derived-classs-keys` gains one sentence
saying so.

**The parent is found through the class table the checker already holds**, the one `class_has_codec`
reads (`crates/nvs-types/src/derive.rs:2297`). A parent with no source declaration (a `Core` class)
contributes no fields. Safe default if the IR join cannot find an inherited slot: the layout is the
flattened slot order (`crates/nvs-ir/src/lower/mod.rs:301`), so join by property name against it, as
the join already does for own fields.

**A set of allowed values is codec-reachable in both formats.** A union whose every non-`null` arm is
a single-value `string` (or every one a single-value `int`) erases to its base wire type, `Str` or
`Int`, and carries its members on a new `allowed` list on `DerivedField` and on
`nvs_runtime::CodecField` (`crates/nvs-runtime/src/object.rs:1301`). `?"a"` and `"a"|"b"|null` are
nullable as any `?T` is. Encode writes the value and checks nothing, because the static type already
holds. Decode reads the base type and then checks membership, and a value outside the set is one
issue with its path, worded like the checked conversion's run-time message in
`rule:types/single-value-types` (`` `"z"` is not one of `"a"`, `"b"` ``), with the same 64-character cut.
The row decoder (`crates/nvs-stdlib/src/db/row.rs:409`) runs the same check, because `codec_ty` is
shared and a column read into `"draft"|"live"` should not skip it. A union that mixes `string` and
`int` single values is not a set of allowed values for this decision and stays `E0756`.
**What it spends:** one list per field per derived class at compile time, and one membership test of
a small closed set per decoded field. Nothing per object beyond the field itself.

**`string|int`, a union of two classes, and any other union of two kinds stay `E0756`.** `mixed` covers
"any JSON value", and a union of two classes cannot be decoded without a discriminator key, which
would be a naming policy that `rule:core-classes/derive-property-list` already declines. The rule's
Declined list gains both, with these reasons. The `E0756` help (`crates/nvs-types/src/derive.rs:965`)
and its card (`crates/nvs-diagnostics/src/code/types_e07.rs:1063`) name `mixed` and a set of allowed
values among the reachable types.

**`nvs meta --json <entry>` gives each user class with a JSON codec a `json` key holding its object
schema.** The schema is the one `nvs build --openapi` already writes for a codec
(`Schemas::object`, `crates/nvs-cli/src/openapi.rs:267`), called from `meta.rs`, never
written a second time. A nested class is a `$ref` to `#/program/schemas/<name>`, and the program
object gains a `schemas` map holding each class a schema reached, once, under the same component
name OpenAPI uses. An allowed-value set is `{"type": "string", "enum": [...]}` in both documents.
Only the codec's result is emitted: the attributes themselves and their argument values are not,
because a renderer that reads raw attributes repeats the derive rules, and `rule:ide/ast-json-schema-is-frozen`
keeps literal text out of machine output. A class with a hand-written `toJson` and no derive gets no
`json` key: nothing knows its shape. The no-argument form stays byte-identical, as the rule requires.
Declined: raw attributes in meta. Declined: both, which gives two ways to read one fact.

**`E0820`'s help names the cause it found.** When the derive is short of a constructor parameter
because a field is skipped, the help stays. When the codec has fewer fields than the constructor has
parameters for any other reason, the help says which parameters no field fills. Stage 1 fixes this
once inherited fields make the common case go away (`crates/nvs-types/src/derive.rs:1595`, `:1777`).

**Chain position:** right behind `calls-by-name-are-checked`, by the user's call (2026-11-11). It
shares `crates/nvs-stdlib/src/json.rs` with `fitted-dependencies`, which runs much later and rebases
onto this.

**Lanes:** stages 1 and 2 both change `crates/nvs-types/src/derive.rs` and run in order. Stage 3
reads stage 1 and 2's codec and runs after them. Stage 4's examples and attack run after stage 3.

## Stages

### 1. a derived class's fields include what its parents declare

**Does:** walk the class chain for a derived class's fields, fix the `E0820` help, and flip the rule.

- [ ] A function in `crates/nvs-types/src/derive.rs` lists a class's fields over its whole chain,
  root first, each class through `field_order` (`rule:core-classes/derive-property-list`,
  `crates/nvs-types/src/derive.rs:2609`).
- [ ] `check_class_format` walks that list in place of `field_order(&decl.members, …)`, with each
  inherited field checked by `codec_field` and `check_constructor_parameter` against the subclass's
  constructor, and refusals reported at the subclass's attribute with a label on the parent's property
  (`crates/nvs-types/src/derive.rs:2466`, `:2795`).
- [ ] The IR join gives an inherited field its slot in the flattened layout and its position in the
  subclass's constructor (`crates/nvs-ir/src/lower/mod.rs:485`).
- [ ] `E0820`'s help names the parameters no field fills when no field is skipped
  (`crates/nvs-types/src/derive.rs:1595`, `:1777`, `crates/nvs-diagnostics/src/code/types_e08.rs:351`).
- [ ] Conformance case `tests/conformance/core/json-derive-encodes-inherited-fields-parents-first.nvst:1`:
  a two-level chain encodes parents first, decodes back to an equal object, an `array<Base>` holding a
  subclass encodes the subclass's keys, and a parent's `#[Json\Field(name: …)]` renames for the
  subclass.
- [ ] Conformance case `tests/conformance/reject/a-json-derive-subclass-needs-a-parameter-for-each-inherited-field.nvst:1`:
  a subclass whose constructor has no parameter for an inherited field is `E0460` at its attribute.
- [ ] `rule:core-classes/derive-property-list`'s first paragraph says every instance property in the
  class chain, root first, with the two Declined lines above; `rule:core-classes/keep-key-order-freezes-a-derived-classs-keys`
  gains its sentence (`docs/rules/core-classes/derive-property-list.md:1`).

### 2. a set of allowed values is codec-reachable

**Does:** admit a set of allowed values in both formats, check it on decode, show it in OpenAPI, and
fix the `E0756` help.

- [ ] `json_reachable` and `db_reachable` admit a union whose non-`null` arms are all single-value
  strings or all single-value ints (`crates/nvs-types/src/derive.rs:2253`, `:2189`).
- [ ] `codec_ty` erases it to `Str` or `Int` and fills a new `allowed` list on `DerivedField`
  (`crates/nvs-types/src/derive.rs:1171`, `:997`), carried to `nvs_runtime::CodecField::allowed`
  (`crates/nvs-runtime/src/object.rs:1301`) through the IR join and codegen.
- [ ] The JSON decoder checks membership after the base read and records one issue with its path
  (`crates/nvs-stdlib/src/json.rs:5473`).
- [ ] The row decoder runs the same check (`crates/nvs-stdlib/src/db/row.rs:409`), with the test
  `a_row_decode_checks_an_allowed_value_set` in its `#[cfg(test)]` module
  (`crates/nvs-stdlib/src/db/row.rs:1267`).
- [ ] The OpenAPI renderer writes the set as `enum` beside the base type
  (`crates/nvs-cli/src/openapi.rs:291`), with the test `an_allowed_value_set_is_an_enum_in_the_schema`
  in `crates/nvs-cli/tests/openapi.rs:1`.
- [ ] The `E0756` help and card name `mixed` and a set of allowed values
  (`crates/nvs-types/src/derive.rs:965`, `crates/nvs-diagnostics/src/code/types_e07.rs:1063`).
- [ ] Conformance cases `tests/conformance/core/json-derive-checks-an-allowed-value-set.nvst:1`
  (encode, decode, a value outside the set reported with its path, `?"a"` takes `null`) and
  `tests/conformance/reject/a-json-derive-refuses-a-union-of-two-kinds.nvst:1` (`string|int` and
  `A|B` are `E0756`, and the help names `mixed`).
- [ ] `rule:core-classes/derive-property-list`'s reachable list names a set of allowed values and
  `mixed`, and its Declined list gains the union of two kinds and the union of two classes
  (`docs/rules/core-classes/derive-property-list.md:1`).

### 3. nvs meta --json carries each derived class's JSON schema

**Does:** emit the codec's object schema on each derived user class, through OpenAPI's renderer.

- [ ] `Schemas` (or the part of it `object` needs) is callable from `meta.rs` with a `$ref`
  prefix argument, so both documents share one renderer (`crates/nvs-cli/src/openapi.rs:267`).
- [ ] `user_class_json` adds `json` for a class whose codec the checker recorded
  (`ExprTypeTable::codec`, `crates/nvs-types/src/expr_table.rs:2426`), and `program_json` adds the
  `schemas` map (`crates/nvs-cli/src/meta.rs:1034`, `:920`).
- [ ] Fixture `crates/nvs-cli/tests/fixtures/meta/derive.nvs:1` and test
  `a_derived_class_carries_its_json_schema` in `crates/nvs-cli/tests/meta.rs:1`: an inherited field,
  a renamed key, a skipped field absent, an allowed set as `enum`, a nested class as `$ref`, and a
  class with no codec has no `json` key.
- [ ] `rule:tooling/meta-json-takes-a-program` states the `json` key, the `schemas` map and the two
  Declined lines above (`docs/rules/tooling/meta-json-takes-a-program.md:1`).

### 4. the derive feature has its proofs

**Does:** the examples, the attack step, the reference and the user pages for the three changes.

**Split:** one writing subagent writes the three examples and the attack step, held to AGENTS.md
§ *Text an end user reads*. The session writes `about.md`, `details.md` and the reference, and checks
the comments with `bun nv proofs --comments`.

- [ ] Example `docs/examples/lang/attributes/core-json-derive-and-core-json-field-a-class-with-a-json-codec/08-a-subclass-writes-its-parents-keys-first.nvs:1`.
- [ ] Example `docs/examples/lang/attributes/core-json-derive-and-core-json-field-a-class-with-a-json-codec/09-a-property-with-a-set-of-allowed-values.nvs:1`.
- [ ] Example `docs/examples/lang/attributes/core-json-derive-and-core-json-field-a-class-with-a-json-codec/10-a-property-that-holds-any-json-value.nvs:1`.
- [ ] The attack gains a step that sends a value outside an allowed set and one that sends a parent's
  key with the wrong type, each refused with its path
  (`tests/hostile/lang/attributes/core-json-derive-and-core-json-field-a-class-with-a-json-codec/01-documents-a-client-sent-to-break-the-decoder.nvs:1`).
- [ ] `about.md` and `details.md` say what a subclass encodes, what a set of allowed values does and
  that `mixed` holds any JSON value
  (`docs/examples/lang/attributes/core-json-derive-and-core-json-field-a-class-with-a-json-codec/about.md:1`).
- [ ] The reference section says the same (`docs/reference/lang/90-attributes.md:684`), and the
  `nvs meta` reference names the `json` key.
- [ ] `bun nv proofs --id 'lang:attributes/core-json-derive-and-core-json-field-a-class-with-a-json-codec'`
  owes nothing, and the website renders (`bun nv render --website`).

### 5. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Nothing built yet. The gaps were reproduced on `main` at 4724df76ac with the release `nvs`: a derived
subclass encodes only its own keys, `"a"|"b"` is `E0756`, and `mixed` already works.

### Next

Stage 1, all items: they share `crates/nvs-types/src/derive.rs` and the IR join. Expect
`tests/conformance/core/json-derive-encodes-declared-fields.nvst` to stay green, and any test that
pins a subclass's own-keys-only output to go red and be updated.

### Backlog
