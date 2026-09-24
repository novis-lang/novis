- **Adding a `// covers:` marker to a case under `tests/conformance/reject/` moves every line number
  its `--EXPECTF-ERROR--` block pins.** That block reproduces each diagnostic's `--> case.nvs:NN:CC`
  anchor, counted from the `<?nvs` of the `--FILE--` block, so one inserted comment renumbers all of
  them while the case still reads as untouched. Put the marker and the `NN + 1` bumps in one
  `bun nv splice` patch, then run `target/debug/nvs.exe test <path.nvst>` before believing it.
  [until: reviewed 2026-09-20]
