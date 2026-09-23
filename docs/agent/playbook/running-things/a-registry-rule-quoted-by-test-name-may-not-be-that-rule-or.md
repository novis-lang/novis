- **A registry rule quoted by test name may not be that rule, or may not exist.** A handoff can cite
  `a_union_is_only_ever_a_parameter` for "a union cannot be a return type" when the real test is
  `a_union_option_excludes_null`, which restricts an option's type and nothing else, and
  `CoreTy::Union`'s doc says "legal in either direction" outright. One `grep -n 'fn [a-z_]*('
  registry.rs` over the test names settles it; designing around a constraint that is not there costs
  a member's whole surface. [until: reviewed 2026-09-06]
