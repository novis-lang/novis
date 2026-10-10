- **A `Core` row that spells its callback's signature does not on its own hand
  `check_anon_fn` a substituted expected type.** `nvs_types::expr::args`' `check_generic_args`
  checks every argument but the options bag in its **first** pass, before `sig.substituted`, so an
  anonymous function at a parameter mentioning a type variable is checked against `unplaced_expectation`'s
  answer — nothing — and an unannotated parameter still has nothing to take. Read that function's
  three passes before budgeting the registry rows: the callback argument has to be deferred the way
  the bag is, in a slice of its own.
  [until: gone crates/nvs-types/src/expr/args.rs:unplaced_expectation]
