---
looper: 1
title: Field paths the compiler checks
summary: "A dotted path through a class's properties, and a string whose `{…}` holes are such paths, are checked where they are written, completed one segment at a time, checked again at run time, and split by `Core`, with a concept page and a walkthrough on the website."
context: [docs/rules/types/property-key.md]
stages:
  - id: 1
    title: the rules
    status: done
    context: [docs/rules/types/property-key-variance.md, docs/rules/types/property-key-access.md, docs/rules/ide/completion-offers-only-what-the-compiler-derived.md, docs/rules/ide/completion-is-asked-where-a-spelling-ends.md, docs/rules/statements/static-is-a-member-modifier.md]
    extra:
      context:
        rules: [types/property-key, types/property-key-variance, types/property-key-access, types/conversion, ide/completion-offers-only-what-the-compiler-derived, ide/completion-is-asked-where-a-spelling-ends, statements/static-is-a-member-modifier, core-classes/reflect, core-classes/db-one-api, core-classes/db-compile-time-query-checking]
        shapes: [A rule fragment]
  - id: 2
    title: "`static` is the calling class in a parameter and in an array return"
    status: done
    context: [crates/nvs-types/src/signatures.rs, crates/nvs-types/src/expr/calls.rs, crates/nvs-types/src/lower.rs]
    extra:
      context:
        rules: [statements/static-is-a-member-modifier]
        shapes: ["A `.nvst` test case", A diagnostic]
  - id: 3
    title: a literal at a key parameter is checked where it is written
    status: done
    context: [crates/nvs-types/src/expr/operators.rs, crates/nvs-types/src/expr/members.rs, crates/nvs-lsp/src/completion.rs, crates/nvs-lsp/src/arguments.rs]
    extra:
      context:
        rules: [types/property-key, ide/completion-offers-only-what-the-compiler-derived]
        shapes: ["A `.nvst` test case", A diagnostic]
  - id: 4
    title: "`propertyPath<T>` is a type"
    status: done
    context: [crates/nvs-syntax/src/parser/ty.rs, crates/nvs-types/src/ty.rs, crates/nvs-types/src/lower.rs, crates/nvs-types/src/expr/operators.rs, crates/nvs-ir/src/lower/convert.rs]
    extra:
      context:
        rules: [types/property-path, types/property-key, types/conversion]
        shapes: ["A `.nvst` test case", A diagnostic]
  - id: 5
    title: the editor completes a path one segment at a time
    status: done
    context: [crates/nvs-lsp/src/completion.rs, crates/nvs-lsp/src/arguments.rs, crates/nvs-lsp/src/hover.rs, crates/nvs-lsp/src/definition.rs]
    extra:
      context:
        rules: [ide/completion-offers-only-what-the-compiler-derived, ide/completion-is-asked-where-a-spelling-ends]
  - id: 6
    title: "`string<K>` checks the holes in a string"
    status: open
    context: [crates/nvs-syntax/src/parser/ty.rs, crates/nvs-types/src/ty.rs, crates/nvs-types/src/expr/operators.rs, crates/nvs-ir/src/lower/convert.rs, crates/nvs-lsp/src/capabilities.rs]
    extra:
      context:
        rules: [types/string-holes, types/property-path, security/taint-propagation]
        shapes: ["A `.nvst` test case", A diagnostic]
  - id: 7
    title: "`Core\\Str::parts` and `Core\\Reflect::steps`"
    status: open
    context: [crates/nvs-stdlib/src/str.rs, crates/nvs-stdlib/src/reflect.rs, crates/nvs-stdlib/src/registry.rs]
    extra:
      context:
        rules: [core-classes/str-parts-and-reflect-steps, core-classes/reflect]
        shapes: ["A `Core` member", "A `.nvst` test case"]
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
        rules: [testing/website-pages-are-stamped, testing/a-replay-plays-what-the-language-server-returned, core-classes/db-compile-time-query-checking]
checks:
  - id: rules-check
    stage: 1
    name: the rulebook renders and every record is valid
    run: [bun, nv, rules, --check]
  - id: static-binds
    stage: 2
    name: "`static` in a parameter type and in an `array<static>` or `?static` return is the class the call names"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/lang/a-static-parameter-type-is-the-class-the-call-names.nvst, --case, tests/conformance/lang/a-static-array-return-is-the-class-the-call-names.nvst]
    tests: [tests/conformance/lang/a-static-parameter-type-is-the-class-the-call-names.nvst, tests/conformance/lang/a-static-array-return-is-the-class-the-call-names.nvst]
  - id: literal-at-key-parameter
    stage: 3
    name: "a string literal at a `property<T>` parameter is checked where it is written"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/lang/a-string-literal-at-a-property-key-parameter-is-checked-where-it-is-written.nvst]
    tests: [tests/conformance/lang/a-string-literal-at-a-property-key-parameter-is-checked-where-it-is-written.nvst]
  - id: literal-at-key-parameter-editor
    stage: 3
    name: "the editor offers the names at a `property<T>` parameter"
    run: [bun, nv, suite, lsp-test, tests/lsp/, --case, tests/lsp/completion/a-string-at-a-property-key-only-parameter-offers-the-property-names.lspt]
    tests: [tests/lsp/completion/a-string-at-a-property-key-only-parameter-offers-the-property-names.lspt]
  - id: property-path
    stage: 4
    name: "a `propertyPath<T>` steps through objects, nullable objects and arrays, is checked where it is written and again at run time"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/lang/a-property-path-steps-through-objects-nullable-objects-and-arrays.nvst, --case, tests/conformance/lang/a-misspelled-property-path-does-not-compile.nvst, --case, tests/conformance/lang/a-property-path-converted-at-run-time-throws-for-a-bad-segment.nvst]
    tests: [tests/conformance/lang/a-property-path-steps-through-objects-nullable-objects-and-arrays.nvst, tests/conformance/lang/a-misspelled-property-path-does-not-compile.nvst, tests/conformance/lang/a-property-path-converted-at-run-time-throws-for-a-bad-segment.nvst]
  - id: property-path-editor
    stage: 5
    name: "the editor completes, hovers, defines and underlines a path one segment at a time"
    run: [bun, nv, suite, lsp-test, tests/lsp/, --case, tests/lsp/completion/a-property-path-completes-one-segment-at-a-time.lspt, --case, tests/lsp/hover/a-property-path-segment-shows-its-property.lspt, --case, tests/lsp/definition/a-property-path-segment-jumps-to-its-property.lspt, --case, tests/lsp/diagnostics/a-misspelled-property-path-is-underlined-at-its-segment.lspt]
    tests: [tests/lsp/completion/a-property-path-completes-one-segment-at-a-time.lspt, tests/lsp/hover/a-property-path-segment-shows-its-property.lspt, tests/lsp/definition/a-property-path-segment-jumps-to-its-property.lspt, tests/lsp/diagnostics/a-misspelled-property-path-is-underlined-at-its-segment.lspt]
  - id: string-holes
    stage: 6
    name: "a `string<K>` literal checks every hole where it is written, `{{` and `}}` are one brace, and a tainted string stays tainted"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/lang/a-string-with-holes-checks-every-hole-where-it-is-written.nvst, --case, tests/conformance/lang/a-doubled-brace-in-a-string-with-holes-is-one-brace.nvst, --case, tests/conformance/lang/a-string-with-holes-from-a-request-keeps-its-text-tainted.nvst]
    tests: [tests/conformance/lang/a-string-with-holes-checks-every-hole-where-it-is-written.nvst, tests/conformance/lang/a-doubled-brace-in-a-string-with-holes-is-one-brace.nvst, tests/conformance/lang/a-string-with-holes-from-a-request-keeps-its-text-tainted.nvst]
  - id: string-holes-editor
    stage: 6
    name: "a `{` in a `string<K>` literal opens the list of its holes"
    run: [bun, nv, suite, lsp-test, tests/lsp/, --case, tests/lsp/completion/a-brace-in-a-string-with-holes-opens-the-list-of-its-holes.lspt]
    tests: [tests/lsp/completion/a-brace-in-a-string-with-holes-opens-the-list-of-its-holes.lspt]
  - id: parts-and-steps
    stage: 7
    name: "`Core\\Str::parts` gives the text and the checked holes, and `Core\\Reflect::steps` gives each segment's property"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/str-parts-gives-the-text-and-the-checked-holes.nvst, --case, tests/conformance/core/reflect-steps-gives-each-segment-of-a-path.nvst]
    tests: [tests/conformance/core/str-parts-gives-the-text-and-the-checked-holes.nvst, tests/conformance/core/reflect-steps-gives-each-segment-of-a-path.nvst]
  - id: proofs
    stage: 8
    name: every feature this goal adds has all its feature proofs
    run: [bun, nv, proofs, --gate, --only, "lang:types/property-paths", "lang:types/strings-with-holes", "Core\\Str::parts", "Core\\Reflect::steps"]
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
the `Core` functions a library reads them with. It runs after `editor-replay`, because its website
stage shows the completion with that goal's replays.

## Standing decisions

The user agreed the whole design below. A session that finds the code forcing a different answer
takes the fallback named, and records it in the rule.

**`propertyPath<T>`** is a type atom beside `property<T>`, in the same rule family. Its values are
dotted chains of public property names, starting at `T`. Every segment but the last names a property
whose type is a class `C`, a `?C`, or an array of `C` (`array<C>`), and the next segment is
looked up on `C`. The last segment is any public property. `T` follows `property<T>`'s rules exactly:
a class, `E0799` otherwise, visibility decided at the conversion, contravariant
(`rule:types/property-key-variance`), its own equality domain. `as string` is total and gives the
dotted text. A run-time conversion costs one lookup per segment and has no length limit: its cost is
linear in what the caller sent. Reading or writing through a path (`$obj->{$path}`) is not part of
this goal, because a path through an array would read an array of values, and that is a decision of
its own. It goes to the rule's Declined list as "not yet", with that reason.

**A literal at a key parameter is checked where it is written.** At a parameter typed `property<T>`,
`propertyPath<T>` or `string<K>` (alone, not in a union with plain `string`), a string literal is
converted while compiling: a wrong name is the unknown-member error at the literal, and a right one is
accepted with no `as`. This changes `rule:types/property-key` ("a string literal is a `string` and
stays one", `docs/rules/types/property-key.md:12`) and the completion rule's "offered nothing"
(`docs/rules/ide/completion-offers-only-what-the-compiler-derived.md:57`): the editor offers the names
at such a parameter too. A string built at run time still needs `as`. A parameter typed `…|string`
keeps today's meaning: any string, names offered, nothing checked.

**`static` in a parameter type is the class the call names.** At `Category::find(...)` and
`$category->find(...)`, a `static` inside a parameter type (`propertyPath<static>`,
`string<propertyPath<static>>`) is the receiver's class, the way an exact `static` return already is
(`MethodSig::returns_static`, `crates/nvs-types/src/expr/calls.rs:240`,
`crates/nvs-types/src/expr/calls.rs:652`). An `array<static>` and a `?static` return are bound the
same way. Stage 2 took the fallback, because the checker types `static` as the declaring class inside
the body: there a `property<static>` is a `string`, the property's name, and
`rule:statements/static-is-a-member-modifier` says so. Stage 4 does the same for
`propertyPath<static>` (`crates/nvs-types/src/signatures.rs:static_key_as_text`), and stage 7 decides
how such a body reaches `Core\Str::parts` and `Core\Reflect::steps`, either by accepting that text or
by converting it with `as propertyPath<self>`.

**`string<K>`** is a string with holes. The name is the user's call: it adds no new word, so it clashes
with nothing, and it says what the value is. `format` was declined because `Core\Str::format` is the
`printf` member (`crates/nvs-stdlib/src/format.rs:1`), `template` because of HTML templates,
`placeholder` because a SQL query's `:0` and `?` are placeholders (`rule:core-classes/db-parameters`),
and `path` and `text` because each already means something. `string` takes `<` as a type word only
when `<` follows it, the way `property` does (`crates/nvs-syntax/src/parser/ty.rs:91`). `K` is
`property<T>`, `propertyPath<T>`, or a set of allowed string values (`rule:types/single-value-types`).
A hole is `{` + one `K` value + `}`, with no spaces; `{{` is a literal `{` and `}}` a literal `}`; an
empty hole, an unclosed `{` and a lone `}` are errors. Holes work in both quote styles, because a
brace with no `$` after it is already plain text in a string
(`tests/conformance/lang/a-string-literal-escapes-and-interpolates-exactly-what-each-quote-allows.nvst:61`).
A double-quoted string that interpolates `{$…}` is not a literal, so it needs `as string<K>`. `as
string` is total, as it is for `property<T>`, and gives the text as written, braces and escapes
included. The concept page explains in its first lines that `string<K>` is not a collection like
`array<K>`, because a reader may guess that first.

**Taint stays on the text.** `as property<T>` and `as propertyPath<T>` strip `tainted`, because their
output can only be a declared name (`docs/rules/types/property-key.md:21`). `as string<K>` does not:
the text between the holes is still the caller's text. A `tainted string` converts to a
`tainted string<K>`, and its text parts are `tainted string`. So a condition built from a request can
never reach `Core\Db`'s query text, which refuses tainted (`rule:core-classes/db-one-api`). Only the
holes are stripped. This is the security half of the feature, and the attack in stage 8 pins it.

**`Core\Str::parts(string<K> $text): array<string|K>`** gives the text pieces and the holes in order.
A text piece has `{{` and `}}` already turned into one brace, so a library that joins the pieces never
handles an escape. **`Core\Reflect::steps(propertyPath<T> $path): array<Core\Reflect\PropertyInfo>`**
gives one row per segment: the existing description, with the property's name, visibility and declared
type (`crates/nvs-stdlib/src/reflect.rs:1334`), so a library sees whether a step is one object, a
nullable one or an array. No new class is added. `Core` parses only the holes: the text between them
is the library's own language, and query builders are not `Core` (`rule:core-classes/db-one-api`).
Both run once per call, at run time. Splitting a literal while compiling is decided only if the stage
8 bench shows the split costs more than the work around it.

**The editor.** Inside a literal at a `propertyPath<T>` parameter, and inside a hole of a `string<K>`
literal, the list offers `T`'s public properties with their types, and each `.` after a valid prefix
offers the properties of the class reached. Depth is unlimited and a path that loops costs nothing,
because one class is listed per request. `{` becomes a trigger character
(`crates/nvs-lsp/src/capabilities.rs:229`), answered only inside a `string<K>` literal, and
`rule:ide/completion-is-asked-where-a-spelling-ends` says so. Hover on a segment shows its property,
go-to-definition jumps to it, and a misspelled segment is underlined alone, not the whole string. All
of it reads the roster the checker records, never a second walk
(`rule:ide/completion-offers-only-what-the-compiler-derived`).

**The rules** are written in stage 1 as `designed`, owned by this goal: `types/property-path`,
`types/string-holes`, `core-classes/str-parts-and-reflect-steps`, and the changes to
`rule:types/property-key`, `rule:statements/static-is-a-member-modifier` and both completion rules.
Each later stage flips what it builds to `shipped` with its guard tests. Three more designed rules
carry the changes to shipped ones, because a shipped rule cannot hold a designed row:
`types/a-literal-at-a-key-parameter-is-checked-where-it-is-written` (folded into
`rule:types/property-key` by stage 3),
`statements/static-in-a-parameter-type-is-the-class-the-call-names` and
`ide/a-field-path-completes-one-segment-at-a-time`. Each shipped rule names the one that changes it,
and the stage that ships the change folds it into the shipped rule. A `lang:` feature id is the
heading of its section in `docs/reference/lang/20-types.md`, so stages 4 and 6 write the headings
`# Property paths` and `# Strings with holes`, which are `lang:types/property-paths` and
`lang:types/strings-with-holes`.

**Feature proofs and the attack.** `propertyPath<T>`, `string<K>`, `Core\Str::parts` and
`Core\Reflect::steps` each owe every proof (`rule:testing/feature-proofs`), and the `static` change
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
attack. The walkthrough says what this adds to database work: Novis never parses SQL, so a column
typo inside SQL text is found at run time (`rule:core-classes/db-compile-time-query-checking`), and a
field named through a path is checked against the classes while compiling. The pages describe the
combination as it is: the compiler checks field paths through relations inside a string, with no
plugin and no database connection, and checks the same paths at run time. They do not claim no other
language does anything similar. All of it follows AGENTS.md § *Text an end user reads*.

**Trade-offs, agreed.** Performance: a literal costs nothing at run time; a converted string costs one
lookup per segment; `parts` is one pass per call. Memory: the checker keeps one roster per class a key
names, as `property<T>` already does (`crates/nvs-types/src/expr/members.rs:704`); at run time a path is
one string, and `parts` allocates its pieces per call, charged to the request. Usability: field names
are checked and completed where they were plain strings. Simplicity: one new type word
(`propertyPath`), a second form of `string`, one rule that a literal at a key parameter is checked, and
`static` binding in parameters; this is the cost.

## Stages

### 1. the rules

**Does:** every rule this goal builds exists as `designed`, owned by `property-paths`, and the changed rules say what will change.

- [x] `types/property-path`. `docs/rules/types/property-path.md:1`
- [x] `types/string-holes`. `docs/rules/types/string-holes.md:1`
- [x] `core-classes/str-parts-and-reflect-steps`. `docs/rules/core-classes/str-parts-and-reflect-steps.md:1`
- [x] The changed rows of `rule:types/property-key` and `rule:statements/static-is-a-member-modifier`. `docs/rules/types/property-key.md:12`
- [x] The changed rows of both completion rules. `docs/rules/ide/completion-offers-only-what-the-compiler-derived.md:53`
- [x] The feature ids in the `proofs` check match the roster. `docs/goals/property-paths.md:1`

### 2. `static` is the calling class in a parameter and in an array return

**Does:** a `static` in a parameter type and in an `array<static>` or `?static` return is the class the call names.

- [x] Bind `static` in a parameter type at the call. `crates/nvs-types/src/expr/calls.rs:244`
- [x] Bind an `array<static>` and a `?static` return. `crates/nvs-types/src/signatures.rs:1720`
- [x] Keep `static` apart from the declaring class inside the body, or take the fallback. `crates/nvs-types/src/lower.rs:798`

### 3. a literal at a key parameter is checked where it is written

**Does:** a string literal at a `property<T>` parameter is converted while compiling, and the editor offers the names there.

- [x] Convert and check the literal at the argument. `crates/nvs-types/src/expr/members.rs:719`
- [x] Offer the names at a `property<T>`-only parameter. `crates/nvs-lsp/src/arguments.rs:309`

### 4. `propertyPath<T>` is a type

**Does:** `propertyPath<T>` parses, checks a written path, converts a run-time one, and gives its text with `as string`.

- [x] Parse the atom. `crates/nvs-syntax/src/parser/ty.rs:549`
- [x] The type and its lowering. `crates/nvs-types/src/ty.rs:102`
- [x] The written-out check, one diagnostic at the bad segment. `crates/nvs-types/src/expr/operators.rs:300`
- [x] The run-time conversion, one lookup per segment. `crates/nvs-ir/src/lower/convert.rs:1558`

### 5. the editor completes a path one segment at a time

**Does:** completion, hover, go-to-definition and the underline work per segment of a path.

- [x] Completion per segment, from the checker's roster. `crates/nvs-lsp/src/completion.rs:781`
- [x] Hover and go-to-definition on a segment. `crates/nvs-lsp/src/hover.rs:1`
- [x] The underline on the bad segment alone. `crates/nvs-lsp/src/diagnostics.rs:1`

### 6. `string<K>` checks the holes in a string

**Does:** `string<K>` checks every hole of a literal, converts a run-time string, keeps taint on its text, and `{` opens the list.

- [ ] Parse `string<` the way `property<` is parsed, and lower it. `crates/nvs-syntax/src/parser/ty.rs:91`
- [ ] The hole scanner, its escapes and its errors. `crates/nvs-types/src/expr/operators.rs:300`
- [ ] The run-time conversion that keeps `tainted` on the text. `crates/nvs-ir/src/lower/convert.rs:1558`
- [ ] `{` as a trigger character, answered only in a `string<K>` literal. `crates/nvs-lsp/src/capabilities.rs:229`

### 7. `Core\Str::parts` and `Core\Reflect::steps`

**Does:** `Core\Str::parts` splits a `string<K>` into text and holes, and `Core\Reflect::steps` gives the property of each segment of a path.

- [ ] `Core\Str::parts` and its card. `crates/nvs-stdlib/src/str.rs:345`
- [ ] `Core\Reflect::steps` and its card, returning the existing property rows. `crates/nvs-stdlib/src/reflect.rs:1334`

### 8. the feature proofs

**Does:** every feature this goal adds has its description, tests, three examples, a bench, the attack and its help.

- [ ] `propertyPath<T>` and `string<K>`. `docs/examples/lang/types/property-path/01-a-path-through-a-relation.nvs:1`
- [ ] `Core\Str::parts` and `Core\Reflect::steps`. `docs/examples/core/Str/parts/01-text-and-holes.nvs:1`
- [ ] The sort-field attack. `tests/hostile/lang/types/property-path/01-a-sort-field-from-a-request.nvs:1`

### 9. the concept page and the walkthrough

**Does:** the website explains the features together, shows the editor with replays, and links the attack.

- [ ] The concept page. `website/src/content/docs/in-depth/concepts/checked-field-names.mdx:1`
- [ ] The walkthrough and its snippets. `website/src/content/docs/guides/how-to-use/checked-field-paths.mdx:1`
- [ ] The replays of a path being completed and of a misspelled segment. `website/snippets/guides/how-to-use/field-paths/01-complete-a-path.nvs:1`

## Handoff

### State

Stages 1 to 5 are done. Stage 6's compiler half is written and committed on the branch
`wip/string-holes`, **not on `main`**, because one test it breaks is still red:
`a_quote_at_a_property_key_string_opens_the_property_names` (`crates/nvs-lsp/tests/completion.rs:429`)
answers `false` for `Grid::strict('`. It passes on `main`, so a change on the branch causes it. The
branch's own three `.nvst` cases pass. What the branch does: `string<K>` parses (`string` followed by
`<`, and `tainted string<K>`), lowers to `Ty::StringHoles { key, tainted }`, and `K` other than a key, a
path or a set of strings is `E0860`. A literal at a `string<K>` parameter and under `as string<K>` has
every hole checked (`report_unwritten_string_holes`, `crates/nvs-types/src/expr/members.rs:1056`), and a
bad brace or a value outside a set is `E0861`. A run-time `as string<K>` reuses the path walk through
`Helper::StringHolesValid` (`string_holes_miss`, `crates/nvs-runtime/src/helpers.rs:3225`). One scanner,
`nvs_runtime::holes`, serves the checker, the run time and stage 7's `Core\Str::parts`. `as string<K>`
keeps `tainted` through `tainted_result`. `rule:types/string-holes` is shipped with the three `.nvst`
cases, and `docs/novis.md` is regenerated.

`string-holes` and `string-holes-editor` stay red on `main` until the branch lands. The brace case does
not exist yet.
`rule:ide/a-field-path-completes-one-segment-at-a-time` stays `designed` until it does. The nightly CI
failures are on `origin/main` at `cb7021a24`, far behind local `main`. The loop never pushes.

### Next

**Stage 6: land the compiler half, then `{` opens the list of a `string<K>`'s holes**, one file set: `crates/nvs-lsp/src/capabilities.rs`, `crates/nvs-lsp/src/completion.rs`, `crates/nvs-lsp/src/arguments.rs`, `crates/nvs-types/src/expr/members.rs`.

- [ ] Merge `wip/string-holes` into `main` (`git merge --ff-only` works, since `main` has only this handoff on top: rebase the branch first), find why `Grid::strict('` stops opening the names, fix it, and run `bun nv verify`. Suspects are the edits to `crates/nvs-types/src/expr/args.rs:130` and `crates/nvs-types/src/expr/members.rs:880`. Then tick stage 6's first three items and delete the branch with `git branch -d`.

- [ ] Find a `string<K>` literal for the cursor, at a `string<K>` parameter and under `as string<K>`, the way `key_classes` finds a key or a path (`crates/nvs-lsp/src/arguments.rs:305`). `HoleKind` and `hole_kind` are `pub(crate)` and must be exported for it (`crates/nvs-types/src/expr/members.rs:997`). The rosters are already recorded (`record_hole_keys`, `crates/nvs-types/src/expr/members.rs:1042`).
- [ ] Offer inside a hole only: the text from the last unclosed `{` to the cursor is a path prefix, stepped with `path_steps` (`crates/nvs-lsp/src/completion.rs:800`); a key offers its names and a set its values. `{{` opens no hole.
- [ ] `{` as a trigger character, answered only in a `string<K>` literal (`crates/nvs-lsp/src/capabilities.rs:229`, `continues_a_trigger`, `crates/nvs-lsp/src/completion.rs:442`), and the case `tests/lsp/completion/a-brace-in-a-string-with-holes-opens-the-list-of-its-holes.lspt`.
- [ ] Flip `rule:ide/a-field-path-completes-one-segment-at-a-time` to shipped with the four stage 5 cases and the brace case, and fold it into `rule:ide/completion-offers-only-what-the-compiler-derived` and `rule:ide/completion-is-asked-where-a-spelling-ends`. Then stage 6 is done. `data/rules/ide/a-field-path-completes-one-segment-at-a-time.json:1`

### Backlog

- Reading and writing through a path, `$obj->{$path}`: declined for now in `types/property-path`.
- Splitting a `string<K>` literal while compiling, if the stage 8 bench asks for it (`core-classes/str-parts-and-reflect-steps`).
- A literal at a `property<A>|property<B>` parameter is not converted and gets nothing offered (`rule:types/property-key`). Decide it if a program needs it.
- `propertyPath<T>` over a class with no public property reports with the `property<T>` wording (`reject_empty_property_key_set`, `crates/nvs-types/src/expr/operators.rs:414`). Give it the path's word in stage 8.
- A `string<K>` parameter whose `K` names `static` is text inside the body (`static_key_as_text`, `crates/nvs-types/src/signatures.rs`). Stage 7 decides how such a body reaches `parts` and `steps`.
- Go-to-definition on a path segment that names an inherited property answers nothing, because the roster records no declaring class (`property_segment_at`, `crates/nvs-lsp/src/definition.rs:1157`). Hover still shows the type.
- Nightly CI on `cb7021a24` failed `a_spawned_process_ends_when_nvs_is_killed_and_what_it_started_does_not` (`crates/nvs-cli/tests/run_command.rs:688`) and the four-core throughput guard (`crates/nvs-cli/src/serve.rs`). Both are Linux-only. Check them once `main` is pushed.
