- **A `Core` row that spells its callback's signature does not on its own hand
  `check_fn_literal` a substituted expected type.** `nvs_types::expr::args`' `check_generic_args`
  checks every argument but the options bag in its **first** pass, before `sig.substituted`, so a
  closure at a parameter mentioning a type variable is checked against `unplaced_expectation`'s
  answer — nothing — and an unannotated parameter still has nothing to take. Read that function's
  three passes before budgeting the registry rows: the callback argument has to be deferred the way
  the bag is, and that is a slice of its own rather than a line in the row's.
  [until: reviewed 2026-10-07]
