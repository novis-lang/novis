Nothing outside Tier 0 may register a class under the `Core` namespace, and nothing whose presence a build
flag can remove is registered there at all. A first-party sandboxed component is named like any other
component, under its own namespace, so its tier is visible at the use site.

The failure this prevents is a program that compiles in development and fails to load in production because
an optional subsystem was not built. That would make the reserved namespace
(`rule:core-api/reserved-namespace`) *conditional*, which is a worse outcome than an unfamiliar namespace:
`Core\X` would stop meaning "always there" and start meaning "there if someone installed it", and every
`use` of it would become a deployment question.

Attempting it is a load-time diagnostic naming the class, not a silently missing symbol.
