Every dependency names the **minimum version** it needs, and the version selected for a package is the
**highest minimum any package in the graph asked for**, across majors too. That is the whole algorithm.

There is no solver, no backtracking and no unsolvable graph: resolution is a walk taking a maximum, so
it always succeeds, it is fast, and it is deterministic without the lockfile — the lockfile records
digests for *integrity*, not to pin a choice that would otherwise wobble
(`rule:packaging/the-lockfile-holds-every-digest`). Adding a dependency cannot move an unrelated one,
and the version you get is one somebody explicitly asked for, never the newest thing published this
morning, so a build that worked yesterday works today and a fresh clone matches CI. A graph where two
packages require different minimums of a third selects the higher, and no other package's version
changes.

**A higher version inside a major is always acceptable**, because a minor or patch release may not
break the API of the release before it (`rule:packaging/only-a-major-may-break-the-api`). **Across a
major, a break that matters is a compile error** naming the package, the member and both versions,
because the whole program is type-checked. `nvs pkg add` and `nvs pkg update` print when a
selection crosses a major and which package asked for the older one. A `0.0.x` dependency is an
ordinary minimum.

**Only a released version is selected.** A version still pending
(`rule:packaging/a-release-waits-seven-days-unless-an-owner-confirms`) is fetchable by its exact
version and is never chosen by `add` or `update`. A retracted version is not selected, and resolution
names its fix (`rule:packaging/a-known-bad-version-is-retracted-not-deleted`).

**Upgrades are explicit.** `nvs pkg update <package>` raises a minimum in `package.toml` and shows
the effect on the whole graph; `nvs pkg outdated` reports what is available. Nothing upgrades
itself, which is also why a security patch does not arrive on its own, and why retraction and
`nvs pkg audit` have to be genuinely good rather than an afterthought.

**Declined:**
- **A solver over version ranges.** A range can say "not 2.3.1, it is broken", and every incumbent has
  one. It brings back unsolvable graphs, slow resolution, and an error that blames the user for a
  conflict between two libraries they never read. Retraction covers the one case a range expresses
  that a minimum cannot (`rule:packaging/a-known-bad-version-is-retracted-not-deleted`).
- **Exact pins only, with no minimums.** Every transitive upgrade becomes a manual sweep across the
  whole graph, so a security patch spreads more slowly still.
- **A ceiling at the next major.** It makes a graph unsolvable whenever two packages want different
  majors of a third. The compile error names the real break, and only when the program uses it.
- **A root-only rule for `0.0.x` dependencies.** A `0.0.x` package promises nothing, and the compile
  error already names a break that matters.

**Not on disk.** Resolution, its crossings and the version states are
`crates/nvs-cli/src/package/resolve.rs`. `nvs pkg add`, `nvs pkg update` and `nvs pkg outdated`
choose only a released version (`crates/nvs-cli/src/package/minimum.rs`), and `add` and `update`
print the crossings in their report (`crates/nvs-cli/src/package/report.rs`).
