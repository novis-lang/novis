- **`nvs_db::encode` is PostgreSQL's text input format, not a driver-neutral rendering, and another
  driver reusing it gets quietly wrong rows.** It looks neutral — a `Value` in, `Option<Vec<u8>>`
  out, both drivers binding `&[Option<&[u8]>]` — but `bool` renders `t`/`f`, `bytes` as `bytea` hex
  and non-finite floats as `Infinity`/`NaN`, none of which errors on MySQL. Use the driver's own
  encoder (`nvs_db::mysql::encode`, `nvs_db::tds::encode`) and its `Dialect`; a second driver's
  `query` is the dialect and the encoder, not one branch in the drain. [until: gone crates/nvs-db/src/mysql.rs:encode]
