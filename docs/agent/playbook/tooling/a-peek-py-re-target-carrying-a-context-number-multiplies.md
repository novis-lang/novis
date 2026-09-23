- **A `peek.py` `re:` target carrying a context number multiplies that context by every file the
  glob matches.** `"crates/nvs-stdlib/src/*.rs:re:Core.Request:2"` came back as 45 KB in one call,
  because the two lines of context are applied per hit and a crate-wide glob had dozens of them.
  Ask for the matching line alone first — a bare `re:pat` — and add the context number only once
  the target has narrowed to one file. [until: reviewed 2026-09-17]
