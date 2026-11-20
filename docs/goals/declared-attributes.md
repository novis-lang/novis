---
looper: 1
title: "an attribute is a declaration that names where it may be used, and the editor offers only the attributes that fit"
summary: "Reached when only an attribute declaration may follow #[, each declares the sites it may be used on, one declared inside a class or interface is used only on that type's descendants, a second use needs repeatable, retrieval matches the declaration and never the shape, Core's compiler attributes are declarations in the stub tree, every shape field may carry a doc comment, the editor offers at #[ only the attributes that fit and shows each key's doc, the corpus is migrated, and the feature has its proofs and pages."
context: [docs/rules/attributes/attach-sites-and-forms.md, crates/nvs-types/src/attributes.rs, crates/nvs-types/src/derive.rs]
stages:
  - id: 1
    title: the rules are written, and the cases that pin them are written ahead
    status: done
    context: [docs/rules/attributes/inert-metadata.md, docs/rules/attributes/structural-retrieval.md, docs/rules/attributes/repeatable.md, docs/rules/types/shape-type.md, docs/rules/types/class-scoped-alias.md]
  - id: 2
    title: the parser reads an attribute declaration and a doc comment on a shape field, and the formatter writes both
    status: done
    context: [crates/nvs-syntax/src/parser/decl.rs, crates/nvs-syntax/src/parser/ty.rs, crates/nvs-syntax/src/parser/mod.rs, crates/nvs-syntax/src/ast.rs, crates/nvs-fmt/src/lib.rs]
  - id: 3
    title: the checker resolves an attribute to its declaration and checks its site, its owner, its count and its payload
    status: open
    context: [crates/nvs-hir/src/aliases.rs, crates/nvs-hir/src/autoload.rs, crates/nvs-diagnostics/src/code/types_e08.rs]
  - id: 4
    title: Core's compiler attributes are declarations, and one site error covers them all
    status: open
    context: [crates/nvs-lsp/src/stubs.rs, docs/rules/attributes/access-payload.md]
  - id: 5
    title: every retrieval matches the declaration, never the shape
    status: open
    context: [crates/nvs-types/src/retrieval.rs, crates/nvs-types/src/match_attribute.rs, crates/nvs-types/src/program.rs, crates/nvs-codegen/src/lib.rs, crates/nvs-stdlib/src/reflect.rs]
  - id: 6
    title: the corpus declares its attributes
    status: open
    context: [docs/agent/prose.md]
  - id: 7
    title: the editor offers only the attributes that fit, and shows each key's doc
    status: open
    context: [crates/nvs-lsp/src/completion.rs, crates/nvs-lsp/src/hover.rs, crates/nvs-lsp/src/card.rs, crates/nvs-lsp/src/definition.rs, crates/nvs-lsp/src/semantic.rs, editors/vscode/syntaxes/nvs.tmLanguage.json]
  - id: 8
    title: the feature's proofs, the reference and the website say what attributes are now
    status: open
    context: [docs/reference/lang/90-attributes.md, docs/examples/lang/attributes/chapter.md, docs/agent/prose.md]
  - id: 9
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: cases
    stage: 3
    name: "an attribute declaration is used on its sites and its owner's descendants, and a wrong site, a wrong owner, a second use, a class or alias after #[ and an unread site do not compile"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/lang/an-attribute-is-declared-with-its-sites.nvst, --case, tests/conformance/lang/an-attribute-declared-in-a-class-is-used-on-its-descendants.nvst, --case, tests/conformance/lang/a-shape-field-carries-a-doc-comment.nvst, --case, tests/conformance/reject/an-attribute-on-a-site-it-does-not-list-does-not-compile.nvst, --case, tests/conformance/reject/an-attribute-outside-its-owners-descendants-does-not-compile.nvst, --case, tests/conformance/reject/a-second-use-of-an-attribute-that-is-not-repeatable-does-not-compile.nvst, --case, tests/conformance/reject/only-an-attribute-declaration-follows-the-attribute-bracket.nvst, --case, tests/conformance/reject/an-attribute-declaration-lists-only-sites-a-retrieval-reads.nvst, --case, tests/conformance/reject/an-attribute-name-two-scopes-give-does-not-compile.nvst]
    timeout: 1800
  - id: retrieval
    stage: 5
    name: "a retrieval finds the payloads of the declaration it names and no other, also two declarations with the same fields, and reflection names the declaration"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/lang/a-retrieval-matches-the-declaration-it-names.nvst, --case, tests/conformance/core/a-reflected-attribute-names-its-declaration.nvst]
    timeout: 1800
  - id: editor
    stage: 7
    name: "the editor offers at #[ only the attributes whose sites and owner fit, the site words after on, and each key with its doc"
    run: [bun, nv, suite, lsp-test, tests/lsp/, --case, tests/lsp/completion/an-attribute-bracket-offers-only-the-attributes-that-fit.lspt, --case, tests/lsp/completion/an-attribute-bracket-on-a-descendant-offers-its-owners-attributes.lspt, --case, tests/lsp/completion/the-on-list-of-an-attribute-offers-the-site-words.lspt, --case, tests/lsp/completion/an-attribute-payload-offers-each-key-with-its-doc.lspt, --case, tests/lsp/hover/an-attribute-name-answers-its-declaration.lspt, --case, tests/lsp/hover/a-key-in-an-attribute-payload-answers-its-doc.lspt]
    timeout: 1800
    tests:
      - tests/lsp/completion/an-attribute-bracket-offers-only-the-attributes-that-fit.lspt
      - tests/lsp/completion/an-attribute-bracket-on-a-descendant-offers-its-owners-attributes.lspt
      - tests/lsp/completion/the-on-list-of-an-attribute-offers-the-site-words.lspt
      - tests/lsp/completion/an-attribute-payload-offers-each-key-with-its-doc.lspt
      - tests/lsp/hover/an-attribute-name-answers-its-declaration.lspt
      - tests/lsp/hover/a-key-in-an-attribute-payload-answers-its-doc.lspt
  - id: proofs
    stage: 8
    name: "the declaring-an-attribute feature owes no proof"
    run: [bun, nv, proofs, --id, "lang:attributes/an-attribute-is-declared-with-the-places-it-may-be-used", --gate]
    timeout: 1800
---

# an attribute is a declaration that names where it may be used, and the editor offers only the attributes that fit

## Why

Today an attribute is any `type` alias whose right-hand side is a shape, or the bare `#[{…}]`. Nothing
says where an attribute belongs. So a developer must already know which attribute does what and where:

- any type name fits after `#[`, and the editor offers every class and alias there;
- an attribute meant for one kind of class, such as the index and column attributes of a framework's
  stored-object base class, compiles on any declaration, where nothing reads it;
- a field of a shape has no doc comment, so the editor cannot say what a key means;
- a retrieval matches by shape, so two unrelated attributes with the same fields answer each other's
  retrievals, and nothing links an attribute to the code that reads it.

The person wants the "must know" gone: typing `#[` should show exactly the attributes that mean
something at that place, with their docs, the way `Arr::` shows the array functions. Breaking changes
are accepted. The person called it a big refactor that is the right call and must be done properly.

## Standing decisions

**The person's calls.**

- An attribute declared inside a class or interface body is usable **only** on that type's descendants:
  the class itself when it extends or implements the owner, and the members and parameters of such a
  class. An attribute declared at namespace level is usable everywhere its sites allow. Both exist.
- Every attribute declares an allowlist of sites, one or more. Nothing is allowed by default.
- The design below as a whole.

**The declaration (my call, approved as a design).**

```nvs
abstract class Storable
{
    /// Adds a database index over one or more columns of this table.
    repeatable attribute Index on class {
        /// The column names, separated by commas.
        columns: string,
        /// `true` means no two rows may have the same values in these columns.
        unique?: bool,
    }

    /// Sets how this property is stored.
    attribute Column on property, parameter {
        /// What happens to this row when the row it points to is deleted.
        onTargetDelete?: 'deleteThisObjectToo'|'setNull'|'refuse',
    }
}
```

- `[repeatable] attribute Name on <site>, <site> { field, … }`, at namespace level or as a member of a
  class or interface body. An enum body declares none. `attribute`, `repeatable` and `on` are
  contextual words, read as `type` is (`crates/nvs-syntax/src/parser/decl.rs:874`), so no identifier
  is taken from programs.
- The body is a shape's field list, `name: T` and `name?: T`, with the shape's own grammar and check.
  Defaults on fields are left for a later goal.
- A doc comment above the declaration documents the attribute. A doc comment above a field documents
  the key.
- The declaration is a name of its own kind, not a type. It cannot be written in a type position, and
  `#[`, a retrieval's type argument and an `implementingWith` argument are the only places that name
  it.

**The site words (my call).** `class`, `interface`, `enum`, `method` (a constructor included),
`property`, `hook`, `constant`, `case`, `parameter`. They are fixed words, and the list after `on`
has no wildcard such as `on any`: a wildcard brings back an attribute that shows up everywhere and
says nothing about where it means something. A promoted constructor parameter is a property and a
parameter, and an attribute fits there when its list has either word.

**Sites nothing reads (my call).** A user's declaration may list only the sites a retrieval reads:
`class`, `method`, `property` and `parameter`. Listing another site is an error at the declaration,
and that replaces `E0871` at each use (`crates/nvs-types/src/attributes.rs:115`). Only `Core`'s
declarations list `interface`, `enum`, `hook`, `constant` and `case`, for example
`Core\Deprecated`. When a retrieval learns to read one of those sites, the restriction for that site
is lifted in the rule.

**Resolution (my call).** `#[Name(…)]` resolves `Name` through the namespace and `use` scope as today.
On a descendant of an owner, the owner's attributes are also in scope by their short names, and an
inherited owner's attributes too: the owner chain is walked through `nvs_hir::ClassGraph`. The
qualified `#[Storable::Index(…)]` resolves to the same declaration, so the owner check applies to both
forms. A short name that both the owner chain and the namespace scope give is an error naming both,
never a silent choice. A name that resolves to a class or a `type` alias is an error that says only
an attribute declaration may follow `#[`, and its fix rewrites a shape alias declared in the program
to `attribute Name on <the site of this use> { … }`
(`rule:ide/a-code-action-ships-only-a-fix-a-diagnostic-already-knows`).

**The checks at a use (my call).** Each is its own diagnostic at the attribute, in this order, and one
mistake is reported once:

1. the site is one the declaration lists; the message names the allowed sites and this site;
2. the owner check, for an owned declaration; the message names the owner;
3. a second use on one declaration needs `repeatable`; the message names both uses;
4. the payload fits the fields, with the check `rule:attributes/attach-sites-and-forms` already makes
   (single-value literals, nested shapes and arrays included).

`#[Access]`'s "exactly one" (`E0763`) becomes check 3, since `Core\Access` is not `repeatable`. The
same goes for `#[Api]`'s second use. `#[Route]` and `#[Index]`-like attributes are `repeatable`.

**Retrieval (my call).** `Core\Attributes::get<Storable::Index>(…)` and `::all<…>` name a declaration
and find only the payloads written under it. The match is by declaration, never by shape. So
the structural match is replaced by `rule:attributes/a-retrieval-matches-the-declaration-it-names`, and an inline shape such as `get<{name: string}>`
does not compile. `Core\Router\Match::attribute<T>()` and `Core\Program::implementingWith<I, T>`
take a declaration the same way. `Core\Reflect\AttributeInfo::name()` returns the declaration's full
name. A retrieval still folds while compiling (`rule:attributes/retrieval-folds-while-checking`).

**The bare form `#[{…}]` is removed (my call).** Nobody can discover it, and no retrieval can name it.
Its parse error says to declare an attribute.

**`Core`'s compiler attributes become declarations (my call).** Their data stays Rust data in
`crates/nvs-types/src/derive.rs`, so start-up parses nothing new, but each row becomes the same
declaration value the checker builds from a program's `attribute` declaration: its name, its sites,
`repeatable`, its fields and their docs. The checker, the stub tree and the editor read that one
value. `derive::sites` (`crates/nvs-types/src/derive.rs:150`) and the card/shape split in the editor
(`crates/nvs-lsp/src/completion.rs:1419`) go away, and `E0874` becomes the site error of check 1. What
a recognizing pass checks beyond the site, such as `#[Route]` needing `#[Access]`, stays with that pass.

**A doc comment on every shape field (my call).** `///` above a field of any shape type, not only in
an attribute, is that key's doc. `nvs_syntax::ast::ShapeField` (`crates/nvs-syntax/src/ast.rs:276`)
gains the doc, and hover and completion show it for a key in an object literal, a payload, a property
access on a shape and a named argument whose parameter is a shape.

**What it costs.** Nothing at run time and nothing per request: a payload is still a compile-time
constant, and a retrieval still folds to its answer. While compiling, each attribute use costs one
site check, at most one walk up its class's parents, and one count per declaration. The language gains
three contextual words and one declaration kind, and loses the bare form, the structural match, the
alias-as-attribute rule and the second attribute mechanism `Core` used.

**Where it is written down.** A new fragment `docs/rules/attributes/an-attribute-is-declared-with-its-sites.md`
holds the declaration, the site words, the owner rule, `repeatable` and resolution, with the Declined
list below. `inert-metadata`, `attach-sites-and-forms`, `structural-retrieval`, `repeatable`,
`retrieval-folds-while-checking`, `access-payload`, `routing/a-match-reads-an-attribute-as-data`,
`programs/implementing-with` and `types/shape-type` are rewritten to match. Every
`docs/rules/attributes/` fragment's `**Declined:**` list moves an item that is now the rule.

**Declined (written into the new fragment).**

- **An attribute usable anywhere by its qualified name**, `#[Storable::Index]` on any class. It brings
  back an attribute that compiles where nothing reads it.
- **`on any`, or no `on` list meaning everywhere.** The same.
- **A meta-attribute on a `type` alias**, `#[Core\Attribute(on: 'class')] type Index = {…}`. It is one
  more name a developer must know, and a type alias carries no attributes.
- **Keeping the structural match beside the declaration.** Two attributes with the same fields would
  answer each other's retrievals again.
- **An attribute class, constructed when read.** Still class overhead for a bag of data.

**Lanes.** Stages 2 to 5 run in order: each builds on the one before, and 3 to 5 share
`crates/nvs-types/`. Stage 6 needs stage 5, and its batches are disjoint directories. Stage 7 changes
`crates/nvs-lsp/` and `editors/vscode/` alone and needs stage 4, so it may run beside stage 6 as a
side goal. Stage 8 needs stages 6 and 7.

**The diagnostic codes (stage 1's call).** A released code is retired, never repurposed
(`crates/nvs-diagnostics/src/code/mod.rs:3`), so a check whose meaning moves takes a new code, and a
check that only widens keeps its code. The frozen cases name these:

- `E0874` widens: every attribute used on a site its declaration does not list (check 1).
- `E0763` widens: a second use of any declaration that is not `repeatable` (check 3).
- `E1004`: a program's declaration lists a site no retrieval reads. `E0871` is retired.
- `E1005`: a class, interface, enum or `type` alias after `#[`. `E0726` is retired.
- `E1006`: an owned attribute used outside its owner's descendants (check 2).
- `E1007`: a short name both the owner chain and the namespace scope give.
- `E1008`: a retrieval's, `Match::attribute`'s or `implementingWith`'s `T` is not an attribute
  declaration. `E0729` is retired.
- `E0140`: the bare `#[{…}]`. `E0141`: a missing or empty `on` list, or a word that is not a site.
  `E0142`: an attribute declaration in an enum body.
- An attribute declaration in a type position is the ordinary `E0303`, a second member of one name in
  a body is `E0304`, and `Sub::Index` for an inherited owner is `E0405`, as for an alias.

The eight rewritten rules and `enums/no-class-machinery`'s `E0871` sentence state the designed
behaviour, so each rewritten record is `designed` under this goal until stage 9 sets it `shipped`.

**The syntax tree (stage 2's calls).**

- `nvs_syntax::ast::ShapeField::doc` is an `Option<DocComment>`, not a bare span. It is taken with
  `take_doc_comment`, as every other declaration's run is, so the run counts as attached, its tags are
  checked, and hover reads the same type everywhere.
- A modifier or an attribute group written before an attribute declaration is `E0133`, the code a
  `type` alias's decoration gets, and its card now names both. An attribute on a declaration is the
  declined meta-attribute.
- A missing `on` still starts the declaration when `{` follows the name, so it is one `E0141` and
  not a cascade. An unknown site word is left out of `sites`, and the declaration is kept.
- `nvs_hir::SymbolKind::Attribute` is declared by the resolver at file scope
  (`crates/nvs-hir/src/resolve.rs:166`), so two declarations of one name are the ordinary duplicate
  error. `nvs_hir::imports::candidates` leaves it out, since it is not a type. The editor's
  `DeclKind::Attribute` is `Struct` in the outline, and `nvs/fileKinds` does not list it until stage 7
  gives the client a word for it.
- The formatter reads the body as a `ShapeType` node (`crates/nvs-syntax/src/walk.rs` `push_shape`),
  so a body is laid out exactly as a shape is. `crates/nvs-fmt/src/gap.rs` gives the header's words one
  space each. A member attribute declaration does not count as a member with a body for the blank-line
  rule, so two one-line declarations may stand together.

**The goal after this one.** `attribute-property-names` builds on this goal: `static` in an attribute
declaration's fields names the class the attribute is used on, and no top-level `type` alias changes
what `static` means.

**What stops the goal.** One attempt per item, two sessions per stage. An item still red after its
attempt goes into the handoff with its cause, and the stage moves on to its other items. A stage still
open after two sessions stops the loop with `BLOCKED` naming the item.

## Stages

### 1. the rules are written, and the cases that pin them are written ahead

**Does:** writes every rule this goal changes, then the conformance cases the `cases` and `retrieval`
checks name. The cases cannot pass yet, and their `--EXPECT--` is frozen once written.

**Split:** one writing subagent writes the eleven `.nvst` cases from the finished rule fragments while
the session renders and checks the rules. Their files are disjoint.

- [x] `docs/rules/attributes/an-attribute-is-declared-with-its-sites.md:1`: the new rule and its
  `data/rules/attributes/` JSON, from § *Standing decisions*: the declaration, the site words, the
  sites a user's declaration may list, the owner rule, `repeatable`, resolution and the Declined list.
- [x] `docs/rules/attributes/inert-metadata.md:1`, `attach-sites-and-forms.md:1`,
  `structural-retrieval.md:1` (renamed to a declaration match, with its JSON id),
  `repeatable.md:1`, `retrieval-folds-while-checking.md:1`, `access-payload.md:1`: rewritten so each
  states the present rule. The bare form and the alias form leave every fragment.
- [x] `docs/rules/routing/a-match-reads-an-attribute-as-data.md:1`, `docs/rules/programs/implementing-with.md:1`:
  `T` names a declaration.
- [x] `docs/rules/types/shape-type.md:1`: a field may carry a doc comment, and the editor shows it.
- [x] `bun nv rules --render`, then `bun nv rules --check` is green.
- [x] The nine cases of the `cases` check and the two of the `retrieval` check, under
  `tests/conformance/lang/`, `tests/conformance/reject/` and `tests/conformance/core/`, each named in
  the front matter. Neutral names only (`Shop`, `Blog`, `Framework`). Each reject case is one
  diagnostic per mistake, and the `lang` cases print what a retrieval returns.

### 2. the parser reads an attribute declaration and a doc comment on a shape field, and the formatter writes both

**Does:** the syntax. Nothing is checked yet.

- [x] `crates/nvs-syntax/src/ast.rs:2189`: an `AttributeDecl` beside `TypeAliasDecl`, with its name,
  `repeatable`, its site words with their spans, its fields as `ShapeField`s and its doc; a
  `StmtKind` arm and a `ClassMemberKind` arm (`:1982`). `ShapeField` (`:276`) gains `doc: Option<Span>`.
- [x] `crates/nvs-syntax/src/parser/decl.rs:874`: `at_attribute_decl` beside `at_type_alias`, the
  declaration at namespace level (`:860`) and in a class or interface body (`:1226`). A site word that is
  not one of the nine is one error naming the nine. An empty list is one error.
- [x] `crates/nvs-syntax/src/parser/ty.rs:866` `parse_shape_type`: a `///` comment before a field is
  that field's doc, and the attribute body reuses this field list.
- [x] `crates/nvs-syntax/src/parser/mod.rs:168` `parse_attribute`: the bare `#[{…}]` is one error that
  says to declare an attribute, and the group's recovery (`:132`) keeps the declaration after it.
  `E0140`, `E0141` and `E0142` go in `crates/nvs-diagnostics/src/code/parser.rs`, and a parser test in
  `crates/nvs-syntax/src/parser/tests/decl.rs` pins each, since a parse error stops a reject case
  before its checker errors.
- [x] `crates/nvs-fmt/src/`: the declaration is formatted as a class body is, one field per line with
  its doc above, and the site list on the header line. `bun nv nvs-fmt` stays green on the corpus.
- [x] `crates/nvs-hir/src/autoload.rs:1538`: an attribute declaration is a symbol of its own kind, so an
  autoloaded file that declares one is found by its name.
- [x] `cargo build` and `cargo clippy --all-targets -- -D warnings` are green.

### 3. the checker resolves an attribute to its declaration and checks its site, its owner, its count and its payload

**Does:** the keystone. After it, a shape alias after `#[` does not compile, so the corpus is red until
stage 6. That is expected, and no stage before the gate runs the whole suite.

- [ ] `crates/nvs-hir/src/aliases.rs:229`: a table of attribute declarations beside the alias table,
  namespace-level and member ones, each with its owner, its sites, `repeatable` and its fields lowered
  once, as `resolve_member_shape_alias` lowers a member alias today.
- [ ] `crates/nvs-types/src/attributes.rs:655` `resolve_shape_alias` and `:709`
  `resolve_member_shape_alias`: replaced by one resolution to a declaration, with the owner chain's
  short names through `nvs_hir::ClassGraph` and the two-scopes error. A class or alias is the
  "only an attribute declaration" error `E1005` with its fix, and `E0726` at `:744` is retired.
- [ ] `crates/nvs-types/src/attributes.rs:242` `check_attribute`: the four checks of § *Standing
  decisions* in their order, with the codes § *The diagnostic codes* names: the new ones in
  `crates/nvs-diagnostics/src/code/types_e10.rs`, and each retired one marked retired in its file. Every
  message is end-user text (`docs/agent/prose.md`), and the frozen cases already state each one.
- [ ] `crates/nvs-types/src/attributes.rs:115` `reject_unread`: retired. Its check is at the
  declaration, for a user's site list.
- [ ] `crates/nvs-types/src/attributes.rs:50` `check_declaration`: the declaration itself is checked,
  its fields as a shape type is, with a duplicate field and a duplicate site word each one error.
- [ ] The `cases` check runs once.

### 4. Core's compiler attributes are declarations, and one site error covers them all

**Does:** one mechanism. `Core`'s attributes are the same declaration value as a program's.

- [ ] `crates/nvs-types/src/derive.rs:75` `ATTRIBUTES`, `:150` `sites`, `:180` `AttributeDoc` and `:258`
  `ATTRIBUTE_DOCS`: one table of declaration values with sites, `repeatable`, fields and their docs,
  read by the checker through the same lookup as a program's declaration.
- [ ] `crates/nvs-types/src/attributes.rs:335` `check_recognized` and `:144` `check_recognized_sites`:
  the site, count and payload checks are stage 3's. `E0874` (`:192`) and `E0763` become stage 3's
  codes. What remains is each recognizing pass's own check.
- [ ] `crates/nvs-lsp/src/stubs.rs:754` `attribute_stub`: writes `attribute Name on … { /// … }`, and
  the coverage check at `:899` still names every row.
- [ ] `cargo build` and `cargo clippy --all-targets -- -D warnings` are green, then the stub coverage
  test in `crates/nvs-lsp/src/stubs.rs:899` runs once, by name, with `cargo test --lib -- stubs::`.

### 5. every retrieval matches the declaration, never the shape

- [ ] `crates/nvs-types/src/retrieval.rs:306` `fold_retrieval`, `:485` `sites_for` and `:561`
  `matching`: the type argument resolves to a declaration, and the answer is the payloads written
  under it. An inline shape or a type is an error.
- [ ] `crates/nvs-types/src/match_attribute.rs:1` and `crates/nvs-types/src/program.rs:58`: the same for
  `Core\Router\Match::attribute<T>` and `Core\Program::implementingWith<I, T>`.
- [ ] `crates/nvs-codegen/src/lib.rs:2375` `attribute_desc`: the row carries the declaration's full
  name, so `crates/nvs-stdlib/src/reflect.rs:2711` `AttributeInfo::name()` returns it.
- [ ] The `retrieval` check runs once.

### 6. the corpus declares its attributes

**Does:** every `.nvs`, `.nvst` and `.lspt` file in the tree compiles under the new rule. About 66 files
use a shape alias as an attribute, 59 use the bare form, 3 use `Owner::Name`, and about 60 call a
retrieval (the authoring session's count over tracked files).

**Split:** an `Explore` subagent lists the files first, grouped by top directory. Writing subagents
take one directory each, side by side: `docs/examples/`, `tests/conformance/`, `tests/hostile/` with
`benches/`, and `tests/lsp/` with `website/snippets/` and `extensions/`. None of them builds; the
session builds once they all end and runs `bun nv nvs-fmt`. A case whose expected output pinned a
removed rule, such as a structural match across two names, is rewritten to pin the new rule, and its
commit says why.

- [ ] Each alias used as an attribute becomes an `attribute` declaration with the sites its uses need,
  moved into the class it is meant for where the example has one. Each bare form becomes a declared
  attribute. Each inline-shape retrieval names a declaration.
- [ ] Every comment the change touches in a published `.nvs` file follows `AGENTS.md` § *Text an end
  user reads*, and `bun nv proofs --comments` passes on each touched directory.
- [ ] `bun nv suite tests/conformance/` runs once over the touched cases, by name, and each is green.

### 7. the editor offers only the attributes that fit, and shows each key's doc

**Split:** one writing subagent writes the six `.lspt` cases of the `editor` check while the session
changes `crates/nvs-lsp/`. Their files are disjoint.

- [ ] `crates/nvs-lsp/src/completion.rs:2856`: after `#[`, the items are the attribute declarations
  whose sites include this declaration's kind and whose owner fits this class, `Core`'s included, each
  with its doc. No class and no alias is offered.
- [ ] `crates/nvs-lsp/src/completion.rs:1438` `attribute_key_at` and `:1477` `shape_keys`: one path for
  every declaration, and each key item carries its field's doc. A key in any object literal at a shape
  type shows its doc too.
- [ ] After `on` in a declaration, the nine site words, or the four a user's declaration may list.
- [ ] `crates/nvs-lsp/src/hover.rs` and `crates/nvs-lsp/src/card.rs:113`: an attribute name shows its
  declaration's doc, its sites and its owner. A key shows its doc.
- [ ] `crates/nvs-lsp/src/definition.rs:822` `attribute_at`: jumps to the declaration. Find references
  on a declaration lists its uses and its retrievals.
- [ ] `crates/nvs-lsp/src/semantic.rs`: an attribute name is its own token kind, not a type.
- [ ] `editors/vscode/syntaxes/nvs.tmLanguage.json`: `attribute`, `repeatable` and `on` in a declaration
  header are keywords.
- [ ] Every existing `.lspt` case under `tests/lsp/` that writes a shape alias as an attribute was
  migrated in stage 6. Its expected list changes only where this stage changed the answer.
- [ ] The `editor` check runs once.

### 8. the feature's proofs, the reference and the website say what attributes are now

**Does:** the user pages. Every line is text an end user reads, held to `AGENTS.md` § *Text an end
user reads*. Run `bun nv proofs --id 'lang:attributes/an-attribute-is-declared-with-the-places-it-may-be-used'`
first and last.

**Split:** writing subagents take the examples, the attack and the bench while the session writes the
reference, `about.md` and the pages. Their files are disjoint.

- [ ] `docs/reference/lang/90-attributes.md:10`: the first section becomes "An attribute is declared
  with the places it may be used", which renames the feature. Its directory under
  `docs/examples/lang/attributes/` is renamed to match, and `docs/examples/lang/attributes/chapter.md:1`
  follows. The other sections are reread and rewritten where they describe the bare form, aliases or
  the shape match.
- [ ] That feature's `about.md` and `details.md`, its examples (a top-level attribute, an attribute owned
  by a base class, `repeatable`, a documented field), one attack (an attribute written on a class that
  does not extend its owner, and on a site its declaration does not list; neither compiles) and its
  bench, under `benches/members/lang/attributes/`.
- [ ] `data/reference/lang/90-attributes.json`, `data/reference/core/Attributes.json` and the help text
  `nvs` prints for attributes, if it has any.
- [ ] `website/src/content/docs/syntax/attributes/*.mdx`: reread, rewritten where needed, and stamped
  with `bun nv site --stamp`.
- [ ] `website/src/data/changes.json`: a `changed` entry for attributes (declarations, sites, owners,
  `repeatable`, the declaration match, the bare form removed) and a `new` entry for doc comments on
  shape fields.
- [ ] The `proofs` check runs once, and `bun nv site --check stale` and `changes` are green.

### 9. the goal is verified once, and every failure is fixed

- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.
- [ ] The ten rule records `designed` under this goal are set `shipped`, and their `goal` key removed,
  then `bun nv rules --render` and `--check`.

## Handoff

### State

Stages 1 and 2 are done. The parser reads an attribute declaration at file scope and in a class or
interface body (`crates/nvs-syntax/src/ast.rs:2225` `AttributeDecl`,
`crates/nvs-syntax/src/parser/decl.rs:957` `at_attribute_decl`), its body through
`crates/nvs-syntax/src/parser/ty.rs:881` `parse_shape_fields`, and a `///` doc on every shape field.
`E0140` to `E0142` are pinned by seven green tests in `crates/nvs-syntax/src/parser/tests/decl.rs`.
`SymbolKind::Attribute` is declared and autoloaded, and `nvs fmt` writes the declaration. Build,
clippy and `bun nv nvs-fmt` are green. Stage 2's calls are in § *Standing decisions* § *The syntax
tree*. The checker still ignores a declaration, so `#[Name]` naming one is unresolved until stage 3.

**Expected red until stage 6: `#[{…}]` is now `E0140`.** About 55 corpus files fail to parse, under
`tests/conformance/`, `tests/hostile/`, `docs/examples/`, `benches/members/` and two
`tests/lsp/` cases. `Grep` for `#\[\{` lists them. The Rust tests that write the bare form go red
too: `crates/nvs-types/tests/attributes.rs`, `crates/nvs-codegen/tests/probes.rs:220`
`A_FOLDED_CALL`, and the test sources in `crates/nvs-types/src/` `attributes`, `derive`,
`match_attribute`, `program` and `retrieval`. The nine `cases` and two `retrieval` cases stay red
until stages 3 and 5.

### Next

Stage 3 from its first item: the table of attribute declarations beside the alias table at
`crates/nvs-hir/src/aliases.rs:229` (top-level from `StmtKind::AttributeDecl`, owned ones from
`ClassMemberKind::Attribute` keyed as `QName::member` keys an owned alias), then
`crates/nvs-types/src/attributes.rs:655` `resolve_shape_alias` and `:709` resolving to a declaration,
`:242` `check_attribute`'s four checks in order, `:115` `reject_unread` retired for `E1004` at the
declaration, and `:50` `check_declaration`. Rewrite the Rust tests named above to declarations in the
same pass. A build and clippy check it, and the `cases` check runs once.

### Backlog
