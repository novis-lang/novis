A `.json` file under a folder named `.novis/completion/`, anywhere in the workspace and `vendor/`
included, is a completion file: data that names method parameters and the values the editor offers in a
string argument at them. Nothing in it is run, by the language server or by any later reader of
`.novis`, and the server makes no network request for it.

**What a file says.** The top-level fields are `$schema`, `sets` and `parameters`, and the field set is
closed at every level. A set is a name, shared across every completion file in the workspace, and a list
of values. A value is a string, or an object whose one required field is `value`, the inserted text, with
the optional fields `label`, `labelDetail`, `labelDescription`, `kind`, `title`, `documentation`,
`deprecated`, `sortText`, `filterText` and `preselect`, each one field of an LSP completion item. An
attachment is a `method` (`Class::method` with the class's whole name, a constructor as
`Class::constructor`), a `parameter` named without its `$`, and a `set`, `values` or both.

**Where it reaches.** An attachment is keyed by the declaration the checker resolved the call to, so an
inherited method is reached through the class that declares it, and an override needs an attachment of
its own. Every attachment for one parameter contributes, files in path order and entries in written
order, and a repeated `value` keeps the first one read. Inside a string argument at such a parameter,
`'` and `"` open the list, as they do at a path or class-name parameter.

**What an item may do.** Its one edit replaces the text of the string literal with `value`, escaped for
the literal's quote style. No item carries a command, an edit outside the string or a snippet.
`documentation` is untrusted Markdown, and a relative link or image in it resolves against the
completion file's folder.

**How it stays current.** The server finds the files when it starts and registers a client watcher for
`**/.novis/completion/**/*.json`. A file is read again when its modification time or size changes.

**What is reported**, on the completion file's own URI and never for a file under `vendor/`: a file
that is not valid JSON or has an unknown or mistyped field, which then contributes nothing; an
attachment whose method or parameter the class does not have, for a class the workspace index declares;
an attachment at a parameter that is not a string; a set no file defines; and, as a hint only, a class
the index does not declare. Nothing checks a program's string literals against the values, because the
data they describe can change after the file is written.
