---
milestone: post-parity
position: last
---
# Loop goal 183 — the JSON files in a `.novis/completion/` folder offer their values at the string parameters they name

ADR 0242 lets a project describe values the compiler never holds: the keys of a dataset, the names in
an icon set, the modules a deployment has. A JSON file under any `.novis/completion/` folder names a
method parameter and the values to offer there, and the editor offers them inside a string argument
at that parameter. This goal builds it.

```json
{
  "sets": { "icons": ["home", { "value": "arrow-left", "title": "arrow-left", "kind": "constant" }] },
  "parameters": [{ "method": "App\\Ui\\Icon::render", "parameter": "name", "set": "icons" }]
}
```

```nvs
Icon::render('');   // the cursor between the quotes offers `home` and `arrow-left`
```

## Why here

The user decided this interactively, and ADR 0242 records it. Nothing in the tree does it yet:
`crates/nvs-lsp/src/completion.rs` answers inside a string argument only at a path or class-name
parameter (ADR 0240 § 8), and `crates/nvs-lsp/tests/completion.rs:@no_completion_source_reads_a_directory_layout_or_the_network`
refuses `read_dir` and `std::fs` in every completion source, so the loader has to be a module of its
own. The server registers no file watcher today: no `didChangeWatchedFiles` appears under
`crates/nvs-lsp/src/`.

It carries `position: last` because it sits behind goal `coalesce-assign` and in front of goal
`ci-green`, and every goal in that tail is pinned.

## Stage 0 — the catch-up

The sentences on disk this goal makes wrong, each rewritten whole by the session that lands the
behaviour and not before:

- `docs/rules/ide/completion-is-asked-where-a-spelling-ends.md` and its record — the list of strings a
  `'` or `"` opens gains a string argument at a parameter a completion file names, and `because` gains
  `0242`. `docs/decisions/0242.md`'s `changes:` block gains the rule under `modifies`, because that
  block is a copy of the `because` lists.
- `docs/rules/ide/completion-files-offer-values-at-named-parameters.json` and
  `completion-offers-only-what-the-compiler-derived.json` — `status` becomes `shipped` and
  `guardedBy` names the tests below, in the session that makes them true.
- The doc comment of `crates/nvs-lsp/tests/completion.rs:@SOURCED`, which lists every table a
  completion source may read, gains the completion files' table.

`bun nv rules --render` regenerates the chapters, and `docs/novis.md` and `website/` are regenerated,
never edited.

## Stage 1 — the floor

Goal `coalesce-assign`'s whole acceptance list, carried in by the goal switch. Never traded.

## Stage 2 — the files are found, read, reloaded and checked, the keystone

**Does:** Finds every `.novis/completion/` folder, loads its JSON files into one table, reloads a
changed file, and reports a file or an entry that is wrong.

One file set: a new module `crates/nvs-lsp/src/completion_files.rs`, `crates/nvs-lsp/src/lib.rs`
(the module, the `initialized` registration and the `workspace/didChangeWatchedFiles` handler),
`crates/nvs-diagnostics/src/lib.rs` (the new codes).

- **The walk** (ADR 0242 § 2) starts at each workspace root, enters `vendor/`, skips `.git`, `target`,
  `node_modules` and every other dot-folder, and reads only `.json` files under a `.novis/completion/`
  folder. It is not `crate::index`'s walk and shares no skip list with it.
- **The table** is keyed by the declaring class's whole name, the method and the parameter name, the
  triple `ResolvedCall` carries (`crates/nvs-types/src/expr_table.rs:102`). Sets merge by name and
  values by `value`, the first one read kept, files in path order.
- **The format** is parsed with `serde` and `deny_unknown_fields` at every level, so an unknown field
  is a finding with the field's name. A file with a finding contributes nothing.
- **The watcher** is registered with `client/registerCapability` for
  `**/.novis/completion/**/*.json` when the client's capabilities allow dynamic registration of
  `workspace/didChangeWatchedFiles`. An event reloads a file only when its modification time or size
  changed. Under `rule:ide/the-server-is-synchronous` the handler runs on the one loop, as every
  notification does.
- **The diagnostics** (ADR 0242 § *Diagnostics*) are published on the completion file's own URI,
  never for a file under `vendor/`. One `W1xxx` code per kind, the next free numbers from
  `bun nv orient`'s *The next free number*, checked right before they are declared. A class the
  workspace index does not declare is a hint, not a warning. Not checked: whether VS Code shows a
  diagnostic for a URI outside the client's `documentSelector`. The session checks it first, and if
  it does not, the client's selector gains the completion files.
- **Pinned by** the Stage 2 checks, each over a temporary workspace the test writes and deletes.

## Stage 3 — completion offers the values

**Does:** Offers a parameter's values inside a string argument at it, with every field the file
gives, and opens the list on the quote.

One file set: `crates/nvs-lsp/src/completion.rs`, `crates/nvs-lsp/src/arguments.rs`,
`crates/nvs-lsp/tests/completion.rs`.

- **The arm** finds the parameter through `crate::arguments`, as the path and class-name arms do
  (ADR 0240 § 8), and reads the loaded table. It reads no file and no directory itself, and the guard's
  `SOURCED` table gains its row naming the table.
- **An item** maps each field to the `CompletionItem` field ADR 0242 § 3 names, and its one edit
  replaces the literal's text with `value` escaped for the literal's quote style. It carries no
  `command`, no `additionalTextEdits` and no snippet. `documentation` is `MarkupContent` in Markdown,
  and a relative link or image is rewritten to a `file:` URI against the completion file's folder.
- **The trigger.** `continues_a_trigger` (`crates/nvs-lsp/src/completion.rs:415`) answers `'` and
  `"` inside a string argument at a named parameter.
- **A parameter with a path or class-name mark** is offered both lists.
- **Pinned by** the Stage 3 checks and the two guard tests already in `crates/nvs-lsp/tests/completion.rs`.

## Stage 4 — the VS Code client

**Does:** Ships a JSON schema for completion files, so a person writing one by hand gets the editor's
own JSON completion and validation.

One file set: `editors/vscode/package.json`, a new `editors/vscode/schemas/completion-file.json`.

- **`contributes.jsonValidation`** gains one entry, `fileMatch` `**/.novis/completion/**/*.json`,
  `url` the shipped schema. `rule:ide/contributions-are-frozen-and-only-ever-added` allows an
  addition.
- **The schema** states the fields of ADR 0242 § 3, `additionalProperties: false` at every level, and
  the kind names as an enum. It cannot know which methods exist, and the server's warnings stay the
  authority.
- **The image check.** Whether VS Code renders an image in a completion item's documentation, for a
  `file:` URI and for an `https:` one, is checked by hand in the Extension Development Host. The result
  goes in the module doc of `completion_files.rs`. Nothing is changed when it does not render, because
  the text does.

## Stage 5 — the feature proofs

**Does:** Adds the description, tests, examples, bench, attack and help the feature owes.

- **What an editor feature owes** is not checked: the roster's kinds are a `Core` member, a language
  feature, a command and a configuration key. `bun nv proofs` is asked first. If no kind fits, the
  session puts the question in the handoff's `## Backlog` for the user and proves what the closest
  kind owes.
- **The attack** is a completion file that tries to do more than offer text: a `command` field, an
  `additionalTextEdits` field, a `command:` link in `documentation`, and a file outside a `completion/`
  folder. Each is ignored or reported, and nothing runs.
- **The help** is the reference page for the file format, reached by `nvs agent find completion file`.
- **Every comment in a new `.nvs` and every `about.md`** follows `AGENTS.md` § *Text an end user reads*
  at the first write, and `bun nv proofs --comments <paths>` runs over them before the wrap.

## Standing decisions

- **The user's calls**, recorded in ADR 0242, are not re-decided: files only, no command that writes
  them; any number of `.novis` folders, found by name and never configured; `.novis` is not tied to
  the editor, and `completion/` is its first kind; `vendor/` is searched; named sets attach to
  parameters; sources merge; an attachment to a missing method or parameter is a warning; reloads
  compare modification time; every field a completion item can show is definable.
- **The goal writer's calls**, not confirmed by the user: the field names in ADR 0242 § 3, a whole file
  dropped for one parse finding, a hint and not a warning for an undeclared class, no diagnostics
  under `vendor/`, and `command`, `additionalTextEdits` and snippets left out.
- **No ADR slot.** ADR 0242 is the record. A question it does not answer is decided in the module doc
  of `completion_files.rs` and named in the handoff.
- **Nothing in a completion file is run**, and the server makes no network request for one. A session
  that finds a reason to break either stops and asks.
- **Every name in a test, an example and a record is neutral**: `App\Ui\Icon`, `Blog`, `Shop`.
- **Tradeoffs.** Performance: one directory walk at server start, one hash lookup per completion
  request inside a string argument, nothing at run time. Memory: every loaded value, held in the
  server. Usability: values from outside the program complete with labels, icons and documentation.
  Simplicity: one file format and one server module, and no change to the language.
