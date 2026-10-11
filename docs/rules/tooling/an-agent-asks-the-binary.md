`nvs agent` is the surface a coding agent reads the language through, and it is four commands that
answer from what the binary already carries — the registry, and the reference chapters embedded in it:
`primer` prints the short document that makes an agent productive, `index` prints one line per member
and per chapter heading, `find <query>` prints the index lines matching a query, and `show <symbol>`
prints one member's card or one heading's section. Nothing is written to disk and nothing is cached,
so the binary that compiles a program is the binary that answers for it and an answer can never
describe a version that is not installed.

A keyword is grammar and no member is named for it, so the registry alone leaves `autoload`, `require`
and `match` unfindable, and an agent told that an empty `find` means "no such name" concludes the
language lacks them. Every syntactic form the language has is therefore a heading `find` reaches,
and an empty `find` says that a keyword may sit under a heading that does not name it. The chapters
are on the same index under the same two verbs, and not
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
That is why two diagnostics name their own fix: an undefined member gives a `help:` line with the
nearest declared name (`crates/nvs-hir/src/members.rs:1463`), and a capability denial gives one naming
the `nvs.toml` table that grants it (`crates/nvs-runtime/src/capability.rs:256`).

The surface exists because no model has Novis in its weights, so every agent writes it from what it is
handed at the time, on every session. The prior a model does hold is the harder part: Novis reads like
PHP and rejects most of PHP's syntax, so a confident guess is wrong in a systematic way. And the whole
reference is far larger than a context window while the flattened `Core` surface is small, so an
agent can hold every name the language has and still have room to work; what it cannot hold is the
prose around them. A primer read once and a lookup of a few hundred tokens each is the cost that fits.

- **Memory:** nothing on the request path. `nvs agent` is a CLI-only path over the document the binary
  already builds for `nvs meta --json`, and no server, isolate or request reaches it. What it spends is
  simplicity: one subcommand and one renderer in the binary.

**Declined:**
- **A smaller hand-written document.** It goes stale while the language is still adding members, and
  its completeness is whatever its author remembered, so an empty search over it proves nothing.
- **An MCP server.** It answers the same questions over a protocol fewer harnesses speak, needs a
  running process, and buys nothing over a subcommand any agent can already run. It would be one more
  pointer (`rule:tooling/an-adapter-carries-protocol-and-never-language`).
- **Vendoring the primer and the index into the user's project.** A copy on disk goes stale silently,
  adds diff noise to the user's repository, and brings back the shell-quoting trap `find` removes. An
  agent handed a broken pointer to such a copy rebuilds it from the binary, so the binary is the pointer.
- **Teaching the agent through the LSP.** `nvs-lsp` answers an editor's questions at a cursor; an agent
  has no cursor, and most harnesses do not speak the protocol.
- **An embedded form, `nvs agent init --embed`, writing the primer to disk** for a sandbox that cannot
  run the binary per session. Nothing has asked for it, and it is stale the moment it exists.
- **A heading for every lexer keyword.** The keyword table carries every spelling Novis refuses too,
  `and`, `goto` and `global` among them, and a heading per refused word is the wrong shape. The guard
  is a fixed list of the syntactic forms Novis has.
