---
looper: 1
title: A class property is called a property in everything a user reads
summary: "No page, example, attack, benchmark, message, reference card or rule id calls a class property a field, every output that changed is re-recorded, and the checks below pass."
context: [AGENTS.md, docs/agent/prose.md]
stages:
  - id: 1
    title: "the website pages, snippets and URLs say property"
    status: done
    context: [website/src/content/docs/in-depth/concepts/checked-field-names.mdx, website/src/content/docs/guides/why-novis/typed-json.mdx, website/snippets/guides/how-to-use/data-mapper/01-one-field.nvs]
  - id: 2
    title: "the published examples say property"
    status: done
    context: [docs/examples/core/Json/decodeAs/about.md, docs/examples/core/Request/jsonAs/about.md, docs/examples/lang/types/property-paths/about.md, website/src/lib/feature.ts]
  - id: 3
    title: "the attacks and benchmarks say property"
    status: open
    context: [tests/hostile/README.md, benches/members/README.md, tests/hostile/lang/types/property-keys/02-a-write-that-could-reach-a-readonly-field.nvs]
  - id: 4
    title: "the compiler's cards and messages say property"
    status: open
    context: [crates/nvs-diagnostics/src/lib.rs, crates/nvs-types/src/derive.rs, crates/nvs-types/src/conformance.rs]
  - id: 6
    title: "the Core reference cards and runtime errors say property"
    status: open
    context: [crates/nvs-stdlib/src/json.rs, crates/nvs-stdlib/src/db/row.rs]
  - id: 7
    title: "every expectation a changed message pins is re-recorded"
    status: open
    context: [website/snippets/guides/why-novis/typed-json.out]
  - id: 5
    title: "the two rule ids say property"
    status: open
    context: [docs/rules/classes/delegation-by-field.md, docs/rules/core-classes/derive-field-list.md]
checks:
  - id: snippets
    stage: 1
    name: every website snippet and replay matches what the binary does
    run: [bun, nv, site, --check, snippets]
  - id: rules
    stage: 5
    name: the rulebook resolves every rule id and citation
    run: [bun, nv, rules, --check]
---

# A class property is called a property in everything a user reads

## Why

Novis calls a variable of a class a property. The types say so (`property<T>`, `propertyPath<T>`),
and so do `Core\Reflect\ClassInfo::properties`, the syntax pages *Properties* and *Property hooks*,
and the checker's messages. "Field" already names three other things: an anonymous object's members,
an attribute's arguments, and a key in a form, JSON, CSV or a database. A sweep of everything a user
reads found about 140 places that still call a class property a field, so a reader meets two words
for one thing and one word for two things. AGENTS.md § *Text an end user reads* has the rule in its
vocabulary table. This goal brings the text that predates it in line.

## Standing decisions

**The word.** A variable of a class is a *property*. Where the sentence does not already make clear
that it is about a class, it is a *class property*. "Field" stays for an anonymous object's members, an
attribute's arguments, `Core\Json\Field` and `Core\Db\Field`, and a key in a form, JSON, CSV, a log
record or a database. Text that means class properties and anonymous-object fields together stays as
it is. The language keeps its names: no type, method or keyword is renamed.

**The four settled wordings.**
- The message "N field(s) of `T` did not match" (`crates/nvs-stdlib/src/json.rs:2527`) is printed
  for classes and anonymous objects alike, so it says "N value(s) of `T` did not match".
- A "sort field" that is a `propertyPath<T>` is "the property to sort by" in prose, and output that
  says "not a field" or "unknown field" says "not a property path".
- A file or directory name that says "field" for a class property is renamed only where the website
  shows it, as a title or in a URL. Stage 2 checks how `website/src/lib/feature.ts` titles an example
  before it renames one, and a rename moves the feature's proof records with it. Snippet file names
  under `website/snippets/` are not shown, and stay.
- The rule ids `rule:classes/delegation-by-field` and `rule:core-classes/derive-field-list` are
  renamed to `classes/delegation-by-property` and `core-classes/derive-property-list`, with every
  citation in code, help text and docs.

**The two moved pages get no redirect.** The concept page moves to `/in-depth/concepts/checked-property-names/`
and the guide to `/guides/how-to-use/checked-property-paths/`. `website/README.md:105` says a removed
page gets no redirect, and `bun nv site --check structure` fails on a `redirects` entry
(`tools/nv/cmd/site.ts:636`). Simplicity wins over old inbound links, and every link inside the tree
moves with the page.

**How an expectation is re-recorded.** A snippet's `.out` is written by `bun nv site --bless <file>`,
an example's by `bun nv proofs --bless <file>`, and the website's reference data by `bun nv render
--website`. A `.nvst`, `.lspt` or `.err` expectation under `tests/` has no bless command, so it is
edited to the new text the binary prints, one message at a time.

**Left as they are.** The IR mnemonics `field.get`, `field.set` and `field.isinit` appear only in an
IR dump. The internal assert messages about field slots in `crates/nvs-runtime/src/object.rs` cover
classes and anonymous objects together, and so do `crates/nvs-types/src/expr/quals.rs:278` and the
E0851 card. Program identifiers such as `$field` stay unless a comment explains them as a property.

**No word check.** Nothing gates on the word "field", because the right word depends on the meaning.
Each stage reads every hit it changes in context, and the lists below are the places the sweep
confirmed, not a pattern to replace.

**Lanes.** Stages 1, 2 and 3 cover separate trees (`website/`, `docs/examples/`, `tests/hostile/` with
`benches/members/`) and change no printed output, so they run in any order or as a side goal beside
the chain. Stages 4 and 6 change printed messages in separate crates and may run in either order.
Stage 7 runs after both, and after stage 1, because it re-blesses
`website/snippets/guides/why-novis/typed-json.out`, which stage 1 also touches. Stage 5 runs after
stage 4, because both edit `crates/nvs-types/src/derive.rs` and `crates/nvs-types/src/conformance.rs`.
Stage 5 changes rule ids in comments, directives and docs only, so it prints nothing new and needs no
re-record.

**Trade-offs.** None at run time. Stages 4 and 6 change messages the binary prints, so stage 7
re-records every expectation that pins one.

## Stages

### 1. the website pages, snippets and URLs say property

**Does:** the website's pages, components and snippets say property for a class property, and no
URL says field for one.

**Split:** the session moves the two pages and their links. One writing subagent then rewrites the
four guides of the second item, by AGENTS.md § *Text an end user reads*.

- [x] The front page's JSON card says "checks every property". `website/src/components/home/Bento.astro:59`
- [x] The typed JSON guide, the JSON body recipe, the form handler and the tainted input guide say property where they mean the class (the form handler keeps "form field"). `website/src/content/docs/guides/why-novis/typed-json.mdx:4`, `website/src/content/docs/guides/cookbook/read-json-body.mdx:16`, `website/src/content/docs/guides/simple-programs/form-handler.mdx:24`, `website/src/content/docs/guides/how-to-use/tainted-input.mdx:43`, `website/snippets/guides/cookbook/read-json-body/read-json-body.nvs:5`
  - [x] typed-json.mdx and its snippet's comments say property.
  - [x] read-json-body.mdx and its snippet say property.
  - [x] form-handler.mdx and tainted-input.mdx say property for the class.
- [x] The concept page is *How property name types work*, at `/in-depth/concepts/checked-property-names/`, with its description, headings and text in property, and every link to it updated. `website/src/content/docs/in-depth/concepts/checked-field-names.mdx:2`, `website/src/content/docs/in-depth/concepts/index.mdx:11`, `website/src/lib/core.ts:167`, `website/src/content/docs/guides/how-to-use/checked-field-paths.mdx:95`, `website/src/data/questions.json:755`
  - [x] The page file is renamed, and its title, description, headings and text say property.
  - [x] Every link and anchor to it moves: the concepts index, the sidebar, `core.ts`, the guide.
  - [x] Its questions entry moves to the new path.
- [x] The guide *Autocomplete and parsing for class property names in strings* moves to `/guides/how-to-use/checked-property-paths/`, and the how-to index entry links it under that title. `website/src/content/docs/guides/how-to-use/checked-field-paths.mdx:2`, `website/src/content/docs/guides/how-to-use/index.mdx:20`, `website/src/data/questions.json:186`
  - [x] The page file is renamed, and its text says property.
  - [x] The how-to index, the sidebar and the concept page link the new path.
  - [x] Its questions entry moves to the new path.
- [x] The data mapper snippets say property in their comments and output, re-blessed. `website/snippets/guides/how-to-use/data-mapper/01-one-field.nvs:2`, `website/snippets/guides/how-to-use/data-mapper/06-a-sort-field-from-a-request.nvs:21`
  - [x] The comments and printed text of each snippet say property.
  - [x] Each changed snippet is re-blessed, and `bun nv site --check snippets` passes.
- [x] The places the sweep was unsure of are read and decided one by one. `website/src/content/docs/guides/cookbook/read-json-body.mdx:18`, `website/src/content/docs/guides/how-to-use/requests.mdx:58`, `website/src/content/docs/guides/how-to-use/tainted-input.mdx:34`, `website/src/content/docs/guides/example-apps/notes.mdx:120`, `website/snippets/guides/why-novis/typed-json.nvs:14`

### 2. the published examples say property

**Does:** every `about.md` and example comment under `docs/examples/` says property for a class
property, and an example whose title the website shows says it too.

**Split:** the session decides the renames of the last item. Writing subagents then take the
examples one batch after another: the readers and language features (first two items), then
reflection to the spreadsheet chart (third), then `ObjectMap` and `ObjectSet` (fourth).

- [x] The JSON, request and response readers say property. `docs/examples/core/Json/decodeAs/about.md:3`, `docs/examples/core/Json/encode/about.md:5`, `docs/examples/core/Request/jsonAs/about.md:3`, `docs/examples/core/Request/postAs/about.md:3`, `docs/examples/core/Request/queryAs/about.md:4`, `docs/examples/core/Test-Response/jsonAs/about.md:10`, `docs/examples/core/Http-Response/jsonAs/02-a-reply-that-is-not-json.nvs:4`
- [x] The JSON codec attribute, a child's throw, `comparable`, `interfaces` (`implements I by $property`), narrowing, property keys and property paths say property. `docs/examples/lang/attributes/core-json-derive-and-core-json-field-a-class-with-a-json-codec/about.md:6`, `docs/examples/lang/concurrency/a-child-s-throw/about.md:6`, `docs/examples/lang/classes/comparable/02-an-order-built-from-two-properties.nvs:2`, `docs/examples/lang/classes/interfaces/03-one-class-passes-the-work-to-another.nvs:2`, `docs/examples/lang/types/narrowing/about.md:9`, `docs/examples/lang/types/property-keys/about.md:13`, `docs/examples/lang/types/property-paths/about.md:1`
- [x] Reflection, taint, `LateInit`, `PropertyObserver` and the spreadsheet chart say property. `docs/examples/core/Reflect-ClassInfo/hasProperty/03-check-the-properties-a-client-asked-for.nvs:15`, `docs/examples/core/Taint/assertTrustedBytes/about.md:5`, `docs/examples/types/LateInit/01-one-load-sets-every-property.nvs:2`, `docs/examples/types/PropertyObserver/01-an-audit-trail-of-every-change.nvs:3`, `docs/examples/novis/Spreadsheet-Chart/constructor/about.md:4`
- [x] `ObjectMap`, `ObjectSet` and `Spreadsheet\Style` say "equal property values" (or "the same property values") where they say "equal fields" or "the same fields". `docs/examples/core/ObjectSet/add/about.md:5`, `docs/examples/core/ObjectSet/count/about.md:5`, `docs/examples/core/ObjectSet/has/about.md:5`, `docs/examples/core/ObjectSet/intersect/about.md:4`, `docs/examples/core/ObjectSet/remove/about.md:6`, `docs/examples/core/ObjectSet/union/about.md:5`, `docs/examples/core/ObjectMap/get/about.md:6`, `docs/examples/core/ObjectMap/has/about.md:5`, `docs/examples/core/ObjectMap/remove/about.md:5`, `docs/examples/core/ObjectMap/set/about.md:5`, `docs/examples/novis/Spreadsheet-Style/constructor/about.md:5`, `docs/examples/core/ObjectSet/add/02-equal-objects-are-two-members.nvs:2`, `docs/examples/core/ObjectSet/has/02-equal-is-not-the-same-object.nvs:2`, `docs/examples/core/ObjectSet/intersect/02-nothing-in-common.nvs:18`, `docs/examples/core/ObjectMap/set/02-equal-objects-are-two-keys.nvs:2`, `docs/examples/core/ObjectMap/count/02-equal-objects-are-different-keys.nvs:20`
- [x] The sort examples and their `.out` say "the property to sort by" and "not a property path", re-blessed. `docs/examples/core/Reflect/steps/03-check-the-property-to-sort-by.nvs:2`, `docs/examples/lang/types/property-paths/02-the-property-to-sort-by-from-the-address.nvs:29`, `docs/examples/lang/types/property-keys/02-a-function-that-takes-a-property-name.nvs:21`
- [x] The file names that say field are renamed if `website/src/lib/feature.ts` shows them as titles, with their proof records. `docs/examples/types/LateInit/01-one-load-sets-every-property.nvs:1`

### 3. the attacks and benchmarks say property

**Does:** every comment in `tests/hostile/` and `benches/members/` says property for a class
property. Directive lines stay exactly as they are.

**Split:** one writing subagent takes the attacks (first two items), then one takes the benchmarks
(third). The session decides the rename of the last item.

- [ ] The database error and property key attacks say property. `tests/hostile/types/Db-RolledBack/01-a-flood-of-rollbacks-and-a-reason-too-big-to-keep.nvs:28`, `tests/hostile/types/Db-DbError/01-a-flood-of-refusals-and-a-statement-too-big-to-log.nvs:24`, `tests/hostile/lang/types/property-keys/01-request-names-that-reach-for-hidden-fields.nvs:2`, `tests/hostile/lang/types/property-keys/02-a-write-that-could-reach-a-readonly-field.nvs:2`, `tests/hostile/lang/types/property-keys/03-a-tainted-value-written-through-a-key.nvs:2`
- [ ] The error, JSON body and `Core\Program` attacks say property, and the sort attack says "the property to sort by". `tests/hostile/lang/errors/properties-not-accessors/01-properties-read-off-a-throw-under-pressure.nvs:4`, `tests/hostile/core/Request/jsonAs/01-a-chain-of-objects-six-hundred-deep.nvs:95`, `tests/hostile/lang/attributes/core-program-implementing-i-every-class-that-is-an-i/01-every-implementation-asked-for-again-and-again.nvs:7`, `tests/hostile/lang/types/property-paths/01-a-sort-field-from-a-request.nvs:2`
- [ ] The property path, observer, guard, `lateinit` and JSON codec benchmarks say property. `benches/members/lang/types/property-paths.nvs:2`, `benches/members/lang/classes/propertyobserver.nvs:3`, `benches/members/lang/classes/propertyguard.nvs:3`, `benches/members/lang/classes/lateinit.nvs:3`, `benches/members/lang/attributes/core-json-derive-and-core-json-field-a-class-with-a-json-codec.scale.nvs:2`
- [ ] The attack file names that say field are renamed if the website shows them, with their proof records. `tests/hostile/lang/types/property-keys/02-a-write-that-could-reach-a-readonly-field.nvs:1`

### 4. the compiler's cards and messages say property

**Does:** every diagnostic card and compiler message the binary prints says property for a class
property.

- [ ] The error cards E0460 to E0463 and the `implements I by $property` cards E0227, E0720 and E0859 say property. `crates/nvs-diagnostics/src/lib.rs:1722`, `crates/nvs-diagnostics/src/lib.rs:666`, `crates/nvs-diagnostics/src/lib.rs:3457`, `crates/nvs-diagnostics/src/lib.rs:5563`
- [ ] The derive, conformance and parser messages say property. `crates/nvs-types/src/derive.rs:2223`, `crates/nvs-types/src/derive.rs:2332`, `crates/nvs-types/src/derive.rs:733`, `crates/nvs-types/src/conformance.rs:427`, `crates/nvs-syntax/src/parser/decl.rs:995`

### 6. the Core reference cards and runtime errors say property

**Does:** every `Core` reference card and runtime error says property for a class property.

- [ ] The `Json`, `Http`, `Jwt`, `Request`, `Test` and `Db` reference cards say property. `crates/nvs-stdlib/src/json.rs:448`, `crates/nvs-stdlib/src/http.rs:1917`, `crates/nvs-stdlib/src/jwt.rs:737`, `crates/nvs-stdlib/src/request.rs:750`, `crates/nvs-stdlib/src/test.rs:1332`, `crates/nvs-stdlib/src/db/registry.rs:2329`
- [ ] The runtime errors say property, and the decode message says "N value(s) of `T` did not match". `crates/nvs-stdlib/src/db/row.rs:305`, `crates/nvs-stdlib/src/json.rs:2527`, `crates/nvs-stdlib/src/json.rs:2800`, `crates/nvs-codegen/src/emit.rs:3275`

### 7. every expectation a changed message pins is re-recorded

**Does:** every `.out`, `.err`, `.nvst`, `.lspt` and snippet expectation that pins a message stages 4
and 6 changed says the new text, and the website's reference data is rendered again.

**Split:** the session reads the old text of each changed message from the diff of stages 4 and 6's
commits. An `Explore` subagent searches `tests/**/*.{out,err,nvst,lspt}`, `docs/examples/**/*.out`
and `website/snippets/**/*.out` for each old text, and returns `path:line` and the message, one line
each. Writing subagents then edit the `tests/` expectations one directory after
another. The session runs the blesses and the render.

- [ ] The Explore subagent's list of every expectation that pins a changed message. `tests/conformance/core/json-derive-decodes-and-reports-every-field.nvst:1`
- [ ] Each `.nvst`, `.lspt` and `.err` on the list says the new text. `tests/conformance/core/json-derive-decodes-and-reports-every-field.nvst:1`
- [ ] Each snippet and example `.out` on the list is re-blessed. `website/snippets/guides/why-novis/typed-json.out:2`
- [ ] `bun nv render --website` writes the reference data again. `website/src/data/core.json:1`

### 5. the two rule ids say property

**Does:** the rules about delegation and the derive list have ids that say property, and every
citation of them follows.

**Split:** about seventy files cite the two ids, most of them `.nvst` cases and doc comments. The
session renames them with one script under `.agent-tmp/`, deletes the script, and lets `bun nv rules
--check` name any it missed.

- [ ] `rule:classes/delegation-by-field` is renamed to the id `classes/delegation-by-property`, and every citation in code, help text and docs follows. `docs/rules/classes/delegation-by-field.md:1`, `crates/nvs-types/src/conformance.rs:690`, `crates/nvs-diagnostics/src/code/types_e07.rs:418`
- [ ] `rule:core-classes/derive-field-list` is renamed to the id `core-classes/derive-property-list`, and every citation follows. `docs/rules/core-classes/derive-field-list.md:1`, `crates/nvs-types/src/derive.rs:2227`, `crates/nvs-types/tests/derive.rs:1`

## Handoff

### State

Stages 1 and 2 are done. Stage 2 changed about 60 files under `docs/examples/`. Their prose says
property for a class property, and five outputs were re-blessed: the three sort examples,
`hasProperty/03` ("unknown property:") and `PropertyObserver/01` ("properties changed").

`website/src/lib/feature.ts:163` titles an example from its file name, so nine examples whose name
meant a class property were renamed, with their `.out` and `.nvsr`. No proof record names an
example file, so nothing else moved. Names that mean a form, JSON, record, date, shape or attribute
field stay, such as `isPublic/03-list-the-fields-an-api-returns` (the JSON an API sends).

Decided on the way, read in context:
- `Spreadsheet\Style` keeps "fields": its `fields` property is an anonymous object of style fields,
  so "two styles with equal fields" is about that object.
- `postAs`, `queryAs` and `Http-Response/jsonAs` read into "a class or a shape", so their
  sentences mean both together and keep "field". So does `lang/types/mixed/details.md:11` (`->`
  on any object).
- A JSON issue ("one entry for each wrong field") names a key of the document, and stays.

The nightly run 39462919214 the sweep named is the one already fixed on `main` by `21e82d4e6`.

### Next

**Stage 3: the attacks and benchmarks**, one file set: `tests/hostile/` and `benches/members/`. No
gate. Every item follows AGENTS.md § *Text an end user reads*, and a title is the file name as in
stage 2.

- [ ] The database error and property key attacks say property. `tests/hostile/lang/types/property-keys/01-request-names-that-reach-for-hidden-fields.nvs:2`
- [ ] The error, JSON body and `Core\Program` attacks say property, and the sort attack says "the property to sort by". `tests/hostile/lang/types/property-paths/01-a-sort-field-from-a-request.nvs:2`
- [ ] The benchmarks say property. `benches/members/lang/types/property-paths.nvs:2`
- [ ] The attack file names that say field are renamed, with their records. `tests/hostile/lang/types/property-keys/02-a-write-that-could-reach-a-readonly-field.nvs:1`

### Backlog

- None.
