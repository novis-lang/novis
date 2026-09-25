Every doc comment clap renders — a subcommand's, a nested subcommand's, an argument's — is text a
person reads in `nvs --help`, and it is written for that person and nobody else. It names no `rule:`
token, no bare `§ 4`, no path under `docs/`, and no rustdoc `[link]`: each of those points at a
document the reader does not have, so a help page carrying them explains the project to itself while
the flag it was meant to describe goes unexplained.

The reasoning is not lost, it moves down one line. An ordinary `//` comment between the doc comment
and the item carries the citation, where `grep -rn 'rule:'` still finds it and
`bun nv rules --citations` still resolves it. The `///` says what the flag does; the `//`
says which rule decided that.

**It also names no input Novis does not accept.** `nvs run`, `nvs check`, `nvs ast` and `nvs serve`
read a file's content and never its extension, so a `.php` in a help line said "this runs PHP" while
meaning only "this ignores the name" — `rule:programs/no-compatibility-promise` reaching the surface
every user sees first. The one command that decides anything from an extension is `nvs test`, which
has two suites to choose between and takes `.nvs` alone.

What settles a case is not where a comment sits but whether it reaches a terminal, and an enum's own
doc comment does not — clap renders the parent variant's instead — so `ConfigCommand` and its like
are ordinary rustdoc and cite normally. The proof is the output: `nvs --help`, and every
subcommand's `--help`, grepped for `rule:`, `§` and `docs/`.
