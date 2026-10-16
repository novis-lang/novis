`nvs check` warns with `W1023` when a condition's type is a union with `null` in it, naming `!= null`
as the fix. `?bool` warns too, since `null` and `false` are both false. A bare `null` and `mixed` do
not warn. Every position `check_condition` (`crates/nvs-types/src/expr/mod.rs`) covers raises it:
`if`, `while`, `do`/`while`, a `for` header's middle clause and a ternary's condition.

Both `null` and `0` are falsy, so `if ($s as ?int)` is false for an invalid value *and* for a valid
zero — reconstructing precisely the `(int)$x > 0` defect that nullable conversion exists to remove.

It is a warning and not a diagnostic because **the hazard is not new and not specific to the
operator**: any `?int` in an `if` has always had it. Refusing it would narrow
`rule:expressions/truthy-positions` for every nullable value in the language, which is a larger change
than this one.

**Declined:**
- **Refusing a `?T` condition outright.** It is the strongest guard against the trap, but it changes
  `rule:expressions/truthy-positions` for every nullable value rather than for conversions.
- **Warning only where the non-`null` half has a falsy value.** A `?User` cannot be confused with a
  falsy object, but one rule for every `?T` is simpler to learn, and `!= null` reads the same for
  each.
