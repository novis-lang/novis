- **A `catch` binding has no methods at all, and every PHP accessor on one is `E0405`.** `catch
  (Throwable $e) { echo $e->getMessage(); }` is refused where it is written, with a help naming the
  property that answers the same question (`->message`);
  `nvs_types::expr::calls::report_exception_accessor` is that mapping's home. A case that wants to
  show what was thrown reads `$e->message`.
  [until: gone crates/nvs-types/src/expr/calls.rs:fn report_exception_accessor]
