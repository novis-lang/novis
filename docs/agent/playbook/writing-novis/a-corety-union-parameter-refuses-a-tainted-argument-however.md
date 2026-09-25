- **A `CoreTy::Union` parameter refuses a `tainted` argument however its arms are marked, so a
  `string|bytes` parameter is stricter than the `string` beside it.** `CoreTy::classification`
  answers `None` for a union (`crates/nvs-stdlib/src/registry.rs:1059`) and
  `rule:security/unclassified-parameter-refuses-tainted` makes an unclassified text-like parameter
  refuse one, so the marks written on the arms reach nothing — `Core\Socket` split `send` and
  `sendBytes` into two members for exactly this and says so in its row. Decide whether the member
  has to accept tainted data *before* spelling a union, and take that split if it does.
  [until: gone crates/nvs-stdlib/src/registry.rs:CoreTy::classification]
