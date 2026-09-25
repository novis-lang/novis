- **An `nvs-config` census test cannot re-derive the registry's lookup, because
  `directive::governs` is `pub(crate)`.** A case wanting to show that a row is load-bearing —
  that `http.client.pool_idle_timeout` would fall through to the `http` blanket without one of
  its own — cannot filter `DIRECTIVES` by the boundary test, and widening that function for a
  test would put the lookup rule's one implementation behind a public name. Ask `lookup` a near
  miss instead: `http.client.pool_idlex` must resolve to `http` and not to `http.client.pool_idle`,
  which pins the same dot-boundary claim through the API the registry already exports.
  [until: gone crates/nvs-config/src/directive.rs:pub(crate) fn governs]
