`nvs agent index` carries one line for every configuration key, every command and subcommand, every
flag, and every diagnostic code, beside its lines for members and headings, and `find` and `show`
reach each of them under the name a user types. A key's line is `config: [server] max_in_flight`, a
command's `command: nvs serve`, a flag's `flag: nvs serve --port`, a code's `code: E0621` followed by
the first sentence of its card once it carries one (`rule:tooling/a-diagnostic-code-carries-its-card`).
The symbol of such a line is what follows its `config: `, `command: ` or `flag: `, and the code alone
after `code: `; a key is also reached by its dotted name, `server.max_in_flight`. `show` prints a
key's comment block from the shipped file, and a command's or flag's own `--help` text.

Each line is derived at the call from the table the binary already runs on — the shipped `nvs.toml`
that spells every key the parser reads and is held to that parser, the command-line parser with its
hidden commands and flags left out, and the diagnostics crate's code list, which the macro declaring
the codes writes — and kept nowhere, which is `rule:testing/roster-is-derived`'s shape: a key, a flag
or a code that lands owes its line at once, and nobody edits a list.

The entries are exact names, never a search of the text under a heading, so an empty answer still
means that the name does not exist, which is the property `rule:tooling/the-index-is-one-line-per-member`
keeps. Building the lines is more work per `find`, and it is a command-line lookup that no request
path reaches.

**Memory:** nothing held. The lines are derived at the call and kept nowhere.

**Declined:**
- **A full-text search over every section.** It answers one word with every section that mentions
  it, which an agent has to read through, and an empty answer from it no longer means the name does
  not exist.
- **Full text beside the exact entries, exact entries first.** It has the same loose matches after
  the exact ones, and it is more to build and to test.
