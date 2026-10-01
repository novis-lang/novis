---
milestone: post-parity
position: last
---
# Loop goal 183 — the JSON files in a `.novis/completion/` folder offer their values at the string parameters they name

ADR 0242 lets a project describe values the compiler never holds: the keys of a dataset, the names in
an icon set, the modules a deployment has. A JSON file under any `.novis/completion/` folder names a
method parameter and the values to offer there, and the editor offers them inside a string argument
at that parameter. ADR 0243 adds to it: a value's source location for hover and go-to-definition, an
attachment that depends on another argument, keys completed one segment at a time, an opt-in check of
the literals, a replacement for a deprecated value with a quick fix, and completion of a parameter
typed as a union of string literals, which needs no file. This goal builds both records in one pass.

```json
{
  "sets": {
    "icons": ["home", { "value": "arrow-left", "title": "arrow-left", "kind": "constant" }],
    "shop-keys": { "separator": ".", "values": ["shop.cart.title", "shop.checkout"] }
  },
  "parameters": [
    { "method": "App\\Ui\\Icon::render", "parameter": "name", "set": "icons", "strict": true },
    {
      "method": "App\\I18n\\Text::translate", "parameter": "key", "set": "shop-keys",
      "when": { "parameter": "domain", "equals": "shop" }
    }
  ]
}
```

```nvs
Icon::render('');                  // the cursor between the quotes offers `home` and `arrow-left`
Text::translate('shop', 'shop.');  // the cursor after the dot offers `cart` and `checkout`
```

## Why here

The user decided this interactively, and ADR 0242 and ADR 0243 record it. Nothing in the tree does it
yet: `crates/nvs-lsp/src/completion.rs` answers inside a string argument only at a path or class-name
parameter (ADR 0240 § 8), and `crates/nvs-lsp/tests/completion.rs:@no_completion_source_reads_a_directory_layout_or_the_network`
refuses `read_dir` and `std::fs` in every completion source, so the loader has to be a module of its
own. The server registers no file watcher today: no `didChangeWatchedFiles` appears under
`crates/nvs-lsp/src/`. Nothing under `crates/nvs-lsp/src/` reads a literal type, although
`crates/nvs-types/src/expr_table.rs:102` (`ResolvedCall::param_tys`) records every parameter's type on
the resolved call.

It carries `position: last` because it sits behind goal `coalesce-assign` and in front of goal
`ci-green`, and every goal in that tail is pinned.

## Stage 0 — the catch-up

The sentences on disk this goal makes wrong, each rewritten whole by the session that lands the
behaviour and not before. A record's `changes:` block is a copy of the `because` lists, so each rule
whose `because` gains a number is added under `modifies` in that record's block in the same session.

- `docs/rules/ide/completion-is-asked-where-a-spelling-ends.md` and its record — the trigger list
  gains `.`; the strings a `'` or `"` opens gain a string argument at a parameter a completion file
  names and one at a parameter typed as a union of string literals; and `.`, `/` and `:` open the next
  segment inside a string at a parameter whose list has that separator. `because` gains `0242` and
  `0243`.
- `docs/rules/ide/the-request-set-is-closed.md` and its record — `hover` gains a string literal equal
  to a completion file's value, `definition` gains its `location`, and `completion` gains a completion
  file's values and a literal union's members. `because` gains `0242` and `0243`.
- `docs/rules/ide/a-code-action-ships-only-a-fix-a-diagnostic-already-knows.md` and its record — the
  replacement of a deprecated value is one more fix admitted under the same boundary. `because` gains
  `0243`.
- `docs/rules/ide/completion-files-offer-values-at-named-parameters.json` and
  `completion-offers-only-what-the-compiler-derived.json` — `status` becomes `shipped` and
  `guardedBy` names the tests below, in the session that makes the whole rule true.
- The doc comment of `crates/nvs-lsp/tests/completion.rs:@SOURCED`, which lists every table a
  completion source may read, gains the completion files' table and the checker's type table.

`bun nv rules --render` regenerates the chapters, `bun nv render` and `bun nv decisions --render` the
rest, and `docs/novis.md` and `website/` are regenerated, never edited.

## Stage 1 — the floor

Goal `coalesce-assign`'s whole acceptance list, carried in by the goal switch. Never traded.

## Stage 2 — the files are found, read, reloaded and checked, the keystone

**Does:** Finds every `.novis/completion/` folder, loads its JSON files into one table with every field
of both records, reloads a changed file, and reports a file or an entry that is wrong.

One file set: a new module `crates/nvs-lsp/src/completion_files.rs`, `crates/nvs-lsp/src/lib.rs`
(the module, the `initialized` registration and the `workspace/didChangeWatchedFiles` handler),
`crates/nvs-diagnostics/src/lib.rs` (the new codes).

- **The walk** (ADR 0242 § 2) starts at each workspace root, enters `vendor/`, skips `.git`, `target`,
  `node_modules` and every other dot-folder, and reads only `.json` files under a `.novis/completion/`
  folder. It is not `crate::index`'s walk and shares no skip list with it.
- **The table** is keyed by the declaring class's whole name, the method and the parameter name, the
  triple `ResolvedCall` carries (`crates/nvs-types/src/expr_table.rs:102`). Sets merge by name and
  values by `value`, the first one read kept, files in path order. Each attachment keeps its `when` and
  `strict`, and each list its `separator`, so the completion arm decides at the call which apply.
- **The format** is parsed with `serde` and `deny_unknown_fields` at every level, so an unknown field
  is a finding with the field's name. A file with a finding contributes nothing. It includes ADR 0243's
  fields: `location` and `replacement` on a value, `when` and `strict` on an attachment, and a list
  written as `{ "separator", "values" }` with a separator of `.`, `/` or `:`.
- **The watcher** is registered with `client/registerCapability` for
  `**/.novis/completion/**/*.json` when the client's capabilities allow dynamic registration of
  `workspace/didChangeWatchedFiles`. An event reloads a file only when its modification time or size
  changed. Under `rule:ide/the-server-is-synchronous` the handler runs on the one loop, as every
  notification does.
- **The diagnostics** (ADR 0242 and ADR 0243 § *Diagnostics*) are published on the completion file's
  own URI, never for a file under `vendor/`. One `W1xxx` code per kind, the next free numbers from
  `bun nv orient`'s *The next free number*, checked right before they are declared. A class the
  workspace index does not declare is a hint, not a warning. A `location` whose file does not exist is
  checked when the file loads, with one `stat`. Not checked: whether VS Code shows a diagnostic for a
  URI outside the client's `documentSelector`. The session checks it first, and if it does not, the
  client's selector gains the completion files.
- **Pinned by** the Stage 2 checks, each over a temporary workspace the test writes and deletes.

## Stage 3 — completion offers the values

**Does:** Offers a parameter's values inside a string argument at it, with every field the file
gives, only from the attachments whose `when` holds at the call, one segment at a time where a list
has a separator, and opens the list on the quote and on the separator.

One file set: `crates/nvs-lsp/src/completion.rs`, `crates/nvs-lsp/src/arguments.rs`,
`crates/nvs-lsp/src/capabilities.rs`, `crates/nvs-lsp/tests/completion.rs`.

- **The arm** finds the parameter through `crate::arguments`, as the path and class-name arms do
  (ADR 0240 § 8), and reads the loaded table. It reads no file and no directory itself, and the guard's
  `SOURCED` table gains its row naming the table.
- **An item** maps each field to the `CompletionItem` field ADR 0242 § 3 names, and its one edit
  replaces the literal's text with `value` escaped for the literal's quote style. It carries no
  `command`, no `additionalTextEdits` and no snippet. `documentation` is `MarkupContent` in Markdown,
  and a relative link or image is rewritten to a `file:` URI against the completion file's folder.
- **Dependent values** (ADR 0243 § 2). The argument a `when` names is found through
  `crate::arguments`, positional or named. The attachment applies only when it is a string literal
  equal to one of `equals`; otherwise only the attachments without `when` apply.
- **Segments** (ADR 0243 § 3). A list with a separator offers the distinct segments after the text
  already typed, and an item's edit replaces only the segment being typed. A segment that ends a value
  carries that value's fields.
- **The triggers.** `continues_a_trigger` (`crates/nvs-lsp/src/completion.rs:415`) answers `'` and
  `"` inside a string argument at a named parameter, and `.`, `/` and `:` inside one whose applying
  list has that separator. `.` joins the completion trigger characters in
  `crates/nvs-lsp/src/capabilities.rs:218`.
- **A parameter with a path or class-name mark** is offered both lists.
- **Pinned by** the Stage 3 checks and the two guard tests already in `crates/nvs-lsp/tests/completion.rs`.

## Stage 4 — hover, go-to-definition, the strict check and the replacement

**Does:** Answers hover and go-to-definition on a string literal equal to a value, warns on a literal
at a strict parameter that is not a value, and marks a deprecated value with a hint whose quick fix
writes its replacement.

One file set: `crates/nvs-lsp/src/hover.rs`, `crates/nvs-lsp/src/definition.rs`,
`crates/nvs-lsp/src/diagnostics.rs`, `crates/nvs-lsp/src/actions.rs`, `crates/nvs-diagnostics/src/lib.rs`
(the two new codes), `crates/nvs-lsp/tests/navigation.rs` and `crates/nvs-lsp/tests/actions.rs`.

- **Hover** (ADR 0243 § 1) is an arm beside the path and class-name arms in `crate::hover`, asked
  before the walk over the nodes: the value's `title` and `documentation`, as the completion popup
  shows them.
- **Definition** answers the value's `location`, the start of its line, as a `file:` URI. A value
  without one gives no answer.
- **The strict check** (ADR 0243 § 4) runs over the string literal arguments of a `.nvs` document as
  its diagnostics are built, one set lookup each, and only at a strict parameter. It skips a value built
  at run time and a call where a `when` cannot be decided. It is phase-gated like every diagnostic.
- **The deprecated hint** (ADR 0243 § 5) carries `DiagnosticTag::Deprecated`, and its replacement as
  its own `Suggestion`, so `crate::actions` offers the fix as a translation and computes nothing
  (`rule:ide/a-quick-fix-is-a-diagnostics-own-suggestion`). The edit is the literal's text, escaped for
  its quote style.
- **Pinned by** the Stage 4 checks.

## Stage 5 — a parameter typed as a union of string literals completes its members

**Does:** Offers the members of a string literal union inside a string argument at a parameter of
that type, and opens the list on the quote.

One file set: `crates/nvs-lsp/src/completion.rs`, `crates/nvs-lsp/src/arguments.rs`,
`crates/nvs-lsp/tests/completion.rs`; `crates/nvs-types/src/expr_table.rs` is read and not changed.

- **The arm** (ADR 0243 § 6) reads the parameter's type off `ResolvedCall::param_tys` and its members
  from the checker's type table. It applies when the type is made only of string literal types, with
  or without `null`. Each member is an item of kind `value` whose one edit replaces the literal's text.
- **The guard.** The `SOURCED` row for this arm names the checker's type table, a table the compiler
  builds for another reason.
- **A parameter that also has a completion file** is offered both lists, merged by value.
- **Pinned by** the Stage 5 check and the two guard tests.

## Stage 6 — the VS Code client

**Does:** Ships a JSON schema for completion files, so a person writing one by hand gets the editor's
own JSON completion and validation.

One file set: `editors/vscode/package.json`, a new `editors/vscode/schemas/completion-file.json`.

- **`contributes.jsonValidation`** gains one entry, `fileMatch` `**/.novis/completion/**/*.json`,
  `url` the shipped schema. `rule:ide/contributions-are-frozen-and-only-ever-added` allows an
  addition.
- **The schema** states the fields of ADR 0242 § 3 and ADR 0243, `additionalProperties: false` at every
  level, the kind names and the three separators as enums, and a list as either an array or the
  `{ "separator", "values" }` object. It cannot know which methods exist, and the server's warnings
  stay the authority.
- **The image check.** Whether VS Code renders an image in a completion item's documentation, for a
  `file:` URI and for an `https:` one, is checked by hand in the Extension Development Host. The result
  goes in the module doc of `completion_files.rs`. Nothing is changed when it does not render, because
  the text does.

## Stage 7 — the feature proofs

**Does:** Adds the description, tests, examples, bench, attack and help the feature owes.

- **What an editor feature owes** is not checked: the roster's kinds are a `Core` member, a language
  feature, a command and a configuration key. `bun nv proofs` is asked first. If no kind fits, the
  session puts the question in the handoff's `## Backlog` for the user and proves what the closest
  kind owes.
- **The attack** is a completion file that tries to do more than offer text: a `command` field, an
  `additionalTextEdits` field, a `command:` link in `documentation`, a `replacement` that tries to edit
  outside the string, and a file outside a `completion/` folder. Each is ignored or reported, and
  nothing runs.
- **The help** is the reference page for the file format, reached by `nvs agent find completion file`.
- **Every comment in a new `.nvs` and every `about.md`** follows `AGENTS.md` § *Text an end user reads*
  at the first write, and `bun nv proofs --comments <paths>` runs over them before the wrap.

## Standing decisions

- **The user's calls**, recorded in ADR 0242 and ADR 0243, are not re-decided: files only, no command
  that writes them; any number of `.novis` folders, found by name and never configured; `.novis` is not
  tied to the editor, and `completion/` is its first kind; `vendor/` is searched; named sets attach to
  parameters; sources merge; an attachment to a missing method or parameter is a warning; reloads
  compare modification time; every field a completion item can show is definable; a value's
  `location` for hover and ctrl+click, with a warning when its file is missing; `when` on an
  attachment, which does not apply when the other argument is not a literal; segments with the
  separators `.`, `/` and `:`, and `.` as a trigger only inside such a string; `strict` as an opt-in,
  reported by the language server only and not by `nvs check`; a deprecated value's `replacement` as a
  hint with the Deprecated tag and a quick fix that changes only the string; completion of a string
  literal union from the compiler; no values attached to a return value or a property; no file that
  overrides a return type.
- **The record writer's and goal writer's calls**, not confirmed by the user: the field names in ADR
  0242 § 3; a whole file dropped for one parse finding; a hint and not a warning for an undeclared
  class; no diagnostics under `vendor/`; `command`, `additionalTextEdits` and snippets left out; the
  object form of a list allowed in an attachment's `values` as well as in a set; a left-out argument
  counted as not a literal for `when`; the strict check comparing against the attachments that apply
  at the call and skipping a call where a `when` cannot be decided; the Deprecated hint on every
  deprecated value, with the quick fix only where a `replacement` is given; a warning for a
  `replacement` on a value that is not deprecated; a `when` naming a missing parameter joining the
  missing-parameter kind; a segment that only leads to longer values carrying its text alone; a
  literal union's members as items of kind `value`, merged with a completion file's values.
- **No other ADR slot.** ADR 0242 and ADR 0243 are the records. A question they do not answer is
  decided in the module doc of `completion_files.rs` and named in the handoff.
- **Nothing in a completion file is run**, and the server makes no network request for one. A session
  that finds a reason to break either stops and asks.
- **Every name in a test, an example and a record is neutral**: `App\Ui\Icon`, `App\I18n\Text`,
  `Blog`, `Shop`.
- **Tradeoffs.** Performance: one directory walk at server start, one hash lookup per completion,
  hover, definition or code-action request inside a string argument, one set lookup per string literal
  argument at a strict parameter per analysis, nothing at run time. Memory: every loaded value, held in
  the server, with a little more for each `location` and `replacement`. Usability: values from outside
  the program complete with labels, icons and documentation, open their source, depend on another
  argument, complete by segment, and can be checked; a literal union completes with no file at all.
  Simplicity: one file format and one server module, and no change to the language.
