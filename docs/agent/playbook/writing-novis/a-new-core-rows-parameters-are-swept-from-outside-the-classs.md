- **A new `Core` row's parameters are swept from outside the class's module, once from another
  crate.** `registry.rs`'s `UNCLASSIFIED` is deletions-only, so a bare `CoreTy::Str` in an options
  bag fails `every_member_parameter_carries_a_qualifier_classification` and may not be listed beside
  the members already there — classify the bag `CoreTy::Text(Qual::…)`, which reclassifies everything
  sharing that slice. `crates/nvs-stdlib/tests/capability.rs`'s
  `an_open_file_is_an_object_and_never_a_resource` is the other: it sweeps every row for a
  `Core\IO\File` parameter, so such a member is green under `--lib` and red under
  `--test capability`. [until: reviewed 2026-09-16]
