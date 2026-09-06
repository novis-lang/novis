`Cli::ask`, `Cli::confirm`, `Cli::select<T>`, `Cli::multiSelect<T>` and `Cli::secret` are `Core`
members, because raw mode is unreachable from userland with the FFI door shut
(`rule:security/no-ffi`) and an extension runs in a sandbox with no tty: if `Core` does not ship the
interactive half, Novis does not have it at any tier. A question takes its options as one trailing
shape — `{default?, validate?}` on `ask`, `{default?}` on `confirm`, `{labels?, default?}` on
`select` — and a list of choices is `$choices`, since `options` is reserved for the trailing shape
(`rule:core-api/options-bag`).

Four properties follow from rules that already exist. `select<T>` returns the chosen value, not an
index. `confirm` returns a plain `bool`, because a closed two-case answer is a checked conversion and
a checked conversion launders. `secret` originates a `secret tainted string`
(`rule:security/secret-has-no-ambient-source`), so a password typed at a prompt structurally cannot
be echoed, logged, dumped, put in a `Throwable` message or serialized. And **a prompt reads the
controlling terminal, not standard input** — `/dev/tty`, `CONIN$` — so `cat data.csv | myprog` can
still ask a question; without that, piping and prompting are mutually exclusive, which is the defect
in every hand-rolled version.
