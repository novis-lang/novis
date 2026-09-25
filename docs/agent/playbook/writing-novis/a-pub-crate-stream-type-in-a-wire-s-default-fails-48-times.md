- **A `pub(crate)` stream type in a `Wire<S>` default fails 48 times in `nvs-stdlib` and never names
  the cause.** `MySqlRows<'a, S = MyStream>` is public, so the default is part of its signature and
  every stdlib line holding rows becomes a `private type` error pointing at the caller. Make the
  stream `pub` and re-export it beside the rows type in the same edit — `PgRows` and `TdsRows` carry
  the same default. [until: gone crates/nvs-db/src/lib.rs:MyStream]
