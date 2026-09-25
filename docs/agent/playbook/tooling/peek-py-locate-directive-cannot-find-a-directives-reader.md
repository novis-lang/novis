- **`peek.py --locate <directive>` cannot find a directive's reader, because a directive is a string
  key and a reader is a symbol under some other name.** `[limits] max_output` read as "unread by
  anything in the tree" for a whole goal on that evidence, while `Ctx::output_limit` had been reading
  it through `configured_bytes("max_output")` since it landed. Locate a *symbol*; find a directive
  with `peek.py "crates/**/*.rs:re:<key>"`, which reads the string literals too.
  [until: gone tools/peek.py:--locate]
