- **A layout rule `nvs fmt` learns is green only if the corpus already follows it.**
  `crates/nvs-fmt/tests/identity.rs` asserts every `.nvs` file under a tree of
  `tools/data/nvs-fmt-corpus.txt` comes back byte for byte, and that test runs in `nv verify`, so every
  later change has to keep it green. A rule lands with `bun nv nvs-fmt` in the same commit: read what
  moves, and bless or fix by hand each expectation file it names.
  `until: gone crates/nvs-fmt/tests/identity.rs:the_identity_printer_reproduces_every_corpus_file`
