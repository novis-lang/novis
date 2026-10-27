- **A prepared `Core` member has two bodies, and the registry names the unfolded one.**
  `nvs_stdlib::registry`'s `symbol` for `Core\Router::url` is `nvs_core_router_url`, the path a
  method reference passed as a value takes, while `nvs_ir::lower::expr::lower_route_link` swaps a
  written reference for a prepared path and calls the second symbol, `nvs_core_router_link`. For any member
  `rule:expressions/intrinsic-constant-arguments` prepares rather than folds, grep the crate for a sibling
  symbol before believing the body the row points at.
  `until: gone crates/nvs-stdlib/src/router.rs:nvs_core_router_link`
