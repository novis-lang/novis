- **A `spawn script` helper's `Err` edge must not release the transferred `args:` value: the
  caller's fault edge already does.** `lower_spawn_script` is the one `CoreCall` site that skips
  `forget_transferred_since`, so the temporary is still on the stack the landing block sweeps —
  while that function's own doc and every ordinary call site say the opposite, and are what a reader
  finds first. Add no release on a new `Err` path out of `Host::start_isolate` or either spawn
  helper; `crates/nvs-ir/src/lower/expr.rs:3243` proves it, and `copy_graph` leaves the root's
  reference alone for the same reason.
  [until: gone crates/nvs-stdlib/src/script.rs:let crossing = args]
