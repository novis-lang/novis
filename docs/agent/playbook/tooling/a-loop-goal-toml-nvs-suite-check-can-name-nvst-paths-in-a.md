- **A loop-goal.toml `nvs-suite` check can name `.nvst` paths in a directory layout the corpus never
  adopted.** The failure reads exactly like unwritten work; the corpus is flat directories with a
  subsystem prefix on the file name (`tests/conformance/core/io-…`), not `tests/conformance/io/…`,
  and a drafted name describes the *claim*, which is usually already pinned under the name the
  corpus took — sometimes as an `nvs-stdlib` `#[test]`, when it is a panic no `.nvst` can survive.
  `sed -n 2p` the plausible neighbours before writing a case to satisfy a name.
  [until: reviewed 2026-09-06]
