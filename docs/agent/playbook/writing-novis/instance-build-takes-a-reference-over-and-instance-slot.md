- **`instance::build` takes a reference over and `instance::slot` lends one, so parking a borrowed
  slot value into a new instance double-releases.** Every other `Core\Db` member only borrows the
  receiver's block name in passing, so none shows that a member keeping the value owes a `retain()`;
  the program exits 127 with nothing on stderr and its stdout byte-perfect. Grep for a
  `crate::instance::build` whose slot values are not all freshly constructed;
  `crates/nvs-stdlib/src/db/stream.rs` states the rule. [until: gone crates/nvs-stdlib/src/db/stream.rs:crate::instance::build]
