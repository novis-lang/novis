- **`bun nv try` runs a scratch `.nvst`'s `--FILE--` and nothing else, so a case whose claim
  needs a request or a sibling file answers the wrong question there.** `--GET--`, `--HEADERS--` and
  `--FILE <path>--` are the conformance runner's sections: under `bun nv try` the child script never lands
  beside the program and `Core\Request` refuses exactly as it does in any CLI program, which reads as a
  broken case rather than as a runner that was never asked. Iterate such a case with
  `target/debug/nvs.exe test <path/to/case.nvst>`, which takes one file as readily as a directory.
  [until: gone tools/nv/cmd/try.ts:--FILE--]
