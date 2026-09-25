- **Nothing inside `nvs_server::serve_connection`'s service closure may park on the request body;
  the deadlock is `hyper`'s shape.** The h1 dispatcher runs `poll_read` then `poll_write` on one
  task and the service future is what `poll_write` polls, so a `block_on` over the body suspends the
  coroutine that owes the next `poll_read`, for the smallest body too. The service is a future
  answering `Pending` while the isolate runs as a peer task, so a pull may park the isolate but
  never the connection's own task. [until: gone crates/nvs-server/src/serve.rs:serve_connection]
