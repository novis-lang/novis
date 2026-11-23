---
looper: 1
title: an attribute declaration is as well supported in the editor as a class
summary: "Reached when an attribute declaration's site words and fields are coloured, a file that declares one attribute shows `A` in the Explorer, the outline lists every declaration, its body folds, `nvs code order` puts it with the enum and type declarations, `attribute` and `repeatable` and a field's type are completed, a field key reaches its declaration by definition, references, highlight and rename, `#[Name(` shows signature help, an undeclared attribute name has a quick fix that declares it, and each has an .lspt case or a grammar or client test."
context: [crates/nvs-lsp/src/attribute_decls.rs, docs/rules/attributes/an-attribute-is-declared-with-its-sites.md, docs/rules/attributes/attach-sites-and-forms.md, crates/nvs-syntax/src/ast.rs]
stages:
  - id: 1
    title: "the site words and fields are coloured, and a one-attribute file shows A"
    status: done
    context: [editors/vscode/syntaxes/nvs.tmLanguage.json, crates/nvs-lsp/src/semantic.rs, crates/nvs-lsp/src/file_kinds.rs, editors/vscode/src/kinds.ts, docs/rules/ide/a-file-shows-what-it-declares.md]
  - id: 2
    title: "the outline, folding, member order and fmt know a declaration"
    status: done
    context: [crates/nvs-lsp/src/symbols.rs, crates/nvs-lsp/src/folding.rs, crates/nvs-lsp/src/order.rs, docs/rules/tooling/class-members-have-one-order.md]
  - id: 3
    title: "attribute, repeatable and a field's type are completed"
    status: done
    context: [crates/nvs-lsp/src/completion.rs, docs/rules/ide/keywords-are-offered-where-the-compiler-accepts-them.md]
  - id: 4
    title: "a field key reaches its declared field"
    status: done
    context: [crates/nvs-lsp/src/definition.rs, crates/nvs-lsp/src/rename.rs, crates/nvs-lsp/src/hover.rs]
  - id: 5
    title: "signature help in #[Name( and the quick fix that declares an attribute"
    status: done
    context: [crates/nvs-lsp/src/server.rs, crates/nvs-lsp/src/actions.rs, crates/nvs-lsp/src/generate.rs, crates/nvs-types/src/attributes.rs]
  - id: 6
    title: "the feature's proofs, pages and change entries"
    status: done
  - id: 7
    title: "the goal is verified once, and every failure is fixed"
    status: done
    gate: true
checks:
  - id: proofs
    stage: 6
    name: every feature the goal's work reached has its proofs
    run: [bun, nv, proofs, --gate, --goal, attribute-editor-support]
    timeout: 1800
  - id: close
    stage: 7
    name: "the goal-end check is green: ci --check's checks of the tree's files, the owners and the goal's proofs"
    run: [bun, nv, close, --goal, attribute-editor-support]
    timeout: 900
---

# an attribute declaration is as well supported in the editor as a class

## Why

`attribute-declarations` added `[repeatable] attribute Name on <site>, … { field: T, opt?: T = v }` at
file scope and in a class or interface body. The editor support that landed with it covers the name:
it is a `decorator` token, `#[` offers only the declarations that fit, and hover, go to definition,
references, highlight and rename work on the name. A read of the editor tooling on 2026-11-23 found
what the declaration form still lacks. Each gap below is checked against the code:

- The TextMate grammar colours `repeatable`, `attribute`, the name and `on`, and leaves the site words
  and the body to `#code` (`editors/vscode/syntaxes/nvs.tmLanguage.json:304`). The user reported the
  uncoloured `on method` as a bug.
- The Explorer badge maps an attribute declaration to no letter (`crates/nvs-lsp/src/file_kinds.rs:109`),
  while an autoloaded file may declare exactly one attribute (`crates/nvs-hir/src/autoload.rs:1539`).
- The outline has no arm for a declaration, at file scope (`crates/nvs-lsp/src/symbols.rs:135`) or in
  a class (`symbols.rs:233`). Workspace symbols do find them.
- Folding has no arm for either (`crates/nvs-lsp/src/folding.rs:258`, `:283`).
- Member order puts a class-scoped declaration after the methods (`crates/nvs-lsp/src/order.rs:455`).
- `STATEMENT_WORDS` (`crates/nvs-lsp/src/completion.rs:2793`) and `MEMBER_WORDS` (`:2896`) have no
  `attribute` or `repeatable`, and nothing completes a type inside the declaration's body.
- A field key in `#[Name(key: …)]` completes and hovers, but no definition, references, highlight or
  rename reaches the declared field.
- Signature help (`crates/nvs-lsp/src/server.rs:2938`) answers calls only, not `#[Name(`.
- No quick fix declares an attribute that a `#[Name]` uses and nothing declares.
- No `nvs fmt` case holds a declaration. Only the whole-corpus run formats one.

## Standing decisions

**The grammar colours four site words.** After `on`, only `class`, `method`, `property` and
`parameter` get `keyword.other.nvs`, the scope of `on`, up to the `{` or the end of the line. The other
five words the parser reads are `E1004` in a program (`crates/nvs-diagnostics/src/code/types_e10.rs:55`),
so they get no colour (`rule:ide/rejected-syntax-gets-no-colour`). A body field's name is
`variable.other.property.nvs`, the scope a payload key already has in `#attribute-arguments`. Reason:
the same field reads the same at its declaration and at its use. The grammar change is re-rendered into
`website/config/novis.tmLanguage.json` by the site's sync, never edited there.

**The server colours a body field as a `property`**, as a class property's declaration is, and gives
its type no token, as a property's type gets none. The legend has no `declaration` modifier, and a
class property's declaration carries none. Site words get no server token: the grammar is their one
home.

**The Explorer letter is `A`.** The wire word is `attribute`, the tooltip "This file declares an
attribute." An older client ignores the word (`isKind`, `editors/vscode/src/kinds.ts:57`), so the
server change needs no version check. `R` still wins over every kind.

**A declaration is in member order group one**, beside `enum` and `type` declarations, in written order.
Reason: all three declare a type-level name the members below may use. `fmt` still never reorders
(`rule:tooling/fmt-never-reorders-members`); only `nvs code order` and the editor action move it.

**The keywords are offered where the parser accepts them.** `attribute` and `repeatable` at file scope
and in a class or interface body, never in an enum body (`E0142`). With a client that takes snippets,
`attribute` inserts `attribute ${1:Name} on ${2|class,method,property,parameter|} { $0 }`, and
`repeatable` inserts the same after `repeatable `. Without snippets, the word. The extension gets no
`snippets` contribution: the server's completion works in every client and is the one home.

**A field type is completed with the list a property's type gets.** No narrower list: every type a
property may have is a type a payload field may have, and a type the checker rejects there is the
checker's to report.

**Field navigation covers three places.** The declared field, every key in a `#[Name(key: …)]` use, and
every read of the field on a retrieval's result that the checker types against the declaration's shape.
Stage 4's first item finds what the checker records for that read. **Safe default:** if a read is not
recorded and cannot be found from the expression table, rename refuses a field that has such a read,
and references lists the declaration and the uses. A partial rename is worse than none.

**Signature help shows the declaration as one signature**, `Name(field: T, opt?: T)`, with each
field's `///` doc as the parameter doc. The active field is the key at the cursor, and before any key
is written, the first required field not yet written.

**The quick fix declares the attribute in the same file.** On the diagnostic for a `#[Name]` that
nothing declares, *Declare the attribute* writes `attribute Name on <site> { … }` at file scope, after
the `namespace` and `use` lines. The site is the one the use is on. Each field is a key the use writes,
typed as its value's type, and `string` for a value whose type the checker does not record
(`rule:ide/a-code-action-writes-only-what-is-already-determined`). It is not offered in an autoloaded
file, where a second declaration is `E0317`, and not for a qualified or member-form name.
As built: the checker records no type for a payload value of an undeclared attribute, so the type is
read off the value's syntax (`bool`, `int`, `float`, `string`, `array<T>`). "An autoloaded file" is a
file with the autoloaded shape (`nvs_hir::autoload::sole_declaration`): a second declaration breaks
that shape whether or not a program autoloads it today. Only `class`, `method`, `property` and
`parameter` uses are offered, since any other site is `E1004` in a declaration. The edit is analysed
first, as every generator's is.

**No quick fix adds a missing required field.** The value is not determined, so the rule above does not
allow writing one. Key completion and signature help show what is missing.

**The change entries name `tools:editor/nvs-lsp`.** `site --check changes` accepts an entry only for
a feature whose `covers:`-marked tests moved, and it reads markers in `.nvst` cases and Rust tests
alone. The grammar and Explorer-letter tests are TypeScript, and the `.lspt` cases carry no marker,
so `tools:editor/the-vs-code-extension` and `tools:cli/nvs-code` did not move. The server sends the
`attribute` file kind, the field tokens and the member order, so `nvs-lsp` is a true owner of every
entry. Teaching the proofs tool to read TypeScript markers would change what counts as a Rust proof,
a wider change than this goal.

**An ISO year has one syntax: four digits, with `-` before a negative year.** The full gate found
that `Instant::toIso` wrote the earliest instant as `-009999-…`, which `fromIso` rejects since
review-hardening. The writer now writes `-9999-…`, so `fromIso(toIso())` always gives the same
value. Reason: one way to write a year is simpler, and the reader stays strict.

**Lanes.** Stages 1 and 2 touch disjoint files and may run as side goals. Stages 3, 4 and 5 all edit
`crates/nvs-lsp/src/completion.rs` or `server.rs` and run in order. Stage 6 follows them all.

## Stages

### 1. the site words and fields are coloured, and a one-attribute file shows A

**Does:** closes the two visible gaps the user reported: the colour after `on`, and the Explorer letter.

**Split:** none; the files are few and each change is small.

- [x] `editors/vscode/syntaxes/nvs.tmLanguage.json:304`: the header rule colours the four site words
  and the commas between them, and a body field's name, per the standing decision. The rule's comment
  says what it colours now.
- [x] `editors/vscode/test/grammar/fixtures/attributes.nvs:14` gains a declaration with two site words, a
  multi-line body and an optional field. `editors/vscode/test/grammar/comments.test.ts:150`
  asserts each scope, and that `hook` after `on` gets none.
- [x] `crates/nvs-lsp/src/semantic.rs:739` and `:791`: a body field is a `property`, and its type gets
  no token, as a property's. `tests/lsp/semantic/an-attribute-name-is-a-decorator-token.lspt:1`
  expects the new tokens.
- [x] `crates/nvs-lsp/src/file_kinds.rs:109`: `DeclKind::Attribute` answers `attribute`; its doc comment
  says so. `editors/vscode/src/kinds.ts:23` and `:42` add the kind and the `A` badge, and
  `editors/vscode/test/client/kinds.test.ts:1` covers it.
- [x] `rule:ide/a-file-shows-what-it-declares` and `rule:programs/one-declaration-per-autoloaded-file`
  list an attribute declaration among the kinds.

### 2. the outline, folding, member order and fmt know a declaration

**Does:** makes every walker over declarations and class members handle the declaration form.

**Split:** the four `.lspt` cases and the two fmt cases go to one writing subagent once the code lands.

- [x] `crates/nvs-lsp/src/symbols.rs:135` and `:233`: a declaration is a document symbol at file scope
  and as a class member, of the kind the workspace symbols already give it
  (`crates/nvs-lsp/src/server.rs:2076`). Its fields are its children.
- [x] `crates/nvs-lsp/src/folding.rs:258` and `:283`: a body that spans lines folds.
- [x] `crates/nvs-lsp/src/order.rs:445`: a declaration is in group one with `enum` and `type`;
  `rule:tooling/class-members-have-one-order` says so in its step 1.
- [x] `.lspt` cases: `tests/lsp/symbols/an-attribute-declaration-is-in-the-outline.lspt:1`,
  `tests/lsp/folding/an-attribute-declarations-body-folds.lspt:1`, and an order case beside the existing
  ones under `tests/lsp/actions/`.
- [x] An `nvs fmt` case, `tests/fmt/input/attribute-declarations.nvs:1` and its
  `tests/fmt/formatted/attribute-declarations.nvs:1`: a one-line declaration, a multi-line body with a
  doc comment on a field, and a class-scoped one, each written with spacing `fmt` corrects.
- [x] A sweep with `Explore`: every `match` on `StmtKind` or `ClassMemberKind` under `crates/nvs-lsp/src/`
  with a `_ =>` arm. Each that drops a declaration where a class or enum is handled gets an arm, or a
  line under `### State` saying why it needs none.

### 3. attribute, repeatable and a field's type are completed

**Does:** completion offers the declaration's keywords and the types inside its body.

- [x] `crates/nvs-lsp/src/completion.rs:2793` and `:2896`: `attribute` and `repeatable` are offered at
  file scope and in a class or interface body, with the snippet the standing decision gives. Not in an
  enum body (`CASE_WORDS`, `:2915`).
- [x] Inside a declaration's `{ … }`, after `name:` or `name?:`, completion offers the list a property's
  type position gets.
- [x] `rule:ide/keywords-are-offered-where-the-compiler-accepts-them` names the two words and where each is offered.
- [x] `.lspt` cases under `tests/lsp/completion/`: the keywords at file scope and in a class, none in an
  enum, the snippet form and the plain form, and a field type.

### 4. a field key reaches its declared field

**Does:** definition, references, highlight and rename work on a field as they do on the name.

- [x] Find what the checker records for a read of a field on a retrieval's result
  (`Core\Attributes::get` and the others, `crates/nvs-types/src/attributes.rs`). Write the finding and
  the decision it leads to under `### State`.
- [x] `crates/nvs-lsp/src/definition.rs:561`: from a key in `#[Name(key: …)]`, and from a recorded read,
  go to definition reaches the declared field.
- [x] References and document highlight on a declared field list the field, every key and every
  recorded read.
- [x] `crates/nvs-lsp/src/rename.rs`: rename of a field changes all three, or refuses as the standing
  decision says. A `Core` attribute's field is refused, as its name is.
- [x] Hover on a field in the declaration's body shows its type and `///` doc, as hover on a key in a
  use already does (`crates/nvs-lsp/src/hover.rs:858`).
- [x] `.lspt` cases under `tests/lsp/definition/`, `references/`, `highlight/`, `rename/` and `hover/`, one each.

### 5. signature help in #[Name( and the quick fix that declares an attribute

**Does:** the two code-writing aids for using an attribute.

- [x] `crates/nvs-lsp/src/server.rs:2938`: inside `#[Name(`, signature help answers the declaration as the
  standing decision says. A `Core` attribute answers from its recorded declaration.
- [x] Find the code the checker reports for a `#[Name]` that nothing declares (the lookup ends at
  `crates/nvs-types/src/attributes.rs:464`). The quick fix *Declare the attribute* is offered on it, written
  as `generate.rs`'s *Declare the method you called* is.
- [x] `rule:ide/a-code-action-writes-only-what-is-already-determined` lists the new action and what it writes.
- [x] `.lspt` cases: signature help with no key written, at a key and after the last key; the quick
  fix in a script file, its absence in an autoloaded file, and its field types.

### 6. the feature's proofs, pages and change entries

**Does:** pays what the change owes the website.

- [x] `website/src/content/docs/tools/editor/index.mdx` names the `A` badge, the outline and folding of a
  declaration, the keyword completion, signature help in `#[Name(` and the quick fix. Reread and
  stamped with `bun nv site --stamp`.
- [x] `website/src/data/changes.json` entries: `new` for the badge, the quick fix and signature help;
  `changed` for colouring, the outline, folding, completion, field navigation and member order. The
  member-order entry says that `nvs code order` now moves a class-scoped declaration to the top.
- [x] `bun nv proofs --owed --goal attribute-editor-support` names nothing, and
  `bun nv site --check stale changes review` is green.

### 7. the goal is verified once, and every failure is fixed

- [x] The newcomer review of [prose.md](../agent/prose.md) § *A newcomer reads it* over the texts
  stage 6 wrote.
- [x] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [x] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

All seven stages are done. The full gate's third `looper verify` was green over everything since
the last full gate, taint-fail-closed, windows-rust-lld, review-hardening and csrf-cookieless-visitor
included. Its fixes: `signatureHelp` `.lspt` cases and fixtures (`test(lsp)`), tests that pin what
earlier goals shipped (`test`), `Instant::toIso` writing `-9999-…` (`fix(stdlib)`, standing
decision *An ISO year has one syntax*), and the editor page's snippet (`docs(site)`).

The first DONE was refused: `floor-debt` found two `Novis-Rest-Session` examples red. They called
`Core\Test::answerHttp` beside a `.nvsr`, and review-hardening made it throw in a request.
`fix(examples)` builds the token directly in `01-*` and makes `03-*` a `proof: test` program whose
`Core\Test::request` fakes the reply. `bun nv proofs --run --only 'Novis\Rest\Session'` is green,
and the feature's `about.md` and help are restamped. No test is expected to go red.

The uncommitted `docs/rules/tooling.md` hunk is not this goal's.

### Next

The goal is reached. The next goal on the chain is `exact-invalidation`.

### Backlog

Two items `review-hardening` left unreproduced, carried here from `csrf-cookieless-visitor` so they
stay visible. Neither is this goal's work.

- A release of the last reference to a suspended generator runs its `finally` bodies
  (`crates/nvs-runtime/src/object.rs:5126`, `unwind_abandoned`). An `InstKind::Release` counts as no
  program code (`rule:types/a-replaced-value-is-changed-in-place` § *What a quiet call is*), so a
  lend may cross a release that runs a `finally` which writes the lent slot. Not reproduced.
- A class constant's initializer, `public const int N = Framework\Base::LIMIT;`, is folded in the
  signatures pass and records no `ExprInfo`, so a restricted constant there is probably not checked.
  Not reproduced.
- A shape type's own field name, `note` in `{owner: Customer, note?: string}`, gets no definition and
  no highlight, and prepareRename refuses it. An attribute declaration's fields have both. Found while
  writing the `ShapeType` coverage cases. No case pins it.
