- **A `?bool` cannot be tested for truth, so a member answering one has no `yn` rendering at all.**
  `if ($found as bool)` on a `?bool` panics `nvs-ir`'s truthy-condition slice with *"got Tagged"* —
  `as bool` does not narrow the binding out of `Ty::Tagged`, and the guarded-branch conversion that
  works for `?int` and `?string` has no counterpart because the condition is what fails. Keep a
  `bool`-valued subject out of a case about a `?T`-answering member, or render it through a member
  that answers `string`. [until: reviewed 2026-09-06]
