The Windows SCM stores **one string**, and the process gets it back through `CommandLineToArgvW`. The
trailing argv is encoded into that string under those rules — the backslash-run-before-a-quote rule
included — and **that string is the only record of what the service runs**. `sc qc <name>` shows an
auditor literally what runs, with no second place to look.

A sidecar argv file with a short `ImagePath` would make the round trip exact and is refused anyway: a
file that decides what a `LocalSystem` process executes is a new writable instruction source, which is
`rule:packaging/the-installer-is-a-sink` with the check removed.

The encoder is one function with a round-trip property — trailing backslashes, embedded quotes, a
directory path ending in `\` before a closing quote — and a fuzz target. It is
`rule:core-classes/process-is-argv-only` inverted: that rule refuses to *build* a command line for a
child, this one has no choice, so the construction is confined to one tested place.

**The binary path is quoted unconditionally**, whether or not it currently contains a space. An
unquoted `ImagePath` under a path with a space is the textbook Windows privilege-escalation finding,
Novis's default install location is such a path, and the finding usually appears after somebody moves
the installation. Arguments given to `sc start <name> arg` reach `ServiceMain` but are not persisted;
nothing may depend on them, and `nvs service run` ignores them.
