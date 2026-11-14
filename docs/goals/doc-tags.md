---
looper: 1
title: "a doc comment describes each parameter, the return value and the errors"
summary: "Reached when @param, @return and @throws are checked doc comment tags that never repeat a type, the editor completes, fixes, renames and shows them, nvs meta --json and nvs doc print them in the Core card's shape, and the feature has its proofs."
context: [docs/rules/tooling/doc-comment-tags-are-closed-and-checked.md, crates/nvs-syntax/src/ast.rs, crates/nvs-hir/src/members.rs]
stages:
  - id: 1
    title: the rule and the parser know five tags, and a tag never carries a type
    status: done
    context: [crates/nvs-syntax/src/parser/mod.rs, crates/nvs-diagnostics/src/code/parser.rs, crates/nvs-syntax/tests/doc_comments.rs, docs/rules/tooling/strict-docs.md, tests/conformance/syntax/doc-comment/a-tag-outside-the-five-is-a-diagnostic.nvst]
  - id: 2
    title: the checker holds every tag to the signature, and each mistake carries its fix
    status: done
    context: [crates/nvs-diagnostics/src/code/names.rs, crates/nvs-diagnostics/src/diagnostic.rs, docs/rules/ide/a-code-action-ships-only-a-fix-a-diagnostic-already-knows.md, docs/rules/ide/a-quick-fix-is-a-diagnostics-own-suggestion.md]
  - id: 3
    title: meta, doc, hover and signature help show the tags in the Core card's shape
    status: done
    context: [crates/nvs-cli/src/meta.rs, crates/nvs-cli/src/doc.rs, crates/nvs-lsp/src/hover.rs, crates/nvs-stdlib/src/registry.rs, crates/nvs-cli/tests/meta.rs, crates/nvs-cli/tests/doc.rs, crates/nvs-lsp/tests/navigation.rs, docs/rules/tooling/meta-json-takes-a-program.md, docs/rules/core-api/reference-card.md]
  - id: 4
    title: the editor completes, renames, finds and colours the tags
    status: done
    context: [crates/nvs-lsp/src/completion.rs, crates/nvs-lsp/src/rename.rs, crates/nvs-lsp/src/query.rs, crates/nvs-lsp/src/server.rs, crates/nvs-lsp/src/actions.rs, editors/vscode/syntaxes/nvs.tmLanguage.json, editors/vscode/test/grammar/comments.test.ts, tests/lsp/README.md, docs/rules/ide/completion-offers-only-what-the-compiler-derived.md]
  - id: 5
    title: the feature's proofs and its pages say what the five tags do
    status: done
    context: [docs/examples/lang/programs/doc-comments/about.md, docs/examples/lang/programs/doc-comments/details.md, docs/reference/lang/10-programs.md, docs/reference/tools/40-editor.md, docs/reference/tools/35-packages.md]
  - id: 6
    title: nvs fmt puts the tags in one order
    status: open
    context: [crates/nvs-fmt/src/lib.rs, crates/nvs-fmt/src/imports.rs, crates/nvs-fmt/src/tokens.rs, crates/nvs-fmt/tests/identity.rs, docs/rules/tooling/fmt-sorts-the-use-block.md, docs/rules/tooling/fmt-is-idempotent.md, docs/rules/tooling/fmt-at-most-one-blank-line.md]
  - id: 7
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: tags-parse
    stage: 1
    name: "@param, @return and @throws parse with their targets and text, and a type before a parameter's name, an empty text and @returns are each a diagnostic"
    run: [cargo, test, --test, doc_comments, --, param_return_and_throws_parse_with_their_text, a_type_before_the_parameter_name_is_a_diagnostic, a_tag_with_no_text_is_a_diagnostic, returns_is_spelled_return]
    timeout: 1800
    tests:
      - param_return_and_throws_parse_with_their_text
      - a_type_before_the_parameter_name_is_a_diagnostic
      - a_tag_with_no_text_is_a_diagnostic
      - returns_is_spelled_return
  - id: tags-checked
    stage: 2
    name: "the checker holds each tag to the signature: an unknown parameter, a repeated one, a declared type repeated, @return on void, never or a constructor, @throws naming no Throwable, a tag on a declaration that is not a function, and a missing @param beside documented ones"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/syntax/doc-comment/a-param-that-names-no-parameter-is-a-diagnostic.nvst, --case, tests/conformance/syntax/doc-comment/a-parameter-documented-twice-is-a-diagnostic.nvst, --case, tests/conformance/syntax/doc-comment/a-tag-that-repeats-the-declared-type-is-a-diagnostic.nvst, --case, tests/conformance/syntax/doc-comment/a-return-on-a-function-that-returns-nothing-is-a-diagnostic.nvst, --case, tests/conformance/syntax/doc-comment/a-throws-that-names-no-throwable-is-a-diagnostic.nvst, --case, tests/conformance/syntax/doc-comment/a-param-on-a-declaration-that-is-not-a-function-is-a-diagnostic.nvst, --case, tests/conformance/syntax/doc-comment/an-undocumented-parameter-beside-documented-ones-is-a-warning.nvst]
    timeout: 1800
    tests:
      - tests/conformance/syntax/doc-comment/a-param-that-names-no-parameter-is-a-diagnostic.nvst
      - tests/conformance/syntax/doc-comment/a-parameter-documented-twice-is-a-diagnostic.nvst
      - tests/conformance/syntax/doc-comment/a-tag-that-repeats-the-declared-type-is-a-diagnostic.nvst
      - tests/conformance/syntax/doc-comment/a-return-on-a-function-that-returns-nothing-is-a-diagnostic.nvst
      - tests/conformance/syntax/doc-comment/a-throws-that-names-no-throwable-is-a-diagnostic.nvst
      - tests/conformance/syntax/doc-comment/a-param-on-a-declaration-that-is-not-a-function-is-a-diagnostic.nvst
      - tests/conformance/syntax/doc-comment/an-undocumented-parameter-beside-documented-ones-is-a-warning.nvst
  - id: tags-shown
    stage: 3
    name: "nvs meta --json and nvs doc print a user method's tags in the Core card's shape, in signature order, inherited by position, and signature help shows the active parameter's sentence"
    run: [cargo, test, --test, meta, --test, doc, --test, navigation, --, a_user_method_card_has_the_core_card_shape, an_inherited_param_is_shown_under_the_overriding_name, nvs_doc_renders_params_return_and_errors, signature_help_shows_the_active_parameters_sentence]
    timeout: 1800
    tests:
      - a_user_method_card_has_the_core_card_shape
      - an_inherited_param_is_shown_under_the_overriding_name
      - nvs_doc_renders_params_return_and_errors
      - signature_help_shows_the_active_parameters_sentence
  - id: editor-answers
    stage: 4
    name: "the editor completes each tag and parameter, renames a parameter and its @param from either end, finds the tag as a reference, applies every fix, and shows a parameter's sentence on hover"
    run: [bun, nv, suite, lsp-test, tests/lsp/, --case, tests/lsp/completion/param-offers-the-undocumented-parameters-in-order.lspt, --case, tests/lsp/completion/throws-offers-the-throwable-classes.lspt, --case, tests/lsp/completion/a-property-is-not-offered-param.lspt, --case, tests/lsp/rename/a-parameter-rename-rewrites-its-param-line.lspt, --case, tests/lsp/rename/a-rename-at-the-param-line-renames-the-parameter.lspt, --case, tests/lsp/references/a-param-line-is-a-reference-to-its-parameter.lspt, --case, tests/lsp/actions/a-new-parameter-is-offered-its-param-line.lspt, --case, tests/lsp/actions/a-repeated-type-is-offered-its-deletion.lspt, --case, tests/lsp/actions/a-stale-param-is-offered-the-one-undocumented-name.lspt, --case, tests/lsp/hover/a-parameter-in-the-body-shows-its-param-sentence.lspt]
    timeout: 1800
    tests:
      - tests/lsp/completion/param-offers-the-undocumented-parameters-in-order.lspt
      - tests/lsp/completion/throws-offers-the-throwable-classes.lspt
      - tests/lsp/completion/a-property-is-not-offered-param.lspt
      - tests/lsp/rename/a-parameter-rename-rewrites-its-param-line.lspt
      - tests/lsp/rename/a-rename-at-the-param-line-renames-the-parameter.lspt
      - tests/lsp/references/a-param-line-is-a-reference-to-its-parameter.lspt
      - tests/lsp/actions/a-new-parameter-is-offered-its-param-line.lspt
      - tests/lsp/actions/a-repeated-type-is-offered-its-deletion.lspt
      - tests/lsp/actions/a-stale-param-is-offered-the-one-undocumented-name.lspt
      - tests/lsp/hover/a-parameter-in-the-body-shows-its-param-sentence.lspt
  - id: feature-proofs
    stage: 5
    name: "the doc comments feature owes no proof"
    run: [bun, nv, proofs, --id, "lang:programs/doc-comments"]
    timeout: 900
  - id: fmt-orders-tags
    stage: 6
    name: "nvs fmt writes the prose, one blank line, then @param in signature order, @return, @throws, @see and @example, each with its continuation lines, idempotently, and formats a file whose only errors are doc tag errors"
    run: [cargo, test, --test, doc_tag_order, --, the_tags_follow_the_prose_in_one_order, params_follow_the_signature, an_unknown_param_goes_after_the_known_ones, a_tag_keeps_its_continuation_lines, ordering_the_tags_is_idempotent, a_doc_tag_error_does_not_stop_formatting]
    timeout: 1800
    tests:
      - the_tags_follow_the_prose_in_one_order
      - params_follow_the_signature
      - an_unknown_param_goes_after_the_known_ones
      - a_tag_keeps_its_continuation_lines
      - ordering_the_tags_is_idempotent
      - a_doc_tag_error_does_not_stop_formatting
---

# a doc comment describes each parameter, the return value and the errors

## Why

A `Core` member's reference card describes each parameter, the return value and every error it throws
(`rule:core-api/reference-card`). A user's function could carry only Markdown, `@see` and `@example`. So
a published package always read poorer than `Core` in a hover, in `nvs doc` and in `nvs meta --json`,
and signature help had no sentence to show for any parameter, `Core`'s included.

The person decided on 2026-11-14 to add `@param`, `@return` and `@throws`, under the rule's own
standard: a tag stays only where the compiler checks it. Each of the three is checked against the
signature, none of them ever repeats a type, and the editor treats them as code: it completes them,
renames them with their parameter, finds them as references and fixes them.

Other languages, for the rule's reasons: Java, C#, KDoc and JSDoc have a tag per parameter, and C# and
Java's doclint check its name. PHPDoc repeats the type, which drifts. Swift and Rust use Markdown
conventions, and Dart and Go use checked references in prose. The tag form is the one most programmers
already write, and it is the only one signature help can show per parameter.

## Standing decisions

**The set is five tags and stays closed: `@see`, `@example`, `@param`, `@return`, `@throws`.** Any
other `@tag` at the start of a line is still `E0128`. `@returns` gets its own help and a suggestion
that rewrites it to `@return`. `@var`, `@deprecated`, `@since`, `@internal` and `@inheritdoc` stay
errors for the reasons the rule gives. The rule fragment is renamed from
`tooling/doc-comment-tags-are-see-and-example` to `tooling/doc-comment-tags-are-closed-and-checked`,
with every citation, and its Declined entry *Per-parameter documentation as a tag* is replaced by the
ones below.

**The shapes:**
- `@param $name text`: `$name` comes directly after the tag. A variadic parameter is written `$name`
  too, without `...`.
- `@return text`.
- `@throws Name text`: `Name` resolves like a type name in that file, `use` lines included.
- The text is Markdown and runs to the end of the line. A following `///` line that is not a tag and
  is not blank continues the text of the tag above it, so a long sentence can wrap.
- The text is required. A tag with no text is an error: "write what `$name` means". This is the error a
  newly inserted `@param` line shows until its author writes the sentence.

**A tag never carries a type (the person's call).**
- After `@param`, anything but `$name` is an error: `@param int $x` and JSDoc's `@param {int} $x`. Its
  suggestion deletes the type. The parser raises this one, since it needs no signature.
- If the first word of the text is the declared type, it is an error with a suggestion that deletes the
  word. That covers the parameter's type after `@param $x`, and the function's return type after
  `@return`. The checker raises this one. A union or nullable type counts if the word is the whole
  type or one of its members. `null` alone is never flagged, because "null when the card is declined"
  is a normal sentence. A word that only looks like some other type is never flagged.
- `@throws Name` is not a repeated type. A signature has no `throws` clause.

**Where each tag may go:**
- `@param` and `@throws` go on a function, a method (abstract and interface ones included) or a
  constructor.
- `@return` goes on a function or method whose declared return type is neither `void` nor `never`.
  It never goes on a constructor.
- Anywhere else, each of the three is an error naming the declaration kind. An anonymous function is
  not a declaration and carries no doc comment.

**What the checker holds each tag to.** Every one of these errors carries a `Suggestion`, so the quick
fix is a translation (`rule:ide/a-code-action-ships-only-a-fix-a-diagnostic-already-knows`):
- `@param $name` must name a parameter of this declaration, at most once. If the name is unknown and
  exactly one parameter has no `@param`, the suggestion renames the tag to it. This is the fix after a
  parameter was renamed by hand. Otherwise the suggestion deletes the line.
- A parameter documented twice: the suggestion deletes the second line.
- `@return` written twice, or where it may not go: the suggestion deletes the line.
- `@throws Name` must resolve to a class that is a `Throwable`, and names each class at most once. The
  body is not checked against it. Novis has no checked exceptions, and the tag documents intent.
- **A missing `@param` is a warning only where the same comment already has at least one `@param`.**
  It is C#'s CS1573, and it is what offers a new parameter its line. Its suggestion inserts
  `/// @param $name ` after the `@param` of the parameter before it in signature order, or before the
  first `@param` when it is the first. The inserted line then shows the empty-text error until its
  sentence is written. A comment with no `@param` at all is never warned: nothing requires
  per-parameter documentation, for `rule:tooling/strict-docs`'s reason. A fix never writes a sentence.
- Diagnostic codes: each new parser diagnostic takes the next free `E01xx` in `code/parser.rs`. Each new
  checker diagnostic takes the next free code in `code/names.rs` beside `@see`'s `E0323`. The warning
  takes the next free `W` code in `code/warnings.rs`, since `names.rs` holds only `E03xx` and fails
  its tests on any other code. Each code gets its card.

**`nvs fmt` puts the tags in one order (the person's call, 2026-11-14).** A new rule fragment,
`tooling/fmt-orders-the-doc-tags`, says it:
- The prose comes first. Then one empty `///` line, then the tags with no empty line between them. A
  comment with no prose starts with its first tag.
- The tag order is: `@param` in signature order, then `@return`, `@throws`, `@see` and `@example`.
  `@throws`, `@see` and `@example` keep the order their author wrote within their kind, because that
  order is read back (`nvs meta --json` lists `see` and `example` in written order).
- An `@param` whose name is no parameter goes after the known ones, in written order. The checker
  already reports it.
- A tag moves with its continuation lines. `nvs fmt` never changes the text of a line, only the order
  of the lines and the empty lines between them.
- The signature is read from the syntax tree, from the parameter list of the declaration below the
  comment. No type or name is resolved, so the formatter stays syntactic.
- Order is never a diagnostic (`rule:tooling/fmt-is-never-a-diagnostic`). A file out of order is what
  `nvs fmt --check` reports, as it reports any other layout.
- `rule:tooling/fmt-sorts-the-use-block` says the use block is "the only reordering the formatter
  performs anywhere". That sentence changes to name both. A tag's position is not observable, as an
  import's is not.
- **A doc tag error never stops `nvs fmt`.** The formatter refuses a file with a parse error
  (`crates/nvs-fmt/src/lib.rs:168`). Every doc tag diagnostic leaves the tree whole, so it is let
  through like the errors `tokens::repairs` lets through. Otherwise format-on-save refuses the file
  right after the quick fix inserts an `@param` line with no text yet. `E0128` is let through the same
  way. If a doc tag diagnostic is raised by the checker rather than the parser, it never reaches this
  check, and only the parser's need letting through.

Every reader still shows signature order, so an unformatted file reads the same. A quick fix inserts in
signature order.

**Inheritance.** A member with no `///` of its own inherits the whole comment, as now. An inherited
`@param` is shown under the overriding member's parameter at the same position, because an override
may name its parameters differently. A member with its own comment shows only its own tags.

**A promoted constructor parameter's `@param` is its property's documentation.** Hover on the property,
`nvs doc` and `nvs meta --json` show it there. `--strict-docs` still does not report a promoted
property, as `rule:tooling/strict-docs` says.

**One shape for every card.** A user member's `doc` object in `nvs meta --json` uses the `Core` card's
keys: `short`, `params` as a list of `{name, desc}` in signature order, `return`, `errors` as a list of
`{error, desc}`, then `see` and `example` as now. A key is left out when it is empty, as `doc_json`
does. `nvs doc` renders a user card exactly as it renders a `Core` card. A parameter's `name` has no
`$`, as a `Core` card's has none. `errors` takes `doc_json`'s `error` key, not `name`, because one shape
means the `Core` card's keys exactly. A user method also carries `names`, its parameters without `$`,
as a registry member does; `nvs doc` maps an inherited `@param` by position through the two members'
`names`. A promoted constructor parameter is listed in `members` as a `property` after the constructor.

**The editor.**
- Completion after `@` offers only the tags this declaration accepts: no `@param` above a property, and
  no `@return` above a `void` function.
- Completion after `@param ` offers the parameters not yet documented, in signature order.
- Completion after `@throws ` offers the classes that are a `Throwable`, from the hierarchy walk the
  checker uses (`rule:ide/completion-offers-only-what-the-compiler-derived`).
- Rename works from either end. Renaming a parameter rewrites its `@param` name, and the `@param` line
  of a promoted parameter as well. Renaming at the `$name` of an `@param` line renames the parameter.
  Find references and document highlight include the tag's `$name`, and go to definition from it lands
  on the parameter.
- **A parameter rename stops being refused (decided in stage 4).** Rename refuses every parameter now
  (`crates/nvs-lsp/src/rename.rs:952`), because a call may pass it by name, so the rename above needs
  that refusal gone first. For a parameter of a method, the rename rewrites the body's sites, the
  `@param` name, and the `name:` of each named argument at every call the index holds for that method
  (`SymbolIndex::occurrences`), read off the call's own `Arg::name` in its file's tree. A call the
  rename cannot read refuses the whole rename, as an unindexed type site does. An override keeps its
  own parameter names, because a named argument binds to the method the call resolved to. A
  constructor's parameter is renamed only if the index holds its `new` calls; otherwise it stays
  refused. A promoted parameter stays refused as its property is (`outside` at `rename.rs:476`), and
  its `@param` follows when that refusal goes. `tests/lsp/rename/a-parameter-is-refused.lspt` becomes
  a rename that rewrites the `name:` at its call. Cost: one parse of each file that calls the method,
  per rename request, and nothing kept. Lifting the refusal without rewriting named arguments was
  declined, because it breaks every call that passes the parameter by name
  (`rule:ide/no-refactoring-introduces-an-alias`).
- Signature help sets each `ParameterInformation.documentation`, for a user function from its
  `@param`s, and for a `Core` member from its card's `params`. The note at `hover.rs:302` goes.
- Hover on a parameter inside the body shows its `@param` sentence.
- Hover on a function shows its card in the same layout as a `Core` card.
- The TextMate grammar gives the tag names, a `$name` after `@param` and a class after `@throws`
  scopes of their own inside `comment.line.documentation.nvs`. Semantic tokens mark the `$name` as a
  parameter.

**What it costs.** Nothing at run time, in speed or in memory. The tags are read by the checker, `nvs
meta`, `nvs doc` and the language server. The language gets three more tags, and each one is the
syntax most programmers already know.

**Declined** (goes into the rule fragment):
- **A type in the tag**, as in PHPDoc. The signature is the one place a type is written, and a second
  copy drifts.
- **A warning for every undocumented parameter.** A required tag is answered with generated noise.
  The warning fires only where the author already documented the others.
- **Swift's Markdown list form** (`- Parameter x:`). A reader cannot tell a structured list from an
  ordinary one.
- **Dart's checked references in prose only.** They buy a check, but signature help still has no
  sentence per parameter.
- **A diagnostic for tags out of order.** Layout is `nvs fmt`'s job and never a diagnostic.
- **Moving prose written between tags.** A line after a tag is that tag's text, so there is no prose
  there to move.

**Lanes:** none. Every stage builds on the one before it, and stages 2 to 4 share `members.rs`, the
AST and `hover.rs`. The stages run in order.

## Stages

### 1. the rule and the parser know five tags, and a tag never carries a type

**Does:** rewrites the rule, adds the three kinds to the AST, and parses each with its target and text.
It raises the parser's diagnostics: a type before `$name`, an empty text, and `@returns`.

**Split:** an `Explore` subagent lists every citation of `tooling/doc-comment-tags-are-see-and-example`
across `docs/`, `crates/`, `tools/` and `data/`, as `path:line`.

- [x] `docs/rules/tooling/doc-comment-tags-are-see-and-example.md:1` renamed to
  `doc-comment-tags-are-closed-and-checked.md` and rewritten from § *Standing decisions*, with every
  citation the subagent lists updated. `strict-docs.md:19` says the same about autofixes with the new
  tags, and `bun nv rules --check` is green.
- [x] `crates/nvs-syntax/src/ast.rs:1658`: `DocTagKind` gains `Param`, `Return` and `Throws`, and
  `ALL` lists five. `DocTag` gains `target: Option<Span>` (the `$name` or the class) and `text: Span`,
  and its doc comment says which kind fills which.
- [x] `crates/nvs-syntax/src/parser/mod.rs:417`: `doc_tags` parses the three, joins a continuation
  line's text, and raises a type before `$name` and an empty text, each with its suggestion. Then
  `report_unknown_doc_tag` at `:462` drops `param` and `throws` and gives `returns` its suggestion.
  `E0128`'s card at `crates/nvs-diagnostics/src/code/parser.rs:213` names five tags.
- [x] Every `match` on `DocTagKind` builds, so each reader takes the new arms. `members.rs:739`,
  `meta.rs:1415`, `doc.rs:300` and `completion.rs:1781` give them a no-op arm for now. Stages 2 to 4
  fill them in.
- [x] `crates/nvs-syntax/tests/doc_comments.rs:216`: the refusals of `@param`, `@returns` and `@throws`
  are rewritten into the four tests the `tags-parse` check names. `tests/conformance/syntax/doc-comment/a-tag-that-is-not-see-or-example-is-a-diagnostic.nvst:1`
  is renamed to `a-tag-outside-the-five-is-a-diagnostic.nvst` and uses `@since`.

### 2. the checker holds every tag to the signature, and each mistake carries its fix

**Does:** `check_doc` checks the three tags against the declaration they are attached to, and every
diagnostic of § *Standing decisions*' checker list carries its `Suggestion`.

**Split:** once the checker lands, a writing subagent writes the seven `.nvst` cases the `tags-checked`
check names, and the session runs that check once.

- [x] `crates/nvs-hir/src/members.rs:734`: `check_doc` takes the declaration's parameters, its declared
  return type and its kind, from `check_members` and `check_stmts`. Its `match` at `:739` calls
  `check_param`, `check_return` and `check_throws`, and then the missing-`@param` warning.
- [x] `check_param`: the unknown name with the rename-or-delete suggestion, a repeated parameter, a
  repeated type, and the declaration kind.
- [x] `check_return`: `void`, `never`, a constructor, a repeat, a repeated type and the declaration
  kind. `check_throws`: resolves like `check_see` at `:758` does, then `Throwable` through
  `nvs_hir::hierarchy::implements_interface`, then a repeat.
- [x] The missing-`@param` warning with its insertion suggestion, placed in signature order.
- [x] Codes and cards in `crates/nvs-diagnostics/src/code/names.rs`, beside `E0323`.
- [x] The seven `.nvst` cases under `tests/conformance/syntax/doc-comment/`, and the `tags-checked`
  check run once.

### 3. meta, doc, hover and signature help show the tags in the Core card's shape

**Does:** every reader of a doc comment shows the three tags in signature order, a promoted parameter's
sentence is its property's, and an inherited `@param` is mapped by position.

- [x] `crates/nvs-cli/src/meta.rs:1406`: `put_doc` takes the member's parameters and writes `params`,
  `return` and `errors` with `doc_json`'s keys (`:611`), and a promoted property's `doc` takes its
  constructor `@param`. The inherited mapping by position goes where the ancestor's comment is read.
- [x] `crates/nvs-cli/src/doc.rs:296`: `card` renders a user card's parameters, return and errors the
  way it renders a `Core` card.
- [x] `crates/nvs-lsp/src/hover.rs:367`: `documentation` shows a user function's card in the `Core`
  layout. `:300` sets each `ParameterInformation.documentation`, for a user function and for a `Core`
  member (`MethodDoc::params`, `crates/nvs-stdlib/src/registry.rs:1257`).
- [x] The four tests the `tags-shown` check names, in `crates/nvs-cli/tests/meta.rs:103`,
  `crates/nvs-cli/tests/doc.rs:307` and `crates/nvs-lsp/tests/navigation.rs:269`, run once by name.

### 4. the editor completes, renames, finds and colours the tags

**Does:** the completion, rename, references, highlight, definition and grammar of § *Standing
decisions*' editor list. The quick fixes need no code here, because `actions.rs` translates every
`Suggestion` already. Their `.lspt` cases prove it.

**Split:** once the code lands, a writing subagent writes the ten `.lspt` cases the
`editor-answers` check names, copying the shape in `docs/agent/conventions.md` § *An `.lspt` case*.
The session runs that check once.

- [x] `crates/nvs-lsp/src/completion.rs:1758`: `doc_tag_at` reads a cursor after `@param ` and
  `@throws `. `doc_tag_names` at `:1790` offers only the tags the declaration takes, and the two target
  lists come from the declaration's parameters and from the `Throwable` walk.
  - [x] `Documented` and `documented_at` find the declaration under the run, and `takes` mirrors
    `check_return`'s placement.
  - [x] `param_targets` and `throws_targets`, dispatched from `Asked::DocTag`.
  - [x] Two tests in `crates/nvs-lsp/tests/completion_edits.rs`, and the old one rewritten.
- [x] `crates/nvs-lsp/src/locals.rs:101` and `crates/nvs-lsp/src/rename.rs:952`: a parameter's
  sites include its `@param` name, and the `$name` of an `@param` line resolves to its parameter.
  That one change gives references, highlight and definition. The rename follows the standing
  decision *A parameter rename stops being refused*.
  - [x] `locals::of` adds the `@param` target of the method's doc to a parameter's sites, and a
    cursor on that target finds the parameter.
  - [x] Rename lifts the parameter refusal and rewrites each named argument at the method's calls.
  - [x] `a-parameter-is-refused.lspt` rewritten as a rename that rewrites the call's `name:`.
- [x] Semantic tokens mark that `$name` as a parameter.
- [x] `editors/vscode/syntaxes/nvs.tmLanguage.json:113`: the scopes for the tag name, the `$name` and
  the `@throws` class. Cases go in `editors/vscode/test/grammar/comments.test.ts:1`, run once by name.
- [x] The ten `.lspt` cases, and the `editor-answers` check run once.

### 5. the feature's proofs and its pages say what the five tags do

**Does:** brings the doc comments feature's proofs and every user page up to the five tags. Every
file here is text an end user reads (`AGENTS.md` § *Text an end user reads*).

**Split:** one writing subagent writes the examples and their `.out` files, and another the hostile
case. Their files are disjoint. The session writes `about.md`, `details.md` and the reference pages.

- [x] `docs/examples/lang/programs/doc-comments/05-a-tag-never-repeats-a-type.nvs:1` and its `.out`
  are renamed from `04-a-param-tag-has-no-type` and keep showing `E0138`. A new
  `04-describe-parameters-return-and-errors.nvs` takes number 4, with its `.out`.
- [x] `docs/examples/lang/programs/doc-comments/about.md:1` and `details.md:1` say what the five tags
  do and what each checks, within about.md's length bound.
- [x] `tests/hostile/lang/programs/doc-comments/02-tags-that-point-at-code-that-is-not-there.nvs:1`:
  an `@param`, an `@throws` and an `@return` that each name something absent, and each is an error.
- [x] `docs/reference/lang/10-programs.md:404` re-rendered with `bun nv render`, and the
  editor features written into `docs/reference/tools/40-editor.md:1`. `35-packages.md` says a
  package's cards come from these tags.
- [x] `bun nv proofs --comments` over the new `.nvs` files, then the `feature-proofs` check.

### 6. nvs fmt puts the tags in one order

**Does:** the formatter writes every doc comment's tags in the order § *Standing decisions* sets, and a
doc tag error no longer stops it.

- [ ] `docs/rules/tooling/fmt-orders-the-doc-tags.md:1` written from § *Standing decisions*, and the
  "only reordering" sentence in `docs/rules/tooling/fmt-sorts-the-use-block.md:11` names both.
  `bun nv rules --check` is green.
- [ ] `crates/nvs-fmt/src/doc_tags.rs:1`: the reorder, called from `format` at
  `crates/nvs-fmt/src/lib.rs:165` beside the use-block sort (`crates/nvs-fmt/src/imports.rs`). It takes
  each `DocComment` from the parsed tree, with the parameter names of the declaration it is attached
  to.
- [ ] `crates/nvs-fmt/src/lib.rs:170`: every parser doc tag diagnostic, and `E0128`, is let through the
  refusal check.
- [ ] `crates/nvs-fmt/tests/doc_tag_order.rs:1`: the six tests the `fmt-orders-tags` check names, run
  once by name.
- [ ] `bun nv nvs-fmt` reruns the whole corpus, and every file it changes is committed in this slice.
  The examples and pages written in the proofs stage are in that corpus.
- [ ] A `.lspt` case, `tests/lsp/actions/format-orders-the-doc-tags.lspt:1`, if the replay supports a
  format request. Otherwise the Rust tests are the proof, and the handoff says so.

### 7. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stages 1 to 5 are done. `tags-parse`, `tags-checked`, `tags-shown`, `editor-answers` and
`feature-proofs` are green. Every reader takes the card from `DocComment` in
`crates/nvs-syntax/src/ast.rs:1705`.

Stage 5 landed in three commits. `d7f0320236` moves the unknown-`@param` error from `E0330`, which
`E_REQUIRE_PATH_NOT_STATIC` already had, to `E0335`. `no_code_is_declared_twice` in
`crates/nvs-diagnostics/src/code/mod.rs` would have failed on it in stage 6, and it passes now. That
commit also carries the `git mv` of example 04 to 05. `87e5bf1f00` fixes `bun nv proofs
--comments` (`commentProblemsOf` in `tools/nv/proofs/collect.ts`): a `///` tag line is one sentence
and not a line of prose, and a `//` run and a `///` run are two blocks. `tools/nv/test/proof-comments.test.ts`
pins it. `1933a59559` adds example 04, the attack `02-tags-that-point-at-code-that-is-not-there.nvs`
(it gives `E0335`, `E0334` and `E0333`, one per step), `about.md`, `details.md`, the section
*Doc comment tags* in `docs/reference/tools/40-editor.md`, a paragraph in `35-packages.md` § *nvs pkg
api and nvs pkg diff*, and the rendered `10-programs.md`.

The perf proof of `lang:programs/doc-comments` is stale, because `10-programs.md` changed. It is
owed at a release, as `docs/agent/commands.md` § *Verifying* says, and `feature-proofs` is green
with it.

The sweep's `env-secret-proofs` is stale: no check by that name exists, and its log is gone. The
nightly run 43581848915 ran on `be62f3f5a7` before 8bd6b6eda9 landed, so its Miri failures are the
ones that fix answers. The next nightly shows whether they pass. Expected red: none known.

### Next

- `docs/goals/doc-tags.md:393`: stage 6, which the person added before the gate. `nvs fmt` puts the
  tags in one order, and a doc tag error never stops it. Its items name each file.
- `docs/goals/doc-tags.md:414`: stage 7, the gate. Start `looper verify` with `run_in_background`, fix
  every failure it names in one pass, and run it once more. Then the goal-end gates AGENTS.md § *In
  the loop* names: `bun nv verify --doc`, `bun nv owners --closes doc-tags` and `bun nv ci --check`.

### Backlog

- `tests/conformance/novis/image-compare-counts-the-pixels-that-differ.nvst:1` and two other
  `image-*` cases failed once in the 123-case `tags-checked` run, and passed alone and on the rerun.
  Stage 6's verify shows whether they fail under load.
- `crates/nvs-stdlib/src/registry.rs:1`: `bun nv render --website` stops at `Novis\Image\Color::$r`,
  whose signature `uint $r` does not parse. The website render job may be red for it, and `bun nv ci
  --check` in stage 6 shows it.
