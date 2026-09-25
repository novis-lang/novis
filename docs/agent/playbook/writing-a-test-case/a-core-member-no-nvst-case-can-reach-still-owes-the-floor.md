- **A `Core` member no `.nvst` case can reach still owes the floor three cases, so write its logic
  as a free function over the carrier and pin it twice.** Every `Core\Request` member refuses in a
  case, so the corpus pins only the refusal; the behaviour is pinned by `#[cfg(test)]` tests
  building an `Inbound`, which needs a free function taking `&Inbound`, not a body in `nvs_helper!`.
  `crates/nvs-stdlib/src/request.rs`'s `cookie_of` is the shape. [until: gone crates/nvs-stdlib/src/request.rs:Core\Request]
