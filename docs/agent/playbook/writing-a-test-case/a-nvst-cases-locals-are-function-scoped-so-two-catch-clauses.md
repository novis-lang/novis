- **A `.nvst` case's locals are function-scoped, so two `catch` clauses or two `foreach` bodies
  cannot share a declared name.** `catch (RuntimeError $error)` in one `try` and `catch (IOError
  $error)` in the next is `E0406: $error is already declared`, and the later `catch` resolves the
  name against the first clause's class. Name the second one differently, or declare once above and
  assign inside. [until: gone crates/nvs-diagnostics/src/lib.rs:Code::new("E0406")]
