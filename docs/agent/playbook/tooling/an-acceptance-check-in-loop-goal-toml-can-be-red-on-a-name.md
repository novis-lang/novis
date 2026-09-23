- **An acceptance check in `loop-goal.toml` can be red on a *name* rather than on a claim.** A check
  can list tests under names predicted before anyone wrote them, while the implementation landed
  under a name stating the narrower, true claim (`appending_into_spare_capacity_allocates_nothing`,
  not `appending_to_a_uniquely_owned_string_does_not_reallocate`). Before writing anything, `grep -n
  "    fn " crates/<crate>/src/<file>.rs` for the claim; correct the *list* when the tree's name is
  truer and say so in the toml comment — a predicted name is status, not a decision.
  [until: reviewed 2026-09-06]
