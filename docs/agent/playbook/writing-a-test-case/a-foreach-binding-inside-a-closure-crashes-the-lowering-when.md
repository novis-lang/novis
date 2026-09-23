- **A `foreach` binding inside a closure crashes the lowering when a binding of the same name is in
  scope outside it.** `nvs_types` resolves the inner name to the outer binding and records it as a
  capture, so `nvs-ir` panics with `the closure at 0:NN..NN captures `$name`, which is not bound in
  the enclosing frame` rather than reporting anything a case could expect. Give the inner binding its
  own name — no `--EXPECTF-ERROR--` catches this, because the failure is the compiler's process
  exiting 101 and not a diagnostic. [until: reviewed 2026-09-15]
