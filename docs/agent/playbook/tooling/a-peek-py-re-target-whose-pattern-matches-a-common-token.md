- **A `peek.py` `re:` target whose pattern matches a common token prints every hit in the file, and
  a big file answers with tens of thousands of lines.** `crates/nvs-host/src/isolate.rs:re:fn |struct
  |impl ` came back as 88 hits and 173 KB, spilled to a persisted file, and answered nothing the
  question had asked. Write the pattern so it can only match the construct you want — `re:fn
  over_socket|enum Charge` — or use `--outline <file>` and `--locate <symbol>`, which return seams
  and `file:line` anchors rather than bodies. [until: reviewed 2026-09-15]
