Hovering a `#[Route]` attribute, or the name of the method it is on, shows what the program's grants
allow: the files it may read and write, the hosts it may connect to, the databases it may open, the
processes it may run, and every other grant by its name, each with its scope. The hover's first line
names the `nvs.toml` it read and says that the grants are the program's. The attribute keeps its own
card above the grants; the method's name answers the grants alone.

The grants are the configuration snapshot for the hovered file, resolved the way `nvs check` resolves
it when run in that directory, through the same reader the extension set uses
(`nvs_lsp::document::program_tree`). Every capability of the closed roster is a line: what the
snapshot writes for it, or its default for server work marked `(default)` where it writes nothing. A
default that grants nothing is not a line, and one closing sentence says no other grant is given.
Nothing is inferred from the code, so the hover is exactly as deep as the grant table the runtime
enforces (`rule:security/capability-question-is-grant-and-scope`). An edit to the grants changes the
hover with no edit to the source, because the tree is resolved again for each hover, as the extension
set is: a cached tree would outlive an edit to `nvs.toml`. With no `nvs.toml`, the hover says that
none was found and lists the default policy. A tree that does not load is named, with no grants, and
`nvs check` reports why.

A path inside the hovered file's directory is written relative to it, and any other path whole. The
text is `nvs_lsp::grants::of_program`'s.

A grant belongs to a program, never to a route, so the hover never presents it as the route's.
Diagnostics are answered with no grants (`nvs_lsp::document::analyse_file`'s type phase), because a
diagnostic sits on a line of code and cannot say which configuration it came from. A hover can, in its
first line, so the reader always sees which file answered.

- **Memory:** nothing at compile time for running and nothing at run time. A hover reads the
  configuration tree once and frees it with the answer.

**Declined:**
- **Narrowing the grants to what one route's handler calls.** It would be a guess from the code at
  what the program may do, and the runtime checks the program's grant, not the guess.
