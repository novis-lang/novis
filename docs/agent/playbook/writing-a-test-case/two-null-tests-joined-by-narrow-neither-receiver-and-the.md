- **Two `== null` tests joined by `||` narrow neither receiver, and the `else` is where it bites.**
  `if ($a == null || $b == null) { … } else { $a->member(); }` is four `E0459`s — the narrowing runs
  per test, so only a nested `else if ($b == null)` leaves both receivers non-nullable in the final
  arm. Two descriptions from `Core\Reflect::forClass` in one case is the shape that meets it, and the
  nested spelling is what every such case is written in. [until: gone crates/nvs-diagnostics/src/lib.rs:E0459]
