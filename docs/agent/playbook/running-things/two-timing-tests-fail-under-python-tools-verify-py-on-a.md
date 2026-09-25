- **Two timing tests fail under `bun nv verify` on a loaded machine and pass on their
  own.** `nvs_stdlib`'s `http::socket::tests::socket_ping_keeps_a_quiet_live_peer_open` and
  `nvs-lsp`'s `a_warm_index_answers_within_the_reanalysis_bound` both assert a wall-clock bound —
  the second one reported 204.1 ms against a 200 ms ceiling — and `test` runs every binary side by
  side. Read the two lines `nv verify` prints under such a failure: it re-runs the binary alone and
  says so, and neither test is anything a stdlib or docs session touched.
  [until: gone crates/nvs-lsp/tests/latency.rs:a_warm_index_answers_within_the_reanalysis_bound]
