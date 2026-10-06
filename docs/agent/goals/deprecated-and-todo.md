---
milestone: post-parity
position: last
---
# Loop goal 190 — a deprecated member names its replacement as code, every use of it is rewritten with one click, and a `// TODO:` comment is a list the tools keep

Three things a developer marks in their own code, none of them an error unless the developer asks.

```nvs
class Api {
    // 1. A deprecation names its replacement as a Novis expression. The placeholders are the
    //    member's own parameters and `$this`.
    #[Core\Deprecated(since: "2.0", note: "`find` also takes a limit.", replace: "$this->find($id, limit: $limit)")]
    public function findById(int $id, int $limit = 10): ?User { return $this->find($id, limit: $limit); }

    public function find(int $id, int $limit): ?User { … }
}

// At every use: warning W1003, struck through in the editor, and a quick fix that writes
//   $user = $api->find($request->id(), limit: 10);
$user = $api->findById($request->id());

// 2. A todo is a comment, anywhere a comment may stand.
// TODO: page through the results once `find` takes an offset.
```

```toml
# 3. A developer may make a use of deprecated code log or throw while it runs. Off by default.
[errors]
deprecated = "throw"         # "ignore" (default) | "log" | "throw"
```

**The attribute.** `#[Core\Deprecated(since:, note:, replace:)]` joins the compiler-recognized roster,
every field optional. It attaches to a class, an interface, an enum, an enum case, a method, a
constructor, a property, a class constant and a parameter. Every use of the member is warning `W1003`,
whose message carries `since`, `note` and the replacement as it would be written at that use.

**The replacement is code, and the compiler checks it where it is declared.** `replace` is one Novis
expression (one type name for a class, an interface or an enum), compiled in the scope of the
declaration with its parameters and `$this` bound. It must compile, its type must fit the member's own,
and it may use nothing less visible than the member and nothing deprecated. A broken template fails the
build of the code that declares it, never the fix a user applies.

**The fix is mechanical, and nearly always offered.** At each use the checker fills the template in —
arguments by position or name, the default for an argument left out, the receiver for `$this` — and
carries the result as `W1003`'s `Suggestion`, so the language server's quick fix and `source.fixAll.nvs`
are the translation they already are. An argument with a side effect that the template would run twice,
never, or out of order is moved to a `var` line in front of the statement. A use where that line would
change when the code runs gets no fix, and its warning shows the template.

**The todo.** `// TODO: text` is collected from the trivia the lexer already keeps. `nvs check --todos`
lists every one, and the language server shows each at `Information` level. It is never a diagnostic,
never a warning, and never fails a build.

**The command line.** `nvs check --deny deprecated` and `--deny todo` exit non-zero when one is found
outside `vendor/`. `nvs check --fix` applies every `safe` suggestion in the checked program and checks
again until none applies, and never writes under `vendor/`.

**The setting.** `[errors] deprecated` is `Runtime`-class, so a test or a single request may set it.
`"log"` writes one `warning` record per use site per request; `"throw"` throws `Core\DeprecatedError`,
a `LogicError`, whose message names the replacement. `[mode]` does not change it.

## Why here

The user asked for it: a way for a library author to retire a member and hand every caller the rewrite,
and a todo marker the tools know. Almost everything it stands on is built and unused:

- `rule:attributes/attach-sites-and-forms` already has a closed, `Core`-owned roster of
  compiler-recognized attributes (`crates/nvs-types/src/derive.rs:@ATTRIBUTES`), and `#[Core\Path]`
  (`crates/nvs-types/src/paths.rs:@check_marker_sites`, `:@declared_text`) is the precedent for one that
  a use site reads back. ADR 0046 left `#[Deprecated]` to its own record, and none was written.
- `W_DEPRECATED` (`crates/nvs-diagnostics/src/lib.rs:5365`) is declared and nothing raises it.
- `rule:tooling/doc-comment-tags-are-see-and-example` refuses `@deprecated` with the sentence
  "deprecation is an attribute", which is not true of anything yet.
- `rule:ide/a-code-action-ships-only-a-fix-a-diagnostic-already-knows` means a fix the checker computes
  costs the language server nothing: `crates/nvs-lsp/src/actions.rs` translates `Diagnostic::suggestions`.
- The lexer keeps every `//` comment as trivia (`crates/nvs-syntax/src/lexer.rs:@push_trivia`) and the
  language server holds it (`crates/nvs-lsp/src/document.rs:511`).
- `DebugFlags` (`crates/nvs-runtime/src/ctx/mod.rs:278`) is a per-request word codegen already branches
  on (`crates/nvs-codegen/src/emit.rs:@emit_stmt_probe`), and it is the shape of the runtime check.

It sits behind goal `php-oracle-retired` because that goal rewrites the `php-migration` rules and the
reader's text this goal also edits, and it carries `position: last` because every goal around it does.

## Stage 0 — the catch-up

The sentences on disk this goal makes wrong or incomplete, each rewritten whole by the session that
lands the behaviour and not before:

- `docs/rules/attributes/attach-sites-and-forms.md` — "attaches to four things"; the roster gains
  `Core\Deprecated`, and the attach sites this goal uses are stated.
- `docs/rules/ide/a-code-action-ships-only-a-fix-a-diagnostic-already-knows.md` — "two … and no other",
  then a third; the deprecation fix is the fourth, admitted under the same boundary.
- `docs/rules/tooling/doc-comment-tags-are-see-and-example.md` — "deprecation is an attribute" gains its
  `rule:` token.
- `docs/rules/config/every-block-is-argued-where-it-is-added.md` — the table gains `[errors]`.
- `crates/nvs-diagnostics/src/lib.rs:5365` — `W_DEPRECATED`'s card says what the message carries.
- `crates/nvs-diagnostics/src/diagnostic.rs`'s doc on `Suggestion::safe` and
  `crates/nvs-lsp/src/actions.rs`'s module doc — both name `nvs convert`, which does not exist, and the
  module doc counts the fixes that exist; `nvs check --fix` is the reader of `safe` now.
- `crates/nvs-config/src/default.toml` — the `[errors]` block, with its key line ending `# default`.
- `docs/reference/tools/10-cli.md` § *nvs check*, and the reference chapter that states attributes.

The same search closes the stage as it opened it:
`grep -rn "four things\|no other\|nvs convert\|W1003\|@deprecated" docs/rules docs/reference crates/nvs-diagnostics crates/nvs-lsp/src`,
read line by line. Every hit is either true as it stands, rewritten, or under `docs/decisions/`.
`docs/novis.md`, `docs/ground-rules.md`, the `docs/rules/*.md` chapters and `website/` are generated and
are regenerated, never edited.

## Stage 1 — the floor

Goal `php-oracle-retired`'s whole acceptance list, carried in by the goal switch. Never traded. A program
that uses nothing deprecated compiles to the same code as before: no flag test is emitted anywhere a
deprecated member is not used.

## Stage 2 — the attribute and the warning, the keystone

**Does:** Adds `#[Core\Deprecated]`, checks its replacement where it is declared, and raises `W1003` at
every use.

One file set: `crates/nvs-types/src/derive.rs`, `crates/nvs-types/src/attributes.rs`,
`crates/nvs-types/src/deprecated.rs` (new), `crates/nvs-types/src/expr/calls.rs`,
`crates/nvs-types/src/expr/members.rs`, `crates/nvs-types/src/expr/assign.rs`,
`crates/nvs-diagnostics/src/lib.rs`.

- **The decision record**, written first, from § *Standing decisions*, for the whole goal. It `creates`
  one rule under `docs/rules/attributes/` (the attribute, its template and its fix), one under
  `docs/rules/tooling/` (the todo comment and the two `nvs check` flags) and one under
  `docs/rules/errors/` (the setting), and `modifies` the four Stage 0 names. The fragments are written
  with it.
- **The roster.** `Core\Deprecated` in `crates/nvs-types/src/derive.rs:@ATTRIBUTES`, with its card in
  `ATTRIBUTE_DOCS`, so `every_attribute_carries_a_card` holds. Its payload is `since`, `note` and
  `replace` (strings), and `construct` on a class only; anything else is refused by
  `crates/nvs-types/src/attributes.rs:@check_recognized`, and an attach site not in the list above by a
  marker-site check shaped like `paths.rs:@check_marker_sites`.
- **The template is checked once, at the declaration**, in `crates/nvs-types/src/deprecated.rs`: it
  parses as one expression (one type name for a class, an interface or an enum); it compiles with the
  member's parameters and, on an instance member, `$this` bound; its type is assignable to the method's
  return type, the property's type or the constant's type, and a parameter's template is a named argument
  whose value fits that parameter; it names nothing less visible than the member; it names nothing
  deprecated, so every fix is one step. A class's replacement declares every public member the class
  declares, with an assignable signature, and `construct` is a template over the constructor's
  parameters for a `new`. Each refusal is its own code from the `E08xx` band, read from
  `bun nv orient` right before it is declared.
- **`W1003` at every use**, from the places that already resolve one: `expr/calls.rs:@resolved_call`
  for the three call forms and `:@infer_new`, `expr/members.rs:@check_property_access` and
  `:@infer_class_const`, the static-property arm of `expr/mod.rs`, and `expr/assign.rs:@check_write_target`.
  A type position naming a deprecated class warns too. An override or implementation of a deprecated
  member warns at the override, with no fix. **Nothing warns inside a deprecated declaration**, nor inside
  a member of a deprecated class, so a library's own old code is quiet.
- **The message** names the member, then `since` when written, then `note`, then the replacement as it
  would be written at that use. `W_DEPRECATED`'s card is rewritten to say so.
- **Pinned by** the Stage 2 checks, and
  `tests/conformance/reject/a-deprecation-template-that-does-not-fit-does-not-compile.nvst` (one refusal
  per template check; one diagnostic each).

## Stage 3 — the rewrite

**Does:** Fills the template in at every use and carries the result as `W1003`'s suggestion, so the
editor offers it as a quick fix.

One file set: `crates/nvs-types/src/deprecated.rs`, `crates/nvs-types/src/expr/calls.rs`,
`crates/nvs-lsp/src/actions.rs`, `crates/nvs-lsp/src/diagnostics.rs`, `crates/nvs-lsp/src/hover.rs`,
`crates/nvs-lsp/src/completion.rs`.

- **Substitution on the tree, printed once.** Every occurrence of a parameter in the template is the
  argument's source text, matched by position or by name; an argument left out is the parameter's
  default as written at the declaration; `$this` is the receiver as written; a static member's class is
  the class as the use wrote it. A substituted expression is put in parentheses where its precedence is
  lower than its place in the template, and nowhere else.
- **Names.** A name in the template is resolved at the declaration. At the use it is written as the
  shortest name that resolves to the same declaration in that file, or fully qualified, and an import is
  added by the same computation `E0303`'s import fix uses. `rule:ide/no-refactoring-introduces-an-alias`
  holds.
- **Side effects.** An argument or receiver is *pure* when it contains no call, `new`, assignment,
  increment or decrement. A pure one is copied wherever the template uses it. An impure one used exactly
  once and in the order the use wrote it is copied too. Every other impure one is moved to a `var` line
  in front of the statement, in the order the use evaluated it, under a name made from its parameter's
  that nothing in scope takes, and the template uses that variable.
- **No fix where a `var` line would change when the code runs**: the use sits on the right of `&&`,
  `||`, `??` or `?:`, in a loop's condition or step, in an arrow function or a `match` arm, in a default
  value or a property initializer. Nor for a `...$args` spread into a template parameter. The warning is
  raised as before and its help shows the template.
- **Every fix is `safe`.** The rewrite keeps the order and the count of every side effect by
  construction, and what the replacement does is the declaration author's claim, which the template's
  type check bounds. A fix whose output would not compile is a bug, and the test below holds it.
- **The editor.** `crates/nvs-lsp/src/diagnostics.rs` sets `DiagnosticTag::DEPRECATED` on `W1003`, so
  the use is struck through. A completion item for a deprecated member carries
  `CompletionItemTag::DEPRECATED`, as completion files' values already do. Hover on a deprecated member
  shows `since`, `note` and the template. `crates/nvs-lsp/src/actions.rs` needs no new code: its module
  doc is rewritten to name the fourth fix.
- **Pinned by** the Stage 3 checks: every fixture the Rust tests rewrite is checked again after the fix
  and has no `W1003` and no error, and applying the fix a second time changes nothing.

## Stage 4 — the todo comment

**Does:** Collects `// TODO:` comments and shows them in the editor and on the command line.

One file set: `crates/nvs-syntax/src/todo.rs` (new), `crates/nvs-syntax/src/lexer.rs`,
`crates/nvs-lsp/src/diagnostics.rs`, `crates/nvs-lsp/src/document.rs`.

- **The form.** A `//` line comment whose text, after the slashes and spaces, starts with `TODO:`, in
  capitals. The rest of the line is the text. A `///` doc comment is never one, and neither is `todo:`,
  `TODO` without the colon, or a `/* */` comment. `crates/nvs-syntax/src/todo.rs` reads them from the
  trivia `Parser::with_trivia` already keeps, so the parser is unchanged.
- **The editor** publishes each as an LSP diagnostic of severity `Information`, source `todo`, no code,
  beside the file's own diagnostics and never phase-gated.
- **Not a diagnostic.** `Severity` gains no variant, and `nvs check` without `--todos` prints none.
- **Pinned by** the Stage 4 checks.

## Stage 5 — `nvs check`

**Does:** Adds `--deny`, `--fix` and `--todos` to `nvs check`.

One file set: `crates/nvs-cli/src/main.rs` (`Command::Check`, `run_check`), `crates/nvs-cli/src/check.rs`,
`crates/nvs-cli/tests/`.

- **`--deny <kind>`**, repeatable, from a closed list: `deprecated` and `todo`. The command exits with
  failure when one is found in a file outside `vendor/`, after printing everything it would have printed.
- **`--fix`** applies every suggestion marked `safe`, from every diagnostic, in every file of the checked
  program outside `vendor/`. Overlapping edits are applied first-come; the program is checked again and
  the next pass applies what is left, until a pass applies nothing. It prints how many edits it made in
  how many files. The casing and legacy-cast fixes come along, because they are `safe` suggestions too.
- **`--todos`** prints `path:line: text` for every todo comment, in path and line order. With `--json`
  the document gains a `todos` array, and its schema version moves as
  `rule:ide/check-json-is-the-diagnostic-record-as-a-document` says.
- **Pinned by** the Stage 5 checks.

## Stage 6 — the setting

**Does:** Makes a use of deprecated code write a log record or throw while it runs, when the setting
says so.

Two file sets, in this order. The setting and the error: `crates/nvs-config/src/directive.rs`,
`crates/nvs-config/src/default.toml`, `crates/nvs-hir/src/errors.rs`,
`crates/nvs-runtime/src/throwable.rs`, `crates/nvs-runtime/src/ctx/mod.rs`. The check:
`crates/nvs-codegen/src/emit.rs`, `crates/nvs-ir/src/lower/`.

- **`[errors] deprecated`**, a new top-level block argued by the new `errors` rule: `"ignore"`, `"log"`
  or `"throw"`, default `"ignore"` in every mode, `Runtime`-class and reloadable. `Core\Config::set`
  changes it for the request that calls it.
- **`Core\DeprecatedError`**, a `LogicError`, in `crates/nvs-hir/src/errors.rs:@TREE` and
  `crates/nvs-runtime/src/throwable.rs:@ThrownClass`. Its message is `W1003`'s text, fixed at compile
  time and stored once in the unit's constant pool.
- **Where the check is emitted.** A deprecated method or constructor checks on entry, so a call through
  an interface or a dynamic call is caught. A read or write of a deprecated property, class constant or
  enum case, a `new` of a deprecated class, and a passed deprecated parameter check at the use. A type
  position checks nothing, because it runs nothing. The check is a load of one word in the context, as
  `DebugFlags` is, and a branch to a cold helper when it is not zero.
- **`"log"`** writes one `warning` record per use site per request, with the member, the use's file and
  line, and the replacement. A request remembers the sites it has logged in a set dropped with it.
- **Pinned by** the Stage 6 checks.

## Stage 7 — the feature proofs and the reference

**Does:** Adds the tests, examples, attacks, benches and reference text for the attribute, the todo
comment and the setting.

- A reference heading for each of the three, so each is a feature on the roster, and the feature proofs
  `bun nv proofs --id` lists for each: `about.md`, the Novis and Rust tests, three examples, one bench,
  one attack and the help in the binary.
- The attack for the setting: a request tries to read another request's logged sites, and a request
  that sets `"throw"` leaves the next request on `"ignore"`.
- The bench for the attribute: a loop calling a deprecated method with the setting at `"ignore"`, beside
  the same loop calling the replacement.
- Every document Stage 0 lists, each rewritten whole; `bun nv reference` regenerates `docs/novis.md`.

## Standing decisions

- **The user's calls.** The replacement is code, and the fix should be mechanical in nearly every case,
  the complicated ones included. A todo is a comment, and only a comment: an attribute would say the same
  thing a second way. `nvs check` can fail a build on a deprecation. A use of deprecated code can throw
  while it runs, switched in `nvs.toml`, off by default in every mode, because deprecated code still
  works in production and only a developer turns this on. `"log"` is the third value.
- **A deprecation is never an error** at compile time. `W1003` stays a warning, and only `--deny` or the
  setting turns it into a failure.
- **A refused construct stays refused.** The constructs the compiler rejects outright, such as
  `rule:statements/no-return-leaves-a-finally`, are not deprecations, and this goal is about what a
  Novis program's author retires.
- **The placeholders are the parameters.** No `{0}`, no `$1`, no other template syntax: a template is a
  Novis expression with the declaration's own variables in it.
- **One step.** A template that names something deprecated is refused, so no fix produces code with a
  second warning to fix.
- **No new `Severity`.** A todo is a list the tools print, not a diagnostic.
- **No statement attributes.** Marking a block of code is what a todo comment does.
- **Not in this goal:** `--deny warnings` or any other kind; a workspace-wide fix in the editor
  (`workspace/executeCommand` is not implemented, `crates/nvs-lsp/src/settings.rs:98`); `TODO(owner):`
  or `FIXME:`; deprecating a type alias, which carries no attributes. A session that wants one puts it in
  the handoff's `## Backlog` for the user.
- **One ADR slot**: one new record and no other number, checked right before it is written. It states
  the tradeoffs. Performance: nothing in a program that uses no deprecated code; one load and one branch
  per use of deprecated code, every time it runs, whatever the setting. Memory: the template and the
  message once per declaration in the unit; the `"log"` site set per request, O(distinct deprecated sites
  the request reaches), freed with it. Usability: a library retires a member and its callers migrate with
  one click or one `nvs check --fix`. Simplicity: one attribute, one comment form, one setting, two
  flags; the rewrite's side-effect handling is the large part of the work, and it is the checker's.
- **New diagnostic codes** only for the template refusals, from the `E08xx` band. No TextMate change
  beyond what a recognized attribute already gets.
- **Every name in a test, an example and the record is neutral** — `Api`, `Shop`, `Blog`.
- **Every comment in a new `.nvs` and every changed `about.md` follows `AGENTS.md` § *Text an end
  user reads* at the first write**, and `bun nv proofs --comments <paths>` is run over them before the
  wrap.
