A durable row is an output, and an output refuses `secret` (`rule:security/secret-qualifier`). So a
`secret` cannot enter a job payload — the enqueue does not compile, and its help names configuration
(`crates/nvs-types/src/expr/quals.rs:1357`). A job that needs a credential reads it from configuration
when it runs, which is where credentials live anyway and which keeps them out of a table, a backup and
a replica.

Every text a job reads out of its payload is `tainted` (`rule:security/tainted-qualifier`), whether it
was enqueued `tainted` or clean. `Core\Script::args()` answers `mixed`, and text out of `mixed` is
`tainted` (`rule:security/taint-propagation`), so the analysis survives the round trip instead of being
laundered by a database. This is the trust every other stored value gets: a row's `get`,
`Core\Cache::get` and `Core\Session::get` answer `mixed` too. A job recovers a number, a flag or an enum
case with the checked `as` that removes the qualifier on success, and free text with the sink's own
launderer or `rule:security/assert-trusted`. Nothing is recorded in the row and nothing is checked at
run time.

- **Memory:** none. The qualifier is erased before codegen.

**Declined:**
- **Recording each value's qualifiers in the row and restoring them on decode.** The restored mark is
  `rule:security/taint-propagation`'s Declined *hidden, source-marked `mixed`*: it is lost at the first
  `mixed` parameter or property, it cannot be written in a declaration, and it puts a tag on every
  payload value at run time. A clean value coming back `tainted` over-taints, which is the safe
  direction, and one checked `as` clears every non-text field.
