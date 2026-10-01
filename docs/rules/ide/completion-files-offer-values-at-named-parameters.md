A `.json` file under a folder named `.novis/completion/`, anywhere in the workspace and `vendor/`
included, is a completion file: data that names method parameters and the values the editor offers in a
string argument at them. Nothing in it is run, by the language server or by any later reader of
`.novis`, and the server makes no network request for it.

**What a file says.** The top-level fields are `$schema`, `sets` and `parameters`, and the field set is
closed at every level. A set is a name, shared across every completion file in the workspace, and a list
of values. A list is an array of values, or an object `{ "separator", "values" }` whose separator is
`.`, `/` or `:`. A value is a string, or an object whose one required field is `value`, the inserted
text, with the optional fields `label`, `labelDetail`, `labelDescription`, `kind`, `title`,
`documentation`, `deprecated`, `sortText`, `filterText` and `preselect`, each one field of an LSP
completion item, and two more: `location`, a `file` relative to the completion file's folder and a
1-based `line`, and `replacement`, the value that replaces a deprecated one. An attachment is a `method`
(`Class::method` with the class's whole name, a constructor as `Class::constructor`), a `parameter`
named without its `$`, a `set`, a list in `values` or both, and two optional fields: `when`, a
`parameter` of the same method and the string or strings its argument must `equals`, and `strict`.

**Where it reaches.** An attachment is keyed by the declaration the checker resolved the call to, so an
inherited method is reached through the class that declares it, and an override needs an attachment of
its own. An attachment with `when` applies at a call only when that call's argument for the named
parameter is a string literal equal to one of its strings; when that argument is not a literal or is
left out, only the attachments without `when` apply. Every applying attachment for one parameter
contributes, files in path order and entries in written order, and a repeated `value` keeps the first
one read. Inside a string argument at such a parameter, `'` and `"` open the list, as they do at a path
or class-name parameter.

**What an item may do.** Its one edit replaces the text of the string literal with `value`, escaped for
the literal's quote style. In a list with a separator, completion offers the distinct segments that
follow the text already typed, and an item's edit replaces only the segment being typed; the
separator, `.` included, opens the next segment's list inside such a string. No item carries a command,
an edit outside the string or a snippet. `documentation` is untrusted Markdown, and a relative link or
image in it resolves against the completion file's folder.

**What else reads it.** Hover on a string literal at an attached parameter whose text equals a value
shows the value's `title` and `documentation`, and go-to-definition on it answers the value's
`location`. A literal equal to a deprecated value gets a hint with the `Deprecated` tag, and when the
value names a `replacement` the hint carries it as the suggestion its quick fix applies, rewriting only
the string. A parameter is strict when any attachment for it says `"strict": true`, and the language
server then warns on a string literal at it, in a `.nvs` file, whose text is not one of the values of
the attachments that apply at that call. A value built at run time is not checked, nor a literal where
a `when` cannot be decided, and `nvs check` reads no completion file. Without `strict`, nothing checks a
program's string literals against the values, because the data they describe can change after the
file is written.

**How it stays current.** The server finds the files when it starts and registers a client watcher for
`**/.novis/completion/**/*.json`. A file is read again when its modification time or size changes.

**What is reported** on the completion file's own URI, never for a file under `vendor/`: a file
that is not valid JSON or has an unknown or mistyped field, which then contributes nothing; an
attachment whose method or parameter, or whose `when` parameter, the class does not have, for a class
the workspace index declares; an attachment at a parameter that is not a string; a set no file
defines; a `location` whose file does not exist; a `replacement` on a value that is not deprecated;
and, as a hint only, a class the index does not declare.
