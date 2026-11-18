---
looper: 1
title: "an attribute shape names the properties of the class it is on with property<static>"
summary: "Reached when a field typed property<static> in a top-level attribute shape takes a written name of a public property of the class the attribute is on, a wrong name does not compile, Core\\Attributes::get and ::all bind static to their target class, the editor completes those names and jumps to them, and the rule, the reference and the feature proofs say so."
context: [crates/nvs-types/src/attributes.rs, crates/nvs-types/src/retrieval.rs, docs/rules/types/property-key.md, docs/rules/attributes/attach-sites-and-forms.md]
stages:
  - id: 1
    title: the rule is written, and static in a top-level alias binds where the alias is used
    status: open
    context: [crates/nvs-types/src/lower.rs, crates/nvs-types/src/expr/members.rs, docs/rules/attributes/structural-retrieval.md, docs/rules/statements/static-is-a-member-modifier.md]
  - id: 2
    title: a payload names a property with property<static>, and a retrieval binds static to its target
    status: open
    context: [crates/nvs-types/src/expr/members.rs, crates/nvs-types/src/expr/args.rs]
  - id: 3
    title: the editor completes the names and jumps to the property
    status: open
    context: [crates/nvs-lsp/src/completion.rs, crates/nvs-lsp/src/definition.rs, crates/nvs-lsp/tests/completion_file_values.rs]
  - id: 4
    title: the reference, the examples and the attack say what the feature does
    status: open
    context: [docs/examples/lang/types/property-keys/about.md, docs/reference/lang/20-types.md, docs/agent/prose.md]
  - id: 5
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: cases
    stage: 2
    name: "a written name at property<static> in an attribute payload compiles for a public property of the class the attribute is on and is found by a retrieval, and a wrong name, a private one, a use outside a class and the comma-separated form do not compile"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/class/an-attribute-payload-names-a-property-of-its-class.nvst, --case, tests/conformance/class/a-retrieval-binds-static-to-its-target-class.nvst, --case, tests/conformance/reject/an-attribute-payload-naming-no-property-of-its-class-does-not-compile.nvst, --case, tests/conformance/reject/static-in-an-alias-used-outside-a-class-does-not-compile.nvst]
    timeout: 1800
  - id: editor
    stage: 3
    name: "inside a string at property<static> in an attribute payload, the editor offers the class's public properties and jumps to the one written"
    run: [cargo, test, --test, completion_file_values, --, a_string_at_a_property_key_field_of_an_attribute_offers_the_properties_of_its_class, a_name_at_a_property_key_field_of_an_attribute_jumps_to_the_property]
    timeout: 1800
    tests:
      - a_string_at_a_property_key_field_of_an_attribute_offers_the_properties_of_its_class
      - a_name_at_a_property_key_field_of_an_attribute_jumps_to_the_property
---

# an attribute shape names the properties of the class it is on with property<static>

## Why

An attribute often names properties of the class it is written on: the properties a search looks in,
the property a relation stores its key in, the fields a form shows. Today such a field is a `string`,
so a wrong or renamed name compiles and fails only when the program reads it and runs. `property<T>`
already checks a name against a class, and the editor already offers the names. But an attribute's
shape is a top-level `type` alias, so it cannot name the class it will be attached to:
`property<static>` there is `E0303` ("`static` used outside any class"), and even a fixed
`{field: property<Post>}` does not accept `'author'` in a payload.

A user asked for this, with a marker attribute on a `string` field holding a comma-separated list.
The person chose `property<static>` instead: it reuses the type that already exists, its check, its
error and its completion, and it costs nothing at run time.

## Standing decisions

**The syntax (the person's call).** `type Search = { fields: array<property<static>>, exact?: bool };`
and `#[Search(fields: ['author', 'title'])]`. One name is `property<static>`, and a list is
`array<property<static>>`. `propertyPath<static>` follows the same rule at no extra cost, because it
shares `property<T>`'s argument handling (`rule:types/property-path`).

**What `static` means in a top-level alias (my call).** `static` in a `type` alias declared outside any
class names the class at the place the alias is used. The alias is already lowered at each use with
the user's `Ctx` (`crates/nvs-types/src/attributes.rs:689`), so this is the binding the use site has.
At an attribute, it is the class whose body the attribute is written in: the class itself for an
attribute on the class, and the declaring class for one on a property, a method, a constant, a hook or
a parameter. At an ordinary use inside a class it is that class. At a use outside any class it is
`E0303` at the use, and the message names the alias. The alias's own declaration no longer reports
it. `self` and `parent` in a top-level alias keep `E0303` where they are declared: one way to write
it (`rule:statements/nothing-gets-a-second-name`).

**A retrieval binds `static` to its target (my call).** `Core\Attributes::get<Search>(Post::constructor)`
and `::all` lower their type argument with `static` bound to the class the first argument names
(`crate::retrieval::target_declaration`, `crates/nvs-types/src/retrieval.rs:440`), not to the class the
call is written in. A retrieval is folded while compiling (`rule:attributes/retrieval-folds-while-checking`),
so the target is always known. `Core\Program::implementingWith<I, T>` binds it per class it joins.
A retrieval whose target is not a class, such as a function, makes `static` in its type argument
`E0303`.

**A written name in a payload (my call).** A string literal in a payload at a `property<C>` field, also
as an array element, is read as the key-parameter literal is (`rule:types/property-key`,
`crates/nvs-types/src/expr/args.rs:158`): a public declared property of `C` makes it a `property<C>`, and
any other name is the same unknown-member diagnostic, at the literal. `crate::retrieval::literal_type`
(`crates/nvs-types/src/retrieval.rs:663`) is where the payload reads a literal at its field's type, so
the attach check and a retrieval read it the same way. The payload stores the name as written, which
is the key's representation.

**Inheritance (my call).** The names are those of the class `static` binds to, its ancestors' public
properties included, as for every `property<T>`. An attribute is read from its own declaration only
(`rule:attributes/structural-retrieval`), so a subclass never re-checks a parent's payload.

**Declined (my call, written into the rule).** The marker attribute on a `string` field with a
comma-separated list: a second mechanism beside `property<T>`, a small list syntax inside a string, and
a value the reader splits again at run time. A list the configuration declares, such as connection
names, is already declined in `rule:types/single-value-types`.

**What it costs.** Nothing at run time: the check is made while compiling, and the payload is the
same strings. The checker lowers an alias that names `static` once per use, as it does now, and
records the class's property roster once per class for the editor.

**Where it is written down.** A new fragment
`docs/rules/attributes/a-shape-names-its-class-with-static.md`, with its `data/rules/attributes/` JSON,
cited from `rule:types/property-key` and `rule:attributes/attach-sites-and-forms`. The feature is part
of `lang:types/property-keys`, so its proofs are added there.

**Lanes.** Stages 1 and 2 run in order: both change `crates/nvs-types/`. Stage 3 changes
`crates/nvs-lsp/` alone, and stage 4 changes `docs/` and `tests/hostile/` alone. Both need stage 2's
checker and may run side by side after it.

**What stops the goal.** One attempt per item, two sessions per stage. An item still red after its
attempt goes into the handoff with its cause, and the stage moves on to its other items. A stage still
open after two sessions stops the loop with `BLOCKED` naming the item.

## Stages

### 1. the rule is written, and static in a top-level alias binds where the alias is used

**Does:** writes the rule, and moves the `static` check of a top-level alias from its declaration to
each use. Nothing about payloads changes yet.

- [ ] `docs/rules/attributes/a-shape-names-its-class-with-static.md:1`: the new rule with its JSON under
  `data/rules/attributes/`, written from § *Standing decisions*: the syntax, what `static` binds to at an
  attribute, a retrieval and an ordinary use, the written-name check, inheritance, and the Declined list.
  `docs/rules/types/property-key.md:1` and `docs/rules/attributes/attach-sites-and-forms.md:24` cite it.
  Then `bun nv rules --render`, and `bun nv rules --check` is green.
- [ ] `crates/nvs-types/src/lower.rs:880`: `static` in a `type` alias declared outside any class is not
  reported where the alias is declared. Find the caller that lowers each alias at its declaration with
  `bun nv peek --locate`, and skip the report there for `static` alone. At a use outside any class the
  report names the alias: "`static` in `Search` names the class the alias is used in, and this use is
  outside any class". The message is end-user text (`docs/agent/prose.md`).
- [ ] `crates/nvs-types/src/attributes.rs:242` `check_attribute`: confirm that `ctx.current_class` is the
  declaring class for each attach site `check_declaration` (`:50`) walks, the class itself included. A
  site whose `Ctx` has no class gets one there.
- [ ] `tests/conformance/reject/static-in-an-alias-used-outside-a-class-does-not-compile.nvst:1`: an alias
  with `property<static>` declares without an error, is used as a variable's type outside any class, and
  is `E0303` at that use.

### 2. a payload names a property with property<static>, and a retrieval binds static to its target

**Does:** the keystone. A written name at a `property<C>` field is a key, and a retrieval lowers its
type argument with `static` bound to its target.

- [ ] `crates/nvs-types/src/retrieval.rs:663` `literal_type`: a string literal at a `property<C>` or
  `propertyPath<C>` type, alone or as an array element, is checked as `crates/nvs-types/src/expr/args.rs:158`
  checks a literal at a key parameter, through `key_parameter_class`
  (`crates/nvs-types/src/expr/members.rs:862`). A right name gives the key type. A wrong one reports the
  unknown-member diagnostic at the literal once, at attach time, and is `E0401`-free: the payload's own
  mismatch is not reported a second time (`rule:types/one-mistake-reports-once`). The checker records
  the class's roster with `record_property_keys` (`crates/nvs-types/src/expr/members.rs:790`).
- [ ] `crates/nvs-types/src/retrieval.rs:306` `fold_retrieval`: the type argument is lowered with a `Ctx`
  whose `current_class` is the target class from `target_declaration` (`:440`). `matching` (`:561`)
  then reads each payload at that type. `Core\Program::implementingWith` binds it per joined class.
- [ ] `tests/conformance/class/an-attribute-payload-names-a-property-of-its-class.nvst:1`: one name, a
  list, a nullable field and an inherited property, on a class, a property and a method attribute, each
  read back with `Core\Attributes::get` and printed.
- [ ] `tests/conformance/class/a-retrieval-binds-static-to-its-target-class.nvst:1`: one alias on two
  classes with different properties. Each retrieval finds its own payload, and a retrieval written inside
  a third class still binds `static` to its target.
- [ ] `tests/conformance/reject/an-attribute-payload-naming-no-property-of-its-class-does-not-compile.nvst:1`:
  a misspelled name, a `private` property, a `static` property, a name in an array, and the
  comma-separated `'author, title'` form, each one diagnostic at its literal.
- [ ] The `cases` check runs once.

### 3. the editor completes the names and jumps to the property

**Does:** the editor treats a name at a `property<C>` field of an attribute as it treats a key literal
at a key parameter. It reads the shape the checker recorded under the attribute, with `static` already
bound to the class.

- [ ] `crates/nvs-lsp/src/completion.rs:1453` `attribute_values`: where `payload_type` gives a
  `property<C>` or `propertyPath<C>` (also as an array element), the values are `property_keys`
  (`:974`) of `C`, or `path_steps` (`:935`). Items are of kind `property` with their types, as at a key
  parameter.
- [ ] `crates/nvs-lsp/src/definition.rs:1182` `written_property_key`: a string at such a field is a
  written key, so go-to-definition (`:1091`) jumps to the property and hover shows it.
- [ ] `crates/nvs-lsp/tests/completion_file_values.rs:1`: the test
  `a_string_at_a_property_key_field_of_an_attribute_offers_the_properties_of_its_class`, beside the
  attribute tests that end the file, and `a_name_at_a_property_key_field_of_an_attribute_jumps_to_the_property`
  beside it. The `editor` check runs once.
- [ ] `docs/rules/ide/completion-offers-only-what-the-compiler-derived.md:30`: the attribute sentence names
  the property names at a `property<T>` field, read off the same roster.

### 4. the reference, the examples and the attack say what the feature does

**Does:** the user pages. Every line here is text an end user reads, held to `AGENTS.md` § *Text an end
user reads*. Run `bun nv proofs --id lang:types/property-keys` first and last.

**Split:** one writing subagent writes the example and the attack while the session writes the
reference and `about.md`. Their files are disjoint, and no one builds until both end.

- [ ] `docs/examples/lang/types/property-keys/about.md:1`: one paragraph. A field typed
  `property<static>` in an attribute's shape takes the name of a property of the class the attribute is
  written on. A wrong name does not compile.
- [ ] `docs/reference/lang/20-types.md:2887`: the same, with the retrieval binding.
- [ ] `docs/examples/lang/types/property-keys/08-an-attribute-that-names-properties-of-its-class.nvs:1`:
  a search attribute on a class, read back with `Core\Attributes::get`.
- [ ] `tests/hostile/lang/types/property-keys/04-an-attribute-that-names-a-hidden-property.nvs:1`: an
  attack whose payload names a `private` property and a property of another class. It does not compile.
- [ ] `bun nv proofs --id lang:types/property-keys` reports the feature complete, and
  `bun nv proofs --comments` passes on the two new files.

### 5. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Written on 2026-11-18 from a user report. Nothing is built yet. The person chose `property<static>`
over a marker attribute and placed the goal on the chain. Every other choice in § *Standing decisions*
is the authoring session's call under the priority ordering, and stands until the person picks
another. Two parts of the same report already landed: a single-value literal in a payload keeps its
type (`4708c2935a`), and the editor completes a userland attribute's fields and allowed values
(`ed44125957`), which stage 3 builds on.

### Next

Stage 1 from its first item: the rule fragment, then `crates/nvs-types/src/lower.rs:880` and its
caller for a top-level alias, then the `E0303` reject case. The `cases` and `editor` checks name cases
and tests that do not exist yet, and are expected to fail until stages 2 and 3 write them.

### Backlog
