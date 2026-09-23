- **`options` is a reserved parameter name, and a row using it positionally fails a registry gate
  whose message reads as a tautology.** `registry::OPTIONS_NAME` is the trailing bag's name under
  `rule:core-api/shape-rules` R2, so `every_registry_row_names_one_parameter_per_positional_slot`
  reports "gives a positional parameter the trailing bag's own name" with `left: "options"` /
  `right: "options"`. Rename the positional parameter (`select`'s list is `$choices`) and fix the
  rule's own table in the same slice.
  [until: gone crates/nvs-stdlib/src/registry.rs:every_registry_row_names_one_parameter]
