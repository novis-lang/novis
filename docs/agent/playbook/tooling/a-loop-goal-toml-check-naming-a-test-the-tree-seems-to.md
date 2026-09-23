- **A `loop-goal.toml` check naming a test the tree seems to already have can want a *second* test,
  because the older name is itself an earlier check's.** Stage 5's
  `connection_defaults_are_finite_for_every_field` reads exactly like a rename of
  `every_connection_bound_is_finite_with_nothing_configured`, which stage 1's `-p nvs-server` check
  names verbatim — so renaming would have turned an earlier stage red to close a later one. Grep the
  whole of `loop-goal.toml` for the name the tree already has before renaming anything, and where
  both names are claimed, the claim the two differ on is what the second test asserts.
  [until: reviewed 2026-10-11]
