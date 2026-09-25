- **An `[unread: … owner: …]` trailer counts only on the doc comment's *last* line, and goal
  `config-is-written`'s own example spans two.** `tools/directives.py` searches the last non-empty
  line of the field's doc comment, so a trailer wrapped across two `///` lines is reported as
  "something shaped like a trailer that this does not read" rather than accepted. Write the whole
  trailer on one line whatever its length: `rustfmt.toml` sets no `wrap_comments`, so a 200-column
  doc line survives `cargo fmt` untouched. [until: gone tools/directives.py:config-is-written]
