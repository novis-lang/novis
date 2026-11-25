- **Some source files are CRLF, and a script edit that matches `\n` finds nothing or mixes endings.**
  In `extend-blocks` stage 4, an edit script found no match in `crates/nvs-types/src/lib.rs`, and
  a `/^…$/m` regex wrote LF lines into CRLF `lower.rs` and `signatures.rs`, which cost a revert.
  A script normalises `\r\n` to `\n`, edits, and writes `\r\n` back when the file had it. The Edit
  tool handles both. Check with `file <path>`. See `crates/nvs-types/src/lib.rs`.
