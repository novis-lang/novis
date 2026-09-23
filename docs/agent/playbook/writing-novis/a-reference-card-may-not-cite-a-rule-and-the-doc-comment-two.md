- **A reference card may not cite a rule, and the doc comment two lines above it must.**
  `registry::tests::no_registry_card_cites_an_adr` refuses a citation in any `MethodDoc`
  `short`/`ret`/`desc`, `ParamDoc`, `ErrorDoc`, `CoreConst::desc` or `EnumDoc` case, because `nvs
  meta --json` ships the card verbatim to a reader with no rules tree, and it surfaces only at `-p
  nvs-stdlib --lib`. State the fact in the card and leave the citation in the `///` beside it.
  [until: gone crates/nvs-stdlib/src/registry.rs:no_registry_card_cites_an_adr]
