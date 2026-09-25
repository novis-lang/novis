- **A `System` block is still readable by a request — it is `Core\Config::set` that refuses one, not
  `get`.** `rule:observability/metrics-and-trace-blocks-are-system` makes the whole `[trace]` block
  `System`, which reads like "a request cannot see it" and is not: `nvs_config::Request::get`
  answers off the snapshot's own table for any dotted key, and only `set` consults the directive's
  class and returns `false` for `Class::System`. `ctx.config().and_then(|c|
  c.get("trace.propagate"))` is the whole read, the shape `http.rs`'s `bound_of` and `redirects_of`
  already use; what `System` buys is that a request cannot *change* it. [until: gone crates/nvs-config/src/request.rs:Class::System]
