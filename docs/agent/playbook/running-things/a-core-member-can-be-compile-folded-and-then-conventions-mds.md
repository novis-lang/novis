- **A `Core` member can be compile-folded, and then `conventions.md`'s five edits reach half of
  it.** `nvs_types::links` rewrites `Core\Router::url` into `nvs_core_router_link(<prepared path>,
  $params)` because its answer needs the route table, so a sibling needing that table is also a
  change in `nvs-types`, `nvs-ir`'s `lower_route_link` and `nvs-hir`'s `ROUTER_SCAN_MEMBERS`. Grep
  a class for a `link`-style module before scoping a slice on it, and budget for that arm having to
  flatten a `CoreTy::Shape` by hand. [until: gone crates/nvs-types/src/links.rs:Core\Router::url]
