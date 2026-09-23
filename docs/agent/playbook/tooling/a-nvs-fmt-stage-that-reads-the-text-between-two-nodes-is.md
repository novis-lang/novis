- **A `nvs-fmt` stage that reads the text between two nodes is reading comments too.** The gap
  between two children of one production is trivia as well as code, so a scan for a separator in it
  finds the `,`, `{` or `=>` inside a comment — and an arm boundary taken from one puts a rewrite in
  the middle of a run the printer copies whole, which trips `print.rs`'s ordering assertion in debug
  and corrupts the file in release. Mask the trivia out with `indent.rs`'s `commented` before
  searching a gap, the way `arm_starts` and `imports.rs` both do.
  [until: gone crates/nvs-fmt/src/indent.rs:arm_starts]
