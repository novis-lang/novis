WASI is **not** granted to a guest by default. A component receives only Novis's own capability-checked
host functions, so its filesystem and network access is governed by the same root-owned `nvs.toml`
grants as script code (`rule:security/capability-check-at-the-door`,
`rule:security/capability-question-is-grant-and-scope`). Native extension code calls `open()` and
`connect()` directly and bypasses any capability system; a guest cannot, because it has no syscalls at
all.

WASI is available as an opt-in world, and its preopens are derived from the capability grants rather
than declared beside them — there is no second place authority is written.

The same absence is what keeps an extension honest about qualifiers: with no ambient source and no
sink of its own, it can declare what it consumes and produces but cannot launder
(`rule:security/extension-cannot-launder`).
