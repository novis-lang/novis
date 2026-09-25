- **Classifying a `Core` class can break that class's own structural unit test, and the
  `rule:security/sink-predicate` ratchet says nothing about it.** `Core\Test`'s rows share one
  `MESSAGE: &[CoreOption]`, and `test::tests::the_only_option_is_a_message_that_defaults_to_absent`
  asserts `matches!(bag[0].ty, CoreTy::Str)` over every one, which `CoreTy::Text(Qual::Neutral)` is
  not — so the registry-wide `every_member_parameter_carries_a_qualifier_classification` passes
  while a test naming a member you did not think you were editing fails. Grep the class's own `mod
  tests` for `CoreTy::Str` before classifying it. [until: gone crates/nvs-stdlib/src/registry.rs:every_member_parameter_carries_a_qualifier_classification]
