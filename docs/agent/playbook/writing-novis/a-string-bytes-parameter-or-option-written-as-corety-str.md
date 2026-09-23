- **A `string`/`bytes` parameter or option written as `CoreTy::Str` fails the
  qualifier-classification gate, under `-p nvs-stdlib --lib` only.**
  `every_member_parameter_carries_a_qualifier_classification` wants
  `CoreTy::Text(Qual::…)`/`Blob(Qual::…)` or a place on its `UNCLASSIFIED` roster, options in a
  `{…?}` bag included, and names the member rather than the parameter. A lookup key is
  `Qual::Neutral`, not `Qual::Sink` — `Sink` means the content becomes an instruction;
  `Core\Regex\Match::group` is the precedent.
  [until: gone crates/nvs-stdlib/src/registry.rs:every_member_parameter_carries_a_qualifier]
