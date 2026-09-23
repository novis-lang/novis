- **`CoreTy::Text(Qual::Contagious)` on a `void` member refuses the tainted argument the rule says
  it admits.** `nvs_types`' `admits_tainted_argument` (`crates/nvs-types/src/expr/quals.rs`) reads
  `Contagious` as "admits `tainted` only where the return type can carry the bit out", so a `void`
  row marked that way gives `E0401: expected string, found tainted string`. A rule's word for a body
  is not the enum's case for an answer: a writer that returns nothing is `Qual::Neutral`, as
  `Core\Cli::write`'s `string` arm is. [until: reviewed 2026-09-06]
