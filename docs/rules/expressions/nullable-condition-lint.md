`nvs check` warns when a `?T` is used directly as a condition, naming `!= null` as the fix.

Both `null` and `0` are falsy, so `if ($s as ?int)` is false for an invalid value *and* for a valid
zero — reconstructing precisely the `(int)$x > 0` defect that nullable conversion exists to remove.

It is a warning and not a diagnostic because **the hazard is not new and not specific to the
operator**: any `?int` in an `if` has always had it. Refusing it would narrow
`rule:expressions/truthy-positions` for every nullable value in the language, which is a larger change
than this one.

**Not on disk.** No warning code is allocated, and `check_condition`
(`crates/nvs-types/src/expr/mod.rs:199`) reports nothing for a `?T` condition.

**Declined:**
- **Refusing a `?T` condition outright.** It is the strongest guard against the trap, but it changes
  `rule:expressions/truthy-positions` for every nullable value rather than for conversions.
