- **A trailing `->` with a keyword on the next line is parsed as that keyword being the member
  name, not as recovery.** `$u->` followed by `if (true) {` is one `$u->if(true)` call, because a
  member name accepts a keyword the way PHP's does, so a case written to pin resilient behaviour
  ends up freezing an answer about `if` rather than about the arrow. Reach `MemberName::Missing` by
  leaving the arrow at end of file and putting the unclosed brace *above* it.
  [until: gone crates/nvs-syntax/src/index.rs:MemberName::Missing]
