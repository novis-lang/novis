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
behind a fifth command: `nvs agent init`'s stanza names these four, a stanza that reads differently from
what the binary would write is refused, and a verb added later would turn every installed project's
re-run into that refusal.

Over the registry, all four render `nvs meta --json`'s document and decide nothing (`rule:tooling/one-json-several-renderers`),
which makes them a fourth consumer rather than a fifth source of truth. `find` is a command rather than
an instruction to grep the index, because a namespaced name loses its backslash to the shell before
`grep` sees it and the empty result that follows is indistinguishable from a name the language does not
have.

The protocol is three calls and a check: read `primer` once, `find` a name, `show` its card, then
`nvs check`. A diagnostic is the cheapest documentation the system has — it is read only by the agent
that got something wrong — so the check loop is part of the surface rather than an alternative to it.
