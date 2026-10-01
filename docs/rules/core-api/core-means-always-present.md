Nothing outside Tier 0 may register a class under the `Core` namespace, and nothing whose presence a build
flag can remove is registered there at all. The first-party sandboxed components are always present too,
built into every binary under `Novis\` (`rule:packaging/the-first-party-components-are-built-in`), so
their tier stays visible at the use site and their presence is never a deployment question.

The failure this prevents is a program that compiles in development and fails to load in production because
an optional subsystem was not built or installed. That would make the reserved namespace
(`rule:core-api/reserved-namespace`) *conditional*, which is a worse outcome than an unfamiliar namespace:
`Core\X` would stop meaning "always there" and start meaning "there if someone installed it", and every
`use` of it would become a deployment question.

Attempting either — a loaded extension declaring a class under `Core\` or `Novis\` — is a load-time
diagnostic naming the class, not a silently missing symbol.
