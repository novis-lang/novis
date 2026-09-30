- **One timing test fails under `bun nv verify` on a loaded machine and passes on its own.**
  `nvs_stdlib`'s `http::socket::tests::socket_ping_keeps_a_quiet_live_peer_open` in
  `crates/nvs-stdlib/src/http/socket.rs` needs each pong back inside a wall-clock window, and `test`
  runs every binary side by side. Read the two lines `nv verify` prints under such a failure: it
  re-runs the binary alone and says so, and the test is not one a stdlib or docs session touched.
  [until: gone crates/nvs-stdlib/src/http/socket.rs:socket_ping_keeps_a_quiet_live_peer_open]
