A `//` line comment whose text, after the slashes and spaces, starts with `TODO:` in capitals is a todo,
and the rest of the line is its text.

```nvs
// TODO: page through the results once `find` takes an offset.
```

A `///` doc comment is never one, and neither is `todo:`, `TODO` without the colon, or a `/* */`
comment. Todos are read from the trivia the lexer already keeps, so the parser does not change. A todo is
a comment and only a comment: there is no todo attribute and no statement attribute, because either
would say the same thing a second way.

**It is a list, not a diagnostic.** `Severity` has no todo level. The language server shows each todo
as an `Information` entry with source `todo` and no code, never phase-gated, and `nvs check --todos`
prints `path:line: text` for every one, in path and line order; under `--json` the document gains a
`todos` array (`rule:ide/check-json-is-the-diagnostic-record-as-a-document`). `nvs check` without
`--todos` prints none.

**`nvs check` gains two more flags with it.**

- `--deny <kind>`, repeatable, over the closed list `deprecated` and `todo`. The command prints
  everything it would have printed, then exits with failure when one of those kinds is found in a file
  outside `vendor/`.
- `--fix` applies every suggestion marked `safe`, from every diagnostic, in every file of the checked
  program outside `vendor/`. Overlapping edits apply first-come; the program is checked again and the
  next pass applies what is left, until a pass applies nothing. It prints how many edits it made in how
  many files. It never writes under `vendor/`.

`TODO(owner):`, `FIXME:` and any `--deny` kind beyond the two are not part of the form.
