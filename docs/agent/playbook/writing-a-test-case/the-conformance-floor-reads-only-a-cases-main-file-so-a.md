- **The conformance floor reads only a case's main `--FILE--`, so a member called only in an
  auxiliary file counts for nothing.** `crates/nvs-stdlib/tests/corpus/mod.rs`'s `sources()` takes
  the last, unnamed section deliberately, so a member only a child isolate can call, written into
  three `--FILE child.nvs--` sections, still reads as zero cases. Close it with a question the
  *parent* asks in its own section, such as relaying the child's answer back through `args:`.
  [until: gone crates/nvs-stdlib/tests/corpus/mod.rs:The section alone]
