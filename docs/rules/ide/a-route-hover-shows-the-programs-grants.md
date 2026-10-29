Hovering a `#[Route]` attribute, or the name of the method it is on, shows what the program's grants
allow: the files it may read and write, the hosts it may connect to, the databases it may open, the
processes it may run, and every other grant by its name, each with its scope. The hover names the
`nvs.toml` it read and says that the grants are the program's. The attribute keeps its own card above
the grants; the method's name answers the grants alone.

The program is the one the hovered file belongs to. A file another program autoloads or requires
borrows that program's map (`rule:ide/an-autoloaded-file-borrows-its-programs-map`), and its grants
are that program's, found from the directory of its entry, the file `nvs run` is handed. The hover's
first line then names that entry. A file that is its own entry is read from its own directory.

The grants are the configuration snapshot for that entry, resolved the way `nvs check` resolves it
when run in the entry's directory, through the same reader the extension set uses
(`nvs_lsp::document::program_tree`). Every capability of the closed roster is a line: what the
snapshot writes for it, or its default for server work marked `(default)` where it writes nothing. A
default that grants nothing is not a line, and one closing sentence says no other grant is given.
`net.internal`'s addresses are a line after the roster's when they are written. Below the list come
the other grants the program carries: each `[[extension]]` entry's own `grants`
(`rule:security/extension-grants-are-an-intersection`), and the `[grants]` table of the
`package.toml` found from the entry (`rule:security/grants-are-keyed-on-a-namespace`), one line per
key. Nothing is inferred from the code, so the hover is exactly as deep as the grant table the runtime
enforces (`rule:security/capability-question-is-grant-and-scope`). An edit to the grants changes the
hover with no edit to the source, because the tree is resolved again for each hover, as the extension
set is: a cached tree would outlive an edit to `nvs.toml`. With no `nvs.toml`, the hover says that
none was found and lists the default policy. A tree that does not load is named, with no grants, and
`nvs check` reports why, and so is a `[grants]` table that does not load.

A path inside the hovered file's directory is written relative to it, and any other path whole. The
text is `nvs_lsp::grants::of_program`'s.

A grant belongs to a program, never to a route, so the hover never presents it as the route's.
Diagnostics are answered with no grants (`nvs_lsp::document::analyse_file`'s type phase), because a
diagnostic sits on a line of code and cannot say which configuration it came from. A hover can, in its
first lines, so the reader always sees which file answered.

- **Memory:** nothing at compile time for running and nothing at run time. A hover reads the
  configuration tree and `package.toml` once and frees them with the answer.

**Declined:**
- **Narrowing the grants to what one route's handler calls.** It would be a guess from the code at
  what the program may do, and the runtime checks the program's grant, not the guess.
- **The grants of the hovered file's own directory, for a file another program loads.** That
  directory's `nvs.toml` is not the one the program runs under, so its grants would be another
  program's.
