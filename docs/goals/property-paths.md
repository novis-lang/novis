---
looper: 1
title: Field paths the compiler checks
summary: "A dotted path through a class's properties, and a string whose `{…}` holes are such paths, are checked where they are written, completed one segment at a time, checked again at run time, and split by `Core`, with a concept page and a walkthrough on the website."
context: [docs/rules/types/property-key.md]
stages:
  - id: 1
    title: the rules
    status: open
    context: [docs/rules/types/property-key-variance.md, docs/rules/types/property-key-access.md, docs/rules/ide/completion-offers-only-what-the-compiler-derived.md, docs/rules/ide/completion-is-asked-where-a-spelling-ends.md, docs/rules/statements/static-is-a-member-modifier.md]
    extra:
      context:
        rules: [types/property-key, types/property-key-variance, types/property-key-access, types/conversion, ide/completion-offers-only-what-the-compiler-derived, ide/completion-is-asked-where-a-spelling-ends, statements/static-is-a-member-modifier, core-classes/reflect, core-classes/db-one-api]
        shapes: [A rule fragment]
  - id: 2
    title: "`static` is the calling class in a parameter and in an array return"
    status: open
    context: [crates/nvs-types/src/signatures.rs, crates/nvs-types/src/expr/calls.rs, crates/nvs-types/src/lower.rs]
    extra:
      context:
        rules: [statements/static-is-a-member-modifier]
        shapes: ["A `.nvst` test case", A diagnostic]
  - id: 3
    title: a literal at a key parameter is checked where it is written
    status: open
    context: [crates/nvs-types/src/expr/operators.rs, crates/nvs-types/src/expr/members.rs, crates/nvs-lsp/src/completion.rs, crates/nvs-lsp/src/arguments.rs]
    extra:
      context:
        rules: [types/property-key, ide/completion-offers-only-what-the-compiler-derived]
        shapes: ["A `.nvst` test case", A diagnostic]
  - id: 4
    title: "`propertyPath<T>` is a type"
    status: open
    context: [crates/nvs-syntax/src/parser/ty.rs, crates/nvs-types/src/ty.rs, crates/nvs-types/src/lower.rs, crates/nvs-types/src/expr/operators.rs, crates/nvs-ir/src/lower/convert.rs]
    extra:
      context:
        rules: [types/property-path, types/property-key, types/conversion]
        shapes: ["A `.nvst` test case", A diagnostic]
  - id: 5
    title: the editor completes a path one segment at a time
    status: open
    context: [crates/nvs-lsp/src/completion.rs, crates/nvs-lsp/src/arguments.rs, crates/nvs-lsp/src/hover.rs, crates/nvs-lsp/src/definition.rs]
    extra:
      context:
        rules: [ide/completion-offers-only-what-the-compiler-derived, ide/completion-is-asked-where-a-spelling-ends]
  - id: 6
    title: "`format<K>` is a string with checked holes"
    status: open
    context: [crates/nvs-syntax/src/parser/ty.rs, crates/nvs-types/src/ty.rs, crates/nvs-types/src/expr/operators.rs, crates/nvs-ir/src/lower/convert.rs, crates/nvs-lsp/src/capabilities.rs]
    extra:
      context:
        rules: [types/format-holes, types/property-path, security/taint-propagation]
        shapes: ["A `.nvst` test case", A diagnostic]
  - id: 7
    title: "`Core\\Format::parts` and `steps`"
    status: open
    context: [crates/nvs-stdlib/src/reflect.rs, crates/nvs-stdlib/src/registry.rs]
    extra:
      context:
        rules: [core-classes/format-parts, core-classes/reflect]
        shapes: [A `Core` member, "A `.nvst` test case"]
  - id: 8
    title: the feature proofs
    status: open
    extra:
      context:
        rules: [testing/feature-proofs]
  - id: 9
    title: the concept page and the walkthrough
    status: open
    context: [website/src/content/docs/in-depth/concepts/tainted-input.mdx, website/src/components/Replay.astro]
    extra:
      context:
        rules: [testing/website-pages-are-stamped, testing/a-replay-plays-what-the-language-server-returned]
checks:
  - id: rules-check
    stage: 1
    name: the rulebook renders and every record is valid
    run: [bun, nv, rules, --check]
  - id: static-binds
    stage: 2
    name: "`static` in a parameter type and in a `static[]` or `?static` return is the class the call names"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/lang/a-static-parameter-type-is-the-class-the-call-names.nvst, --case, tests/conformance/lang/a-static-array-return-is-the-class-the-call-names.nvst]
    tests: [tests/conformance/lang/a-static-parameter-type-is-the-class-the-call-names.nvst, tests/conformance/lang/a-static-array-return-is-the-class-the-call-names.nvst]
  - id: literal-at-key-parameter
    stage: 3
    name: "a string literal at a `property<T>` parameter is checked where it is written, and the editor offers the names there"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/lang/a-string-literal-at-a-property-key-parameter-is-checked-where-it-is-written.nvst]
    tests: [tests/conformance/lang/a-string-literal-at-a-property-key-parameter-is-checked-where-it-is-written.nvst]
  - id: literal-at-key-parameter-editor
    stage: 3
    name: "the editor offers the names at a `property<T>` parameter"
    run: [bun, nv, suite, tests/lsp/, --case, tests/lsp/completion/a-string-at-a-property-key-only-parameter-offers-the-property-names.lspt]
    tests: [tests/lsp/completion/a-string-at-a-property-key-only-parameter-offers-the-property-names.lspt]
  - id: property-path
    stage: 4
    name: "a `propertyPath<T>` steps through objects, nullable objects and arrays, is checked where it is written and again at run time"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/lang/a-property-path-steps-through-objects-nullable-objects-and-arrays.nvst, --case, tests/conformance/lang/a-misspelled-property-path-does-not-compile.nvst, --case, tests/conformance/lang/a-property-path-converted-at-run-time-throws-for-a-bad-segment.nvst]
    tests: [tests/conformance/lang/a-property-path-steps-through-objects-nullable-objects-and-arrays.nvst, tests/conformance/lang/a-misspelled-property-path-does-not-compile.nvst, tests/conformance/lang/a-property-path-converted-at-run-time-throws-for-a-bad-segment.nvst]
  - id: property-path-editor
    stage: 5
    name: the editor completes, hovers, defines and underlines a path one segment at a time
    run: [bun, nv, suite, tests/lsp/, --case, tests/lsp/completion/a-property-path-completes-one-segment-at-a-time.lspt, --case, tests/lsp/hover/a-property-path-segment-shows-its-property.lspt, --case, tests/lsp/definition/a-property-path-segment-jumps-to-its-property.lspt, --case, tests/lsp/diagnostics/a-misspelled-property-path-is-underlined-at-its-segment.lspt]
    tests: [tests/lsp/completion/a-property-path-completes-one-segment-at-a-time.lspt, tests/lsp/hover/a-property-path-segment-shows-its-property.lspt, tests/lsp/definition/a-property-path-segment-jumps-to-its-property.lspt, tests/lsp/diagnostics/a-misspelled-property-path-is-underlined-at-its-segment.lspt]
  - id: format-holes
    stage: 6
    name: "a `format<K>` literal checks every hole where it is written, `{{` and `}}` are one brace, and a tainted string stays tainted"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/lang/a-format-string-checks-every-hole-where-it-is-written.nvst, --case, tests/conformance/lang/a-doubled-brace-in-a-format-string-is-one-brace.nvst, --case, tests/conformance/lang/a-format-string-from-a-request-keeps-its-text-tainted.nvst]
    tests: [tests/conformance/lang/a-format-string-checks-every-hole-where-it-is-written.nvst, tests/conformance/lang/a-doubled-brace-in-a-format-string-is-one-brace.nvst, tests/conformance/lang/a-format-string-from-a-request-keeps-its-text-tainted.nvst]
  - id: format-holes-editor
    stage: 6
    name: "a `{` in a `format<K>` literal opens the list of its holes"
    run: [bun, nv, suite, tests/lsp/, --case, tests/lsp/completion/a-brace-in-a-format-string-opens-the-list-of-its-holes.lspt]
    tests: [tests/lsp/completion/a-brace-in-a-format-string-opens-the-list-of-its-holes.lspt]
  - id: format-parts
    stage: 7
    name: "`Core\\Format::parts` gives the text and the checked holes, and `steps` gives each segment with its property"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/format-parts-gives-the-text-and-the-checked-holes.nvst, --case, tests/conformance/core/format-steps-gives-each-segment-with-its-property.nvst]
    tests: [tests/conformance/core/format-parts-gives-the-text-and-the-checked-holes.nvst, tests/conformance/core/format-steps-gives-each-segment-with-its-property.nvst]
  - id: proofs
    stage: 8
    name: every feature this goal adds has all its feature proofs
    run: [bun, nv, proofs, --gate, --only, "lang:types/property-path", "lang:types/format-holes", "Core\\Format::parts", "Core\\Format::steps"]
  - id: site-snippets
    stage: 9
    name: every snippet and every replay on the website matches the binary
    run: [bun, nv, site, --check, snippets]
  - id: owners-closes
    stage: 9
    name: no designed rule or gap names this goal
    run: [bun, nv, owners, --closes, property-paths]
    reach: true
---

# Field paths the compiler checks

## Why

`property<T>` lets a program choose one of a class's fields late and still have it checked
(`rule:types/property-key`). Real programs name fields further away: a sort column on a related row,
an ORM condition through two relations, the fields to export, a validation error's field. Today each
of those is a plain string that nothing checks until a database or a template fails on it. The class
graph is finite and known while compiling, so the compiler can check such a path, the editor can
complete it one segment at a time, and a path a request sends can be checked the same way at run
time. This goal adds the path, a string with checked `{…}` holes for conditions and templates, and
the `Core` splitter a library reads them with. It runs after `editor-replay`, because its website stage
shows the completion with that goal's replays.

## Standing decisions

The user agreed the whole design below. A session that finds the code forcing a different answer
takes the fallback named, and records it in the rule.

**`propertyPath<T>`** is a type atom beside `property<T>`, in the same rule family. Its values are
dotted chains of public property names, starting at `T`. Every segment but the last names a property
whose type is a class `C`, a `?C`, or an array of `C` (`C[]`, `array<C>`), and the next segment is
looked up on `C`. The last segment is any public property. `T` follows `property<T>`'s rules exactly:
a class, `E0799` otherwise, visibility decided at the conversion, contravariant
(`rule:types/property-key-variance`), its own equality domain. `as string` is total and gives the
dotted text. A run-time conversion costs one lookup per segment and has no length limit: its cost is
linear in what the caller sent. Reading or writing through a path (`$obj->{$path}`) is not part of
this goal, because a path through an array would read an array of values, and that is a decision of
its own. It goes to the rule's Declined list as "not yet", with that reason.

**A literal at a key parameter is checked where it is written.** At a parameter typed `property<T>`,
`propertyPath<T>` or `format<K>` (alone, not in a union with `string`), a string literal is converted
while compiling: a wrong name is the unknown-member error at the literal, and a right one is accepted
with no `as`. This changes `rule:types/property-key` ("a string literal is a `string` and stays one",
`docs/rules/types/property-key.md:12`) and the completion rule's "offered nothing"
(`docs/rules/ide/completion-offers-only-what-the-compiler-derived.md:57`): the editor offers the names
at such a parameter too. A string built at run time still needs `as`. A parameter typed `…|string`
keeps today's meaning: any string, names offered, nothing checked.

**`static` in a parameter type is the class the call names.** At `Category::find(...)` and
`$category->find(...)`, a `static` inside a parameter type (`propertyPath<static>`, `format<…static…>`)
is the receiver's class, the way an exact `static` return already is (`MethodSig::returns_static`,
`crates/nvs-types/src/expr/calls.rs:240`, `crates/nvs-types/src/expr/calls.rs:652`). A `static[]` and a
`?static` return are bound the same way, which `writes_static_return` leaves out today
(`crates/nvs-types/src/signatures.rs:1698`). Inside the method body a `property<static>` or
`propertyPath<static>` value may be applied only to a receiver whose type is `static`, converted with
`as string`, or passed on. Fallback: if the checker cannot keep `static` apart from the declaring class
inside the body, the body may only convert it with `as string` or pass it to `Core\Format`; the rule
says so.

**`format<K>`** is a string with holes. `K` is `property<T>`, `propertyPath<T>`, or a set of allowed
string values (`rule:types/single-value-types`). A hole is `{` + one `K` value + `}`, with no spaces;
`{{` is a literal `{` and `}}` a literal `}`; an empty hole, an unclosed `{` and a lone `}` are errors.
Holes work in both quote styles, because a brace with no `$` after it is already plain text in a
string (`tests/conformance/lang/a-string-literal-escapes-and-interpolates-exactly-what-each-quote-allows.nvst:61`).
A double-quoted string that interpolates `{$…}` is not a literal, so it needs `as format<K>`. `as
string` is total and gives the text as written, braces and escapes included. The name is the user's
call. `Core\Str::format`, the `printf` member, keeps its name (`crates/nvs-stdlib/src/format.rs:1`).

**Taint stays on the text.** `as property<T>` and `as propertyPath<T>` strip `tainted`, because their
output can only be a declared name (`docs/rules/types/property-key.md:21`). `as format<K>` does not:
the text between the holes is still the caller's text. A `tainted string` converts to a
`tainted format<K>`, and its text parts are `tainted string`. So a condition built from a request can
never reach `Core\Db`'s query text, which refuses tainted (`rule:core-classes/db-one-api`). Only the
holes are stripped. This is the security half of the feature, and the attack in stage 8 pins it.

**`Core\Format::parts(format<K> $text): array<string|K>`** gives the text pieces and the holes in
order. A text piece has `{{` and `}}` already turned into one brace, so a library that joins the pieces
never handles an escape. **`Core\Format::steps(propertyPath<T> $path): array<Core\Format\Step>`** gives
one row per segment: `name`, a `string`, and `info`, its `Core\Reflect\PropertyInfo`, which says whether
the step is one object, a nullable one or an array. `Core` parses only the holes: the text between
them is the library's own language, and query builders are not `Core`
(`rule:core-classes/db-one-api`). Both run once per call, at run time. Splitting a literal while
compiling is decided only if the stage 8 bench shows the split costs more than the work around it.

**The editor.** Inside a literal at a `propertyPath<T>` parameter, and inside a hole of a `format<K>`
literal, the list offers `T`'s public properties with their types, and each `.` after a valid prefix
offers the properties of the class reached. Depth is unlimited and a path that loops costs nothing,
because one class is listed per request. `{` becomes a trigger character
(`crates/nvs-lsp/src/capabilities.rs:229`), answered only inside a `format<K>` literal, and
`rule:ide/completion-is-asked-where-a-spelling-ends` says so. Hover on a segment shows its property,
go-to-definition jumps to it, and a misspelled segment is underlined alone, not the whole string. All
of it reads the roster the checker records, never a second walk
(`rule:ide/completion-offers-only-what-the-compiler-derived`).

**The rules** are written in stage 1 as `designed`, owned by this goal: `types/property-path`,
`types/format-holes`, `core-classes/format-parts`, and the changes to
`rule:types/property-key`, `rule:statements/static-is-a-member-modifier` and both completion rules.
Each later stage flips what it builds to `shipped` with its guard tests. The feature ids in the
`proofs` check are this file's guess; stage 1 fixes them to what the roster names.

**Feature proofs and the attack.** `propertyPath<T>`, `format<K>`, `Core\Format::parts` and
`Core\Format::steps` each owe every proof (`rule:testing/feature-proofs`), and the `static` change
updates the class proofs it touches. The attack for `propertyPath<T>` is a sort field from a request:
a `private` property, a path through a relation to one, `{{`, a very long path, a path that loops, and
a tainted condition that tries to reach `Core\Db` query text. Each must fail, and the server must keep
running. Example classes use neutral names (`Order`, `Customer`, `Category`).

**The website** (stage 9) has four parts. A concept page, "Field names the compiler checks", under
`website/src/content/docs/in-depth/concepts/`, beside `tainted-input.mdx`: the idea once, linking to
each feature's page. A walkthrough under `website/src/content/docs/guides/how-to-use/` that builds a
small data mapper from nothing, one runnable snippet per step: one field, a path, a condition string,
`static`, holes to SQL with `parts` and `steps`, a sort field from a request and what a bad one does.
Replays (`editor-replay`) of completing a path and of a misspelled segment's error. A link to the
attack. The pages describe the combination as it is: the compiler checks field paths through
relations inside a string, with no plugin and no database connection, and checks the same paths at
run time. They do not claim no other language does anything similar. All of it follows AGENTS.md §
*Text an end user reads*.

**Trade-offs, agreed.** Performance: a literal costs nothing at run time; a converted string costs one
lookup per segment; `parts` is one pass per call. Memory: the checker keeps one roster per class a key
names, as `property<T>` already does (`crates/nvs-types/src/expr/members.rs:704`); at run time a path is
one string, and `parts` allocates its pieces per call, charged to the request. Usability: field names
are checked and completed where they were plain strings. Simplicity: two type forms, one rule that a
literal at a key parameter is checked, and `static` binding in parameters; this is the cost.

## Stages

### 1. the rules

**Does:** every rule this goal builds exists as `designed`, owned by `property-paths`, and the changed rules say what will change.

- [ ] `types/property-path`. `docs/rules/types/property-path.md:1`
- [ ] `types/format-holes`. `docs/rules/types/format-holes.md:1`
- [ ] `core-classes/format-parts`. `docs/rules/core-classes/format-parts.md:1`
- [ ] The changed rows of `rule:types/property-key` and `rule:statements/static-is-a-member-modifier`. `docs/rules/types/property-key.md:12`
- [ ] The changed rows of both completion rules. `docs/rules/ide/completion-offers-only-what-the-compiler-derived.md:53`
- [ ] The feature ids in the `proofs` check match the roster. `docs/goals/property-paths.md:1`

### 2. `static` is the calling class in a parameter and in an array return

**Does:** a `static` in a parameter type and in a `static[]` or `?static` return is the class the call names.

- [ ] Bind `static` in a parameter type at the call. `crates/nvs-types/src/expr/calls.rs:240`
- [ ] Bind a `static[]` and a `?static` return. `crates/nvs-types/src/signatures.rs:1698`
- [ ] Keep `static` apart from the declaring class inside the body, or take the fallback. `crates/nvs-types/src/lower.rs:798`

### 3. a literal at a key parameter is checked where it is written

**Does:** a string literal at a `property<T>` parameter is converted while compiling, and the editor offers the names there.

- [ ] Convert and check the literal at the argument. `crates/nvs-types/src/expr/members.rs:719`
- [ ] Offer the names at a `property<T>`-only parameter. `crates/nvs-lsp/src/arguments.rs:309`

### 4. `propertyPath<T>` is a type

**Does:** `propertyPath<T>` parses, checks a written path, converts a run-time one, and gives its text with `as string`.

- [ ] Parse the atom. `crates/nvs-syntax/src/parser/ty.rs:549`
- [ ] The type and its lowering. `crates/nvs-types/src/ty.rs:102`
- [ ] The written-out check, one diagnostic at the bad segment. `crates/nvs-types/src/expr/operators.rs:300`
- [ ] The run-time conversion, one lookup per segment. `crates/nvs-ir/src/lower/convert.rs:1558`

### 5. the editor completes a path one segment at a time

**Does:** completion, hover, go-to-definition and the underline work per segment of a path.

- [ ] Completion per segment, from the checker's roster. `crates/nvs-lsp/src/completion.rs:781`
- [ ] Hover and go-to-definition on a segment. `crates/nvs-lsp/src/hover.rs:1`
- [ ] The underline on the bad segment alone. `crates/nvs-lsp/src/diagnostics.rs:1`

### 6. `format<K>` is a string with checked holes

**Does:** `format<K>` checks every hole of a literal, converts a run-time string, keeps taint on its text, and `{` opens the list.

- [ ] Parse the atom and lower it. `crates/nvs-syntax/src/parser/ty.rs:549`
- [ ] The hole scanner, its escapes and its errors. `crates/nvs-types/src/expr/operators.rs:300`
- [ ] The run-time conversion that keeps `tainted` on the text. `crates/nvs-ir/src/lower/convert.rs:1558`
- [ ] `{` as a trigger character, answered only in a `format<K>` literal. `crates/nvs-lsp/src/capabilities.rs:229`

### 7. `Core\Format::parts` and `steps`

**Does:** `Core\Format::parts` splits a `format<K>` into text and holes, and `steps` gives each segment of a path with its property.

- [ ] The members and `Core\Format\Step`, with their cards. `crates/nvs-stdlib/src/format_parts.rs:1`
- [ ] `info` from the reflection roster. `crates/nvs-stdlib/src/reflect.rs:215`

### 8. the feature proofs

**Does:** every feature this goal adds has its description, tests, three examples, a bench, the attack and its help.

- [ ] `propertyPath<T>` and `format<K>`. `docs/examples/lang/types/property-path/01-a-path-through-a-relation.nvs:1`
- [ ] `Core\Format::parts` and `Core\Format::steps`. `docs/examples/core/Format/parts/01-text-and-holes.nvs:1`
- [ ] The sort-field attack. `tests/hostile/lang/types/property-path/01-a-sort-field-from-a-request.nvs:1`

### 9. the concept page and the walkthrough

**Does:** the website explains the features together, shows the editor with replays, and links the attack.

- [ ] The concept page. `website/src/content/docs/in-depth/concepts/checked-field-names.mdx:1`
- [ ] The walkthrough and its snippets. `website/src/content/docs/guides/how-to-use/checked-field-paths.mdx:1`
- [ ] The replays of a path being completed and of a misspelled segment. `website/snippets/guides/how-to-use/field-paths/01-complete-a-path.nvs:1`

## Handoff

### State

Nothing is built. The design is the § *Standing decisions* above, agreed with the user. `property<T>`
is shipped and is the base: its roster is recorded per class
(`crates/nvs-types/src/expr/members.rs:704`), and the editor already offers it at a `property<T>|string`
parameter (`tests/lsp/completion/a-string-at-a-property-key-or-string-parameter-offers-the-property-names.lspt`).
Stage 9 needs `editor-replay` reached first.

### Next

**Stage 1: the rules**, one file set: `docs/rules/types/`, `docs/rules/core-classes/`, `docs/rules/ide/`, `docs/rules/statements/`, `data/rules/`.

- [ ] Write `types/property-path` and `types/format-holes` as `designed` (`rule:types/property-key` is the model). `docs/rules/types/property-path.md:1`
- [ ] Write `core-classes/format-parts` as `designed` (`rule:core-classes/reflect` is the neighbour). `docs/rules/core-classes/format-parts.md:1`
- [ ] Mark the changes in `rule:types/property-key`, `rule:statements/static-is-a-member-modifier` and both completion rules as designed rows owned by this goal. `docs/rules/types/property-key.md:12`

### Backlog

- Reading and writing through a path, `$obj->{$path}`: declined for now in `types/property-path`.
- Splitting a `format<K>` literal while compiling, if the stage 8 bench asks for it (`core-classes/format-parts`).
