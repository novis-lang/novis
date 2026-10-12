Every dependency names the **minimum version** it needs. The version selected for a package is the
**highest minimum any package in the graph asked for**. That is the whole algorithm.

There is no solver, no backtracking and no unsolvable graph: resolution is a walk taking a maximum,
so it always succeeds, it is fast, and it is deterministic without the lockfile — the lockfile
records digests for *integrity*, not to pin a choice that would otherwise wobble
(`rule:packaging/the-lockfile-holds-every-digest`). Adding a dependency cannot move an unrelated
one, and the version you get is one somebody explicitly asked for, never the newest thing published
this morning, so a build that worked yesterday works today and a fresh clone matches CI.

**Upgrades are explicit.** `nvs update <package>` raises a minimum in `package.toml` and shows the
effect on the whole graph; `nvs outdated` reports what is available. Nothing upgrades itself — which
is also why a security patch does not arrive on its own, and why retraction and `nvs audit`
(`rule:packaging/a-known-bad-version-is-retracted-not-deleted`) have to be genuinely good rather
than an afterthought.

A minimum is a floor and never a ceiling, so the algorithm is sound only if a higher version is
always acceptable — which is what `rule:packaging/a-breaking-release-is-a-new-name` guarantees.

A graph where two packages require different minimums of a third selects the higher, and no other
package's version changes. No manifest exists for which resolution reports a conflict.

**Not on disk.** The tree has no package system and no resolver.

**Declined:**
- **A solver over version ranges.** A range can say "not 2.3.1, it is broken", and every incumbent has
  one. It brings back unsolvable graphs, slow resolution, and an error that blames the user for a
  conflict between two libraries they never read. Retraction covers the one case a range expresses
  that a minimum cannot (`rule:packaging/a-known-bad-version-is-retracted-not-deleted`).
- **Exact pins only, with no minimums.** Every transitive upgrade becomes a manual sweep across the
  whole graph, so a security patch spreads more slowly still.
