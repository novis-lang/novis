- **`ctx.take_pending()` is `""` for a `ParseError` thrown in a test context with no program.** A
  fault with issue slots becomes an exception object, and with no class table behind the context
  that object has no message slot to read. Compare the `Fault`'s own message instead, as
  `refused_in_one_pass` in `crates/nvs-stdlib/src/request.rs` does.
