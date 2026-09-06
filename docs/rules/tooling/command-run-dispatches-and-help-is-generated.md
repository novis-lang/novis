`Core\Command::run(): uint` is the entry point: it matches the command line against the compiled
table, dispatches to the handler the first word names, and returns the process exit status — a
handler's `void` is `0`, and `exit` still works from inside one
(`rule:statements/exit-is-the-only-termination-keyword`). A command line the table cannot fill is a
usage error — the usage line and a non-zero status — not a crash.

`Core\Command::help(?string $name): Cli\Text` and `Core\Command::completions(Cli\Shell $shell): string`
are generated from the same table, for `bash`, `zsh`, `fish` and `pwsh` — the four cases of
`Cli\Shell`, and nothing else. Two artefacts read one table, so neither can disagree with the program.

This is the **one deliberate divergence** from the route table: `rule:routing/matching-is-not-dispatching`
stops at matching because a web framework must own the pipeline — middleware, controllers, response
mapping — and baking a convention in would fight it. A CLI has one entry point and no middleware
question, so the reason the router stops does not exist here, and stopping anyway would copy its
shape rather than its reasoning.
