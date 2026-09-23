`nvs agent index` carries one line for every configuration key, every command and subcommand, every
flag, and every diagnostic code, beside its lines for members and headings, and `find` and `show`
reach each of them under the name a user types. A key's line is `config: [server] max_in_flight`, a
command's `command: nvs serve`, a flag's `flag: nvs serve --mount`, a code's `code: E0621` followed by
the first sentence of its card.

Each line is derived at the call from the table the binary already runs on — the configuration
schema, the command-line parser and the diagnostics table — and kept nowhere, which is
`rule:testing/roster-is-derived`'s shape: a key, a flag or a code that lands owes its line at once,
and nobody edits a list.

The entries are exact names, never a search of the text under a heading. A full-text match answers
one word with every section that mentions it, and an empty answer from it no longer means that the
name does not exist, which is the property `rule:tooling/the-index-is-one-line-per-member` keeps.
