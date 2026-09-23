- **A line-at-a-time brace counter drifts on a multi-line string literal.** `db.rs` holds one whose
  body is `… [capabilities.net]\nconnect = [\"127.0.0.1\"]\n`, and its unmatched `[` left the
  scanner at depth 1 for the rest of the file, so the last eight items looked like nothing at all. A
  splitter needs a stateful scan over the whole file — string, raw string, char, block comment — not
  a regex per line, and the reconstruction assertion is what turns that into a loud failure.
  [until: reviewed 2026-09-06]
