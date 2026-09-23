- **A `-p nvs-server` test cannot ask for a second connection, and `run_until_idle` forbids it, not
  `|| ControlFlow::Break(())`.** It returns as soon as the core has nothing runnable, and an
  `accept` on a client not yet connected is that, so a `keep_serving` counting to two is a
  sixty-second silent hang. Assert server-side, where `serve_on_this_core` parks until every
  connection is done; `a_connection_whose_isolate_panics_is_contained` is the shape.
  [until: reviewed 2026-09-06]
