- **A `cargo-named` check's *test name* can prescribe a design the cited rules already decided
  against, and then the name is what moves.** `..._are_declared_shape_parameters_...` asked
  `Core\Queue::push` for two whole shape parameters, which `rule:core-api/options-bag`'s
  one-optional-tail and `rule:concurrency/queue-four-members` each forbid alone. A name specifies
  only where the rules left the question open, so rename the check — in `loop-goal.toml` and the
  goal's own copy — rather than bending the surface to it. [until: reviewed 2026-09-10]
