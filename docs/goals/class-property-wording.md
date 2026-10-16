---
looper: 1
title: A class property is called a property in everything a user reads
summary: "No page, example, attack, benchmark, message, reference card or rule id calls a class property a field, every output that changed is re-recorded, and the checks below pass."
context: [AGENTS.md, docs/agent/prose.md]
stages:
  - id: 1
    title: "the website pages, snippets and URLs say property"
    status: open
    context: [website/src/content/docs/in-depth/concepts/checked-field-names.mdx, website/src/content/docs/guides/why-novis/typed-json.mdx, website/snippets/guides/how-to-use/data-mapper/01-one-field.nvs]
  - id: 2
    title: "the published examples say property"
    status: open
    context: [docs/examples/core/Json/decodeAs/about.md, docs/examples/core/Request/jsonAs/about.md, docs/examples/lang/types/property-paths/about.md, website/src/lib/feature.ts]
  - id: 3
    title: "the attacks and benchmarks say property"
    status: open
    context: [tests/hostile/README.md, benches/members/README.md, tests/hostile/lang/types/property-keys/02-a-write-that-could-reach-a-readonly-field.nvs]
  - id: 4
    title: "the messages, help and reference cards say property"
    status: open
    context: [crates/nvs-diagnostics/src/lib.rs, crates/nvs-types/src/derive.rs, crates/nvs-stdlib/src/json.rs, crates/nvs-stdlib/src/db/row.rs]
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
for one thing and one word for two things. AGENTS.md § *Text an end user reads* now has the rule in
its vocabulary table. This goal brings the text that predates it in line.

## Standing decisions

**The user decided the word.** A variable of a class is a *property*. Where the sentence does not
already make clear that it is about a class, it is a *class property*. "Field" stays for an anonymous
object's members, an attribute's arguments, `Core\Json\Field` and `Core\Db\Field`, and a key in a
form, JSON, CSV, a log record or a database. Text that means class properties and anonymous-object
fields together stays as it is. The language keeps its names: no type, method or keyword is renamed.

**The user decided the four open questions.**
- The message "N field(s) of `T` did not match" (`crates/nvs-stdlib/src/json.rs:2527`) is printed
  for classes and anonymous objects alike, so it says "N value(s) of `T` did not match".
- A "sort field" that is a `propertyPath<T>` is "the property to sort by" in prose, and output that
  says "not a field" or "unknown field" says "not a property path".
- A file or directory name that says "field" for a class property is renamed only where the website
  shows it, as a title or in a URL. Stage 2 checks how `website/src/lib/feature.ts` titles an example
  before it renames one, and a rename moves the feature's proof records with it.
- The rule ids `rule:classes/delegation-by-field` and `rule:core-classes/derive-field-list` are
  renamed, with every citation in code, help text and docs.

**Left as they are.** The IR mnemonics `field.get`, `field.set` and `field.isinit` appear only in an
IR dump. The internal assert messages about field slots in `crates/nvs-runtime/src/object.rs` cover
classes and anonymous objects together, and so do `crates/nvs-types/src/expr/quals.rs:278` and the
E0851 card. Program identifiers such as `$field` stay unless a comment explains them as a property.

**No word check.** Nothing gates on the word "field", because the right word depends on the meaning.
Each stage reads every hit it changes in context, and the lists below are the places the sweep
confirmed, not a pattern to replace.

**Trade-offs.** None at run time. Stage 4 changes messages the binary prints, so the `.out`, `.err`
and `.nvst` expectations that pin them change in the same commit, re-recorded with `bun nv site
--bless` and the conformance tooling rather than edited by hand.

## Stages

### 1. the website pages, snippets and URLs say property

**Does:** the website's pages, components and snippets say property for a class property, and no
URL says field for one.

- [ ] The front page's JSON card says "checks every property". `website/src/components/home/Bento.astro:59`
- [ ] The typed JSON guide, the JSON body recipe, the form handler and the tainted input guide say property where they mean the class (the form handler keeps "form field"). `website/src/content/docs/guides/why-novis/typed-json.mdx:4`, `website/src/content/docs/guides/cookbook/read-json-body.mdx:16`, `website/src/content/docs/guides/simple-programs/form-handler.mdx:24`, `website/src/content/docs/guides/how-to-use/tainted-input.mdx:43`, `website/snippets/guides/cookbook/read-json-body/read-json-body.nvs:5`
- [ ] The concept page is *How property name types work*, at a URL without "field", with its description, headings and text in property, and every link to it updated. `website/src/content/docs/in-depth/concepts/checked-field-names.mdx:2`, `website/src/content/docs/in-depth/concepts/index.mdx:11`
- [ ] The guide *Autocomplete for class property names in strings* moves to a URL without "field", and the how-to index entry links it under that title. `website/src/content/docs/guides/how-to-use/checked-field-paths.mdx:2`, `website/src/content/docs/guides/how-to-use/index.mdx:20`
- [ ] The data mapper snippets say property in their comments and output, re-blessed. `website/snippets/guides/how-to-use/data-mapper/01-one-field.nvs:2`, `website/snippets/guides/how-to-use/data-mapper/06-a-sort-field-from-a-request.nvs:21`
- [ ] The places the sweep was unsure of are read and decided one by one. `website/src/content/docs/guides/cookbook/read-json-body.mdx:18`, `website/src/content/docs/guides/how-to-use/requests.mdx:58`, `website/src/content/docs/guides/how-to-use/tainted-input.mdx:34`, `website/src/content/docs/guides/example-apps/notes.mdx:120`, `website/snippets/guides/why-novis/typed-json.nvs:14`

### 2. the published examples say property

**Does:** every `about.md` and example comment under `docs/examples/` says property for a class
property, and an example whose title the website shows says it too.

- [ ] The JSON, request and response readers say property. `docs/examples/core/Json/decodeAs/about.md:3`, `docs/examples/core/Json/encode/about.md:5`, `docs/examples/core/Request/jsonAs/about.md:3`, `docs/examples/core/Request/postAs/about.md:3`, `docs/examples/core/Request/queryAs/about.md:4`, `docs/examples/core/Test-Response/jsonAs/about.md:10`, `docs/examples/core/Http-Response/jsonAs/02-a-reply-that-is-not-json.nvs:4`
- [ ] The JSON codec attribute, a child's throw, `comparable`, `interfaces` (`implements I by $property`), narrowing, property keys and property paths say property. `docs/examples/lang/attributes/core-json-derive-and-core-json-field-a-class-with-a-json-codec/about.md:6`, `docs/examples/lang/concurrency/a-child-s-throw/about.md:6`, `docs/examples/lang/classes/comparable/02-an-order-built-from-two-fields.nvs:2`, `docs/examples/lang/classes/interfaces/03-one-class-passes-the-work-to-another.nvs:2`, `docs/examples/lang/types/narrowing/about.md:9`, `docs/examples/lang/types/property-keys/about.md:13`, `docs/examples/lang/types/property-paths/about.md:1`
- [ ] Reflection, taint, `LateInit`, `PropertyObserver` and the spreadsheet chart say property. `docs/examples/core/Reflect-ClassInfo/hasProperty/03-check-the-fields-a-client-asked-for.nvs:15`, `docs/examples/core/Taint/assertTrustedBytes/about.md:5`, `docs/examples/types/LateInit/01-one-load-sets-every-field.nvs:2`, `docs/examples/types/PropertyObserver/01-an-audit-trail-of-every-change.nvs:3`, `docs/examples/novis/Spreadsheet-Chart/constructor/about.md:4`
- [ ] `ObjectMap` and `ObjectSet` say "equal property values" in their examples and in the eleven `about.md` files that say "equal fields". `docs/examples/core/ObjectSet/add/about.md:5`, `docs/examples/core/ObjectMap/count/02-equal-objects-are-different-keys.nvs:20`
- [ ] The sort examples and their `.out` say "the property to sort by" and "not a property path", re-blessed. `docs/examples/core/Reflect/steps/03-check-a-sort-field.nvs:2`, `docs/examples/lang/types/property-paths/02-a-sort-field-from-the-address.nvs:29`, `docs/examples/lang/types/property-keys/02-a-function-that-takes-a-field-name.nvs:21`
- [ ] The file names that say field are renamed if `website/src/lib/feature.ts` shows them as titles, with their proof records. `docs/examples/types/LateInit/01-one-load-sets-every-field.nvs:1`

### 3. the attacks and benchmarks say property

**Does:** every comment in `tests/hostile/` and `benches/members/` says property for a class
property. Directive lines stay exactly as they are.

- [ ] The database error and property key attacks say property. `tests/hostile/types/Db-RolledBack/01-a-flood-of-rollbacks-and-a-reason-too-big-to-keep.nvs:28`, `tests/hostile/types/Db-DbError/01-a-flood-of-refusals-and-a-statement-too-big-to-log.nvs:24`, `tests/hostile/lang/types/property-keys/01-request-names-that-reach-for-hidden-fields.nvs:2`, `tests/hostile/lang/types/property-keys/02-a-write-that-could-reach-a-readonly-field.nvs:2`, `tests/hostile/lang/types/property-keys/03-a-tainted-value-written-through-a-key.nvs:2`
- [ ] The error, JSON body and `Core\Program` attacks say property, and the sort attack says "the property to sort by". `tests/hostile/lang/errors/properties-not-accessors/01-properties-read-off-a-throw-under-pressure.nvs:4`, `tests/hostile/core/Request/jsonAs/01-a-chain-of-objects-six-hundred-deep.nvs:95`, `tests/hostile/lang/attributes/core-program-implementing-i-every-class-that-is-an-i/01-every-implementation-asked-for-again-and-again.nvs:7`, `tests/hostile/lang/types/property-paths/01-a-sort-field-from-a-request.nvs:2`
- [ ] The property path, observer, guard, `lateinit` and JSON codec benchmarks say property. `benches/members/lang/types/property-paths.nvs:2`, `benches/members/lang/classes/propertyobserver.nvs:3`, `benches/members/lang/classes/propertyguard.nvs:3`, `benches/members/lang/classes/lateinit.nvs:3`, `benches/members/lang/attributes/core-json-derive-and-core-json-field-a-class-with-a-json-codec.scale.nvs:2`
- [ ] The attack file names that say field are renamed if the website shows them, with their proof records. `tests/hostile/lang/types/property-keys/02-a-write-that-could-reach-a-readonly-field.nvs:1`

### 4. the messages, help and reference cards say property

**Does:** every diagnostic, help note, `Core` reference card and runtime error the binary prints
says property for a class property, and every expectation that pins one is re-recorded.

- [ ] The error cards E0460 to E0463 and the `implements I by $property` cards E0227, E0720 and E0859 say property. `crates/nvs-diagnostics/src/lib.rs:1722`, `crates/nvs-diagnostics/src/lib.rs:666`, `crates/nvs-diagnostics/src/lib.rs:3457`, `crates/nvs-diagnostics/src/lib.rs:5563`
- [ ] The derive, conformance and parser messages say property. `crates/nvs-types/src/derive.rs:2223`, `crates/nvs-types/src/derive.rs:2332`, `crates/nvs-types/src/derive.rs:733`, `crates/nvs-types/src/conformance.rs:427`, `crates/nvs-syntax/src/parser/decl.rs:995`
- [ ] The `Json`, `Http`, `Jwt`, `Request`, `Test` and `Db` reference cards say property. `crates/nvs-stdlib/src/json.rs:448`, `crates/nvs-stdlib/src/http.rs:1917`, `crates/nvs-stdlib/src/jwt.rs:737`, `crates/nvs-stdlib/src/request.rs:750`, `crates/nvs-stdlib/src/test.rs:1332`, `crates/nvs-stdlib/src/db/registry.rs:2329`
- [ ] The runtime errors say property, and the decode message says "N value(s) of `T` did not match". `crates/nvs-stdlib/src/db/row.rs:305`, `crates/nvs-stdlib/src/json.rs:2527`, `crates/nvs-stdlib/src/json.rs:2800`, `crates/nvs-codegen/src/emit.rs:3275`
- [ ] Every `.out`, `.err`, `.nvst` and snippet expectation the changed messages reach is re-recorded, and the website's reference data is rendered again. `website/snippets/guides/why-novis/typed-json.out:2`

### 5. the two rule ids say property

**Does:** the rules about delegation and the derive list have ids that say property, and every
citation of them follows.

- [ ] `rule:classes/delegation-by-field` is `rule:classes/delegation-by-property`, and every citation in code, help text and docs follows. `docs/rules/classes/delegation-by-field.md:1`, `crates/nvs-types/src/conformance.rs:690`
- [ ] `rule:core-classes/derive-field-list` is `rule:core-classes/derive-property-list`, and every citation follows. `docs/rules/core-classes/derive-field-list.md:1`, `crates/nvs-types/src/derive.rs:2227`

## Handoff

### State

The goal is new. A read-only sweep of the website, `docs/examples/`, `tests/hostile/`,
`benches/members/` and the user-facing strings in `crates/` confirmed the places listed in the
stages. The website guide that was titled *Field paths* is retitled *Autocomplete for class property
names in strings* outside this goal, and its replays were rebuilt then; its URL moves in stage 1.

### Next

Stage 1, the website, on the file set of `website/src/content/docs/` and `website/snippets/`:
- the front page card, the four guides and the JSON body snippet (`website/src/components/home/Bento.astro:59`), per AGENTS.md § *Text an end user reads*
- the concept page and its URL (`website/src/content/docs/in-depth/concepts/checked-field-names.mdx:2`)
- the guide's URL and the data mapper snippets (`website/snippets/guides/how-to-use/data-mapper/01-one-field.nvs:2`)

### Backlog

- The unsure places of stage 1 are decided one by one, never by pattern. `docs/goals/class-property-wording.md`
