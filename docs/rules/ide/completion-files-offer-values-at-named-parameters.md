A `.json` file under a folder named `.novis/completion/`, anywhere in the workspace and `vendor/`
included, is a completion file: data that names method parameters and the values the editor offers in a
string argument at them. Nothing in it is run, by the language server or by any later reader of
`.novis`, and the server makes no network request for it.

The values such a file lists are ones the program cannot see while it compiles: the modules a
deployment has, the names in an icon set, the keys of a dataset kept outside the repository. There is
no first description of them for the file to be a second, unchecked copy of, so
`rule:ide/completion-offers-only-what-the-compiler-derived` admits it as its one source of values the
compiler does not derive. Novis defines the file and nothing that writes it, so a Novis script, a
Python script, a CI job or a person can produce one, and nothing runs inside the editor. `.novis` is a
project's folder for data its tools read, found anywhere and as many times as a project needs, not an
editor's; each kind of data is a subfolder with its own format, so a later kind never shares a file
with this one, and any non-JSON file under `completion/` is ignored, so a README can sit beside the
data. It follows the dot-folder convention of `.vscode` and `.idea`, and is `novis` rather than `nvs`
because `.nvs` is the source file extension.

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

Each optional field answers something a plain list cannot say. A translation key is defined in a file
of its own, and a developer reading it wants its text and the line that defines it: `location`. One
method's keys can depend on another argument, and a list merging every domain's keys offers mostly
wrong ones: `when`. Keys written as paths, with many entries under a few prefixes, are easier to choose
one segment at a time: a separator. A renamed value should say what replaced it: `replacement`.

**Where it reaches.** An attachment is keyed by the declaration the checker resolved the call to, so an
inherited method is reached through the class that declares it, and an override needs an attachment of
its own. An attachment with `when` applies at a call only when that call's argument for the named
parameter is a string literal equal to one of its strings, that argument found the way the completed
one is, so a named and a positional argument read alike; when that argument is not a literal or is
left out, only the attachments without `when` apply. Every applying attachment for one parameter
contributes, files in path order and entries in written order, and a repeated `value` keeps the first
one read. A set or a merge is never a conflict, so nothing about one is reported. Inside a string
argument at such a parameter, `'` and `"` open the list, as they do at a path or class-name parameter,
and a parameter a path or class-name mark already completes is offered both lists.

**What an item may do.** Its one edit replaces the text of the string literal with `value`, escaped for
the literal's quote style. In a list with a separator, completion offers the distinct segments that
follow the text already typed, and an item's edit replaces only the segment being typed. A segment
that ends a value carries that value's fields, and one that only leads to longer values carries its
text alone; the strict check and hover still compare whole values. The
separator, `.` included, opens the next segment's list inside such a string. No item carries a command,
which would let a cloned repository run an editor command when an item is accepted, an edit outside
the string, or a snippet, which means nothing inside a string; each is an unknown field. `documentation`
is untrusted Markdown, so a `command:` link in it is not followed, and a relative link or image in it
resolves against the completion file's folder and is sent as a `file:` URI, so an icon set can show
its icon. A URL in it is fetched, if at all, by the client that renders it.

**What else reads it.** Hover on a string literal at an attached parameter whose text equals a value
shows the value's `title` and `documentation`, and go-to-definition on it answers the value's
`location`, at the start of its line; a value with no `location` answers hover alone. Both read the
loaded table and neither opens the named file. A literal equal to a deprecated value gets a hint with the `Deprecated` tag, and when the
value names a `replacement` the hint carries it as the suggestion its quick fix applies, rewriting only
the string. A parameter is strict when any attachment for it says `"strict": true`, and the language
server then warns on a string literal at it, in a `.nvs` file, whose text is not one of the values of
the attachments that apply at that call. A value built at run time is not checked, nor a literal where
a `when` cannot be decided, and `nvs check` reads no completion file. Without `strict`, nothing checks a
program's string literals against the values, because the data they describe can change after the
file is written and a stale file would raise false warnings. That holds for a file nobody keeps
current and not for one a build step regenerates from the data itself, and only the person who
writes the file knows which kind it is, so the check is theirs to ask for.

**How it stays current.** The server walks each workspace root once when it starts. The walk enters
`vendor/`, so a library can ship values for its own parameters, skips `target`, `node_modules` and
every dot-folder but `.novis`, lists directories and reads no source; it is not the workspace index's
walk, which skips `vendor/`. The server then registers a client watcher for
`**/.novis/completion/**/*.json` where the client supports dynamic registration, and a client without it
gets the files found at start. A file is read again only when its modification time or size changes,
since a client reports changes that are not. The VS Code extension ships a JSON schema for the same
glob, so a file written by hand gets the editor's own JSON completion and validation; the server's
warnings stay the authority, because the schema cannot know which methods exist.

**What is reported** on the completion file's own URI, never for a file under `vendor/`: a file
that is not valid JSON or has an unknown or mistyped field, which then contributes nothing; an
attachment whose method or parameter, or whose `when` parameter, the class does not have, for a class
the workspace index declares; an attachment at a parameter that is not a string; a set no file
defines; a `location` whose file does not exist; a `replacement` on a value that is not deprecated;
and, as a hint only, a class the index does not declare, because it may come from `vendor/`, which
the index does not read. A file with a field it does not understand contributes nothing, so a value is
never offered from a file that is half understood; an attachment naming a missing method or parameter
contributes nothing on its own and the rest of the file still does, so a rename in the program does not
silence every set in a file until it is regenerated. A file under `vendor/` is a library's to fix.

- **Memory:** every value of every completion file, kept in the server for its lifetime, in proportion
  to what the files hold. One walk of the directory tree at start, one `stat` per changed file per
  watcher event, and one lookup in the loaded table per completion, hover, definition or code-action
  request inside a string argument. A strict parameter costs one set lookup per string literal at it
  each time a document is analysed. Nothing at run time, and a program compiles the same with or
  without the files.

**Declined:**
- **A helper written in Novis, run when completion is requested.** Opening a cloned repository would
  run whatever it does, a keystroke would wait on a compile and on the helper's own reads, and one
  position could answer differently from one keystroke to the next.
- **A `nvs` command that runs such a helper and keeps its result in a file.** A file any program can
  write already does this, and a command would fix one way of writing it.
- **A list of completion files in `nvs.toml`.** `nvs.toml` configures a program that runs, and these
  files serve the editor; a folder found by name needs no configuration to add a file.
- **One file per project.** Several generated sources would have to be merged into it by hand.
- **A fixed file name such as `novis-ide-meta.json` found anywhere.** A subfolder per kind leaves
  `.novis` open to later kinds, and a file name would claim the whole folder for completion.
- **An attribute in the source naming a set, `#[Completion('icons')]`.** It needs a change to the code,
  and a project could not give values to a library's parameters.
- **Values attached to a method's return value or to a property.** A return or property type that is a
  set of allowed string values already says this in the program, and the checker holds it.
- **The `strict` check on by default.** A file nobody regenerates would turn into false warnings; the
  person who writes the file says when it is current.
