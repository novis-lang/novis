- **A layout rule `nvs fmt` learns is green only if the corpus already follows it.**
  `crates/nvs-fmt/tests/identity.rs` asserts every `.nvs` file under `examples/` and `tests/` comes
  back byte for byte, and that test runs in `nv verify`, so every
  later change has to keep it green. Before writing a rule into the printer, format the corpus with it and
  read what moves — a rule the corpus disagrees with lands *with* a corpus reformat or not at all.
  [until: gone crates/nvs-fmt/tests/identity.rs:the_identity_printer_reproduces_every_corpus_file]
