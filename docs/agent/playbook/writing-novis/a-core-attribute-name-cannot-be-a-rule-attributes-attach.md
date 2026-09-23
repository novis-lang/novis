- **A `Core` attribute name cannot be a `rule:attributes/attach-sites-and-forms` shape alias,
  because there is no `Core`-seeded alias table.** `nvs_hir::AliasTable` is collected from source
  `type` declarations only (`crates/nvs-hir/src/aliases.rs`), so `Core\Command` reaches
  `nvs_types::attributes::resolve_shape_alias` with no entry and the answer is `E0726: … is not a
  `type` alias` — a refusal no stdlib edit can lift. Every `Core`-owned attribute is a nominal match
  on `nvs_types::derive::ATTRIBUTES` and owes a pass that checks its payload, or `#[Command(nmae:
  "x")]` is admitted in silence. [until: reviewed 2026-09-06]
