- **A sweep of `.nvs` files for a new checker refusal misses all 3090 conformance cases, because a
  `.nvst` file's program sits in its `--FILE--` section.** The two-phase constructor rule was ported
  over `docs/examples/` and `tests/hostile/`, and eight `tests/conformance/` cases still called
  `parent::constructor(...)` first; the next session found them red. `./target/debug/nvs test
  tests/conformance` runs the whole tree on the debug build in about a minute and prints one `FAIL`
  line per case, and `--filter <text>` reruns one. `bun nv suite --case` does not narrow the run.
  `until: gone tools/nv/cmd/suite.ts:--case`
