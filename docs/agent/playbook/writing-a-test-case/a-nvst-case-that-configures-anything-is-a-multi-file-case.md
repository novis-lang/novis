- **A `.nvst` case that configures anything is a multi-file case, and `bun nv try` cannot
  run one.** `--FILE nvs.toml--` is the only way to grant a capability or write a `[db.*]` block to
  a case, and `bun nv try` concatenates the sections into one `.nvs`, so the TOML or a fixture's prose
  arrives as Novis source and the run dies in dozens of parse errors (`error[E0319]: disk is not a
  constant that exists`). The shipped runner takes one path or a list and prints the same
  expected/actual diff, and is already built when the session opens. [until: gone tools/nv/cmd/try.ts:--FILE--]
