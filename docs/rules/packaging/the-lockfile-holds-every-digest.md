**`package.lock` records every package in the graph** — direct and transitive — with its name,
selected version, source and digest. `nvs build --locked`, and every CI invocation, fetches nothing
that is not in the lock and fails rather than updating it.

**A digest mismatch is a hard error** — never a warning and never a re-fetch — and the failure names
the package. The digest is the same BLAKE3 the artifact cache keys on, so a package's identity
(`rule:packaging/a-package-is-its-digest`) and its integrity check are one hash.

**`nvs vendor`** writes the whole resolved graph into the application tree, so a build needs no
network at all. Vendored bytes are digest-checked against the lock on every build, which makes
vendoring a convenience rather than a second trust root.

The lock is for integrity, not for determinism: `rule:packaging/resolution-takes-the-highest-minimum`
is deterministic on its own, and `nvs build --locked` on a clean machine with an empty cache produces
byte-identical compiled units to the machine that wrote the lock, on all three platforms.
