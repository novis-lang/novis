`nvs agent` is the surface a coding agent reads the language through, and it is four commands that
answer from what the binary already carries — the registry, and the reference chapters embedded in it:
`primer` prints the short document that makes an agent productive, `index` prints one line per member
and per chapter heading, `find <query>` prints the index lines matching a query, and `show <symbol>`
prints one member's card or one heading's section. Nothing is written to disk and nothing is cached,
so the binary that compiles a program is the binary that answers for it and an answer can never
describe a version that is not installed.

A keyword is grammar and no member is named for it, so the registry alone leaves `autoload`, `require`
and `match` unfindable, and an agent told that an empty `find` means "no such name" concludes the
language lacks them. The chapters are therefore on the same index under the same two verbs, and not
behind a fifth command: `nvs agent init`'s pointers name these four, and a verb added later is one no
agent learns of until every installed pointer has been rewritten to name it.

Over the registry, all four render `nvs meta --json`'s document and decide nothing (`rule:tooling/one-json-several-renderers`),
which makes them a fourth consumer rather than a fifth source of truth. `find` is a command rather than
an instruction to grep the index, because a namespaced name loses its backslash to the shell before
`grep` sees it and the empty result that follows is indistinguishable from a name the language does not
have.

`index`, `find` and `show` take `--json` for a tool that parses rather than reads, and the document is
a second rendering of the same entries and cards rather than a source of its own: the text card is
rendered from the very map `show --json` prints. It follows `nvs check --json`'s conventions
(`rule:ide/check-json-is-the-diagnostic-record-as-a-document`) — `schemaVersion: 1` at the root and
frozen the same way, every key present as `null` or `[]` where nothing was written, and a `show` that
fails still prints its document, an `error` and the `nearest` records, on standard output while its
exit status says it failed. The shape is sketched in `crates/nvs-cli/src/agent.rs`'s module doc.

The protocol is three calls and a check: read `primer` once, `find` a name, `show` its card, then
`nvs check`. A diagnostic is the cheapest documentation the system has — it is read only by the agent
that got something wrong — so the check loop is part of the surface rather than an alternative to it.
