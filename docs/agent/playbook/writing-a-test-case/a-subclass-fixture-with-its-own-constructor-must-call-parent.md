- **A subclass fixture with its own constructor must call `parent::constructor(...)` on every path,
  or `E0410` fails it.** A `class Dog extends Animal { public function constructor(string $n, int
  $age) {} }` written to make `rule:classes/constructor-compatibility`'s case plausible is itself
  refused, so an `assert!(!diags.has_errors())` fails on a diagnostic the author never considered,
  worst in a negative fixture's control half. One `parent::constructor($n);` line fixes it.
  [until: gone crates/nvs-diagnostics/src/lib.rs:E0410]
