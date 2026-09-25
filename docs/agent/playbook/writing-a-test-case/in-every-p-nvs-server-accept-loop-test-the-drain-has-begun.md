- **In every `-p nvs-server` accept-loop test the drain has begun before the connection's isolate
  runs a line, so anything keyed off `Draining::is_draining()` fires on its first wait.**
  `serve_on_this_core`'s `keep_serving` is `|| ControlFlow::Break(())` in all of them, so the loop
  breaks and calls `draining.begin()` before the child it spawned has run. Make the drain cap the
  wait with a period and the close what the timeout becomes. [until: gone crates/nvs-server/src/serve.rs:serve_on_this_core]
