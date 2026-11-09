- **A seed corpus cannot live under `fuzz/corpus/`: `.gitignore` ignores that whole directory.** It is
  where a `cargo fuzz run` inside the tree grows its corpus, and a file inside an ignored *directory*
  cannot be brought back by a negation pattern. Put seeds in `fuzz/seeds/<target>/`: `bun nv fuzz` reads
  them, and `crates/nvs-fuzz-checks`'s tests replay them.
  `until: gone .gitignore:/fuzz/corpus`
