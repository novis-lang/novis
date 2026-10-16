`nvs check --json` writes the same `Diagnostic` records the terminal renderer prints — code, spans,
severity, help and `suggestions` (`rule:errors/diagnostic-record`) — as one machine-readable document. Its
schema is frozen the way `nvs ast --json`'s is, and snapshot-tested the same way; it is part of the CLI
surface the version contract covers. The document is `schemaVersion: 1` at its root, which is the key a
consumer reads first and the whole of what a version buys — every other key means what that number says
it means.

The text rendering stays the default and is what the Tasks `problemMatcher` reads; nothing about the
terminal output changes. The document exists for CI and for the agents that increasingly drive this
compiler, including the one that maintains this repository: over the corpus it emits one record per
diagnostic the text renderer prints, with the same codes and spans.

`nvs check` on the command line analyses what it is given, as it always has — the scope setting of
`rule:ide/check-scope-defaults-to-the-workspace` is the editor's, not the CLI's.
