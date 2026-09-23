- **Widening a `CoreTy::Union` a `Core` member answers breaks frozen spellings of it in two suites,
  and neither failure names the union.** A `.nvst` quotes the rendering inside a diagnostic about
  something else, and `nvs_types::core_lib`'s `a_verified_signature_does_not_launder_its_claims`
  freezes every `tainted`-answering member's rendered return type. Grep `tests/` and `crates/` for a
  slice of the old rendering first, and take the new one from the binary rather than composing it —
  the order is the interner's. [until: reviewed 2026-09-08]
