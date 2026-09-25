- **Two `foreach` headers in one file may reuse a binding name only at the same type.** A binding is
  function-scoped, so `foreach ($names as string $n)` followed by `foreach ($heap as int $n)` is
  `E0406: `$n` is already declared` at the *second* header, while a second `string $n` walk is fine
  — and a typed declaration *inside* one loop body is fine too, declared once and assigned each
  iteration. A case walking two differently-typed collections needs two names.
  [until: gone crates/nvs-diagnostics/src/lib.rs:E0406]
