A `//` line comment whose text, after the slashes and spaces, starts with `TODO:` in capitals is a todo,
and the rest of the line is its text.

```nvs
// TODO: page through the results once `find` takes an offset.
```

A `///` line is one too, so a declaration's todo goes in the doc comment it already has:

```nvs
/// Returns every order of one customer.
/// TODO: page through the results once `find` takes an offset.
/// @param $customerId The customer whose orders are loaded.
```

The line stays in the run, so the run still attaches, but it is never documentation: hover, `nvs meta`,
signature help, route docs and a package's API text leave it out, and it ends an `@param`, `@return` or
`@throws` text the way a blank line does. `nvs fmt` leaves a run's tags in written order when a todo
line follows its first tag. `todo:`, `TODO` without the colon, and a `/* */` comment are never one.
Todos are read from the trivia the lexer already keeps. A todo is a comment and only a comment.

**It is a list, not a diagnostic.** `Severity` has no todo level. The language server shows each todo
as an `Information` entry with source `todo` and no code, never phase-gated, and `nvs check --todos`
prints `path:line: text` for every one, in path and line order; under `--json` the document gains a
`todos` array (`rule:ide/check-json-is-the-diagnostic-record-as-a-document`). `nvs check` without
`--todos` prints none.

**`nvs check` has two more flags beside it.**

- `--deny <kind>`, repeatable, over the closed list `deprecated` and `todo`. The command prints
  everything it would have printed, then exits with failure when one of those kinds is found in a file
  outside `vendor/`.
- `--fix` applies every suggestion marked `safe`, from every diagnostic, in every file of the checked
  program outside `vendor/`. Overlapping edits apply first-come; the program is checked again and the
  next pass applies what is left, until a pass applies nothing. It prints how many edits it made in how
  many files. It never writes under `vendor/`.

**Declined:**
- **A todo attribute or a statement attribute.** It says what a comment already says, a second way, and
  attributes attach to declarations only.
- **`TODO(owner):`, `FIXME:` and an `@todo` doc tag.** One form keeps the list one list, and
  `TODO:` already works inside a `///` run.
- **Leaving `///` lines out of the list.** A declaration with a doc comment would need a second
  comment block above it for its todo.
- **`--deny warnings`, or any `--deny` kind beyond the two.** The list stays closed until a kind is
  argued for.
- **A workspace-wide fix in the editor.** The language server does not implement
  `workspace/executeCommand`; `nvs check --fix` is the workspace-wide form.
