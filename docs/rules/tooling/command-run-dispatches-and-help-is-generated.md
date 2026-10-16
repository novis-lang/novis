`Core\Command::run(): uint` is the entry point: it matches the command line against the compiled
table, dispatches to the handler the first word names, and returns the process exit status — a
handler's `void` is `0`, and `exit` still works from inside one
(`rule:statements/exit-is-the-only-termination-keyword`). A command line the table cannot fill is a
usage error — the usage line and a non-zero status — not a crash.

`Core\Command::help(?string $name): Cli\Text` and `Core\Command::completions(Command::Shell $shell): string`
are generated from the same table, for `bash`, `zsh`, `fish` and `pwsh` — the four cases of
`Command::Shell`, and nothing else. Two artefacts read one table, so neither can disagree with the program.

This is the **one deliberate divergence** from the route table, whose match is data and whose handler
runs only when the program calls `Core\Router::dispatch` with a gate
(`rule:routing/a-gate-answers-around-every-handler`). A web request has refusals a page must answer and
an access decision to check before every handler, and the gate is where both live. A CLI has one entry
point and no refusal for a page to answer, so a gate here would copy the router's shape rather than its
reasoning.
