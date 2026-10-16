Hovering a `#[Route]` attribute, or the name of the method it is on, shows what the program's grants
allow: the files it may read and write, the hosts it may connect to, the databases it may open, the
processes it may run, and every other grant by its name, each with its scope. The hover's first line
names the `nvs.toml` it read and says that the grants are the program's.

The grants are the configuration snapshot for the program's entry file, resolved the way `nvs check`
resolves it when run in that directory, through the same reader the extension set uses
(`crates/nvs-lsp/src/document.rs:787`). Nothing is inferred from the code, so the hover is exactly as
deep as the grant table the runtime enforces
(`rule:security/capability-question-is-grant-and-scope`). An edit to the grants changes the hover with
no edit to the source, because the tree is resolved again for each hover, as the extension set is: a
cached tree would outlive an edit to `nvs.toml`. With no `nvs.toml`, the hover says that none was found
and that the program runs under the default policy.

A grant belongs to a program, never to a route, so the hover never presents it as the route's.
Diagnostics are answered with no grants (`crates/nvs-lsp/src/document.rs:725-730`), because a
diagnostic sits on a line of code and cannot say which configuration it came from. A hover can, in its
first line, so the reader always sees which file answered.

- **Memory:** nothing at compile time for running and nothing at run time. A hover reads the
  configuration tree once and frees it with the answer.

**Declined:**
- **Narrowing the grants to what one route's handler calls.** It would be a guess from the code at
  what the program may do, and the runtime checks the program's grant, not the guess.
