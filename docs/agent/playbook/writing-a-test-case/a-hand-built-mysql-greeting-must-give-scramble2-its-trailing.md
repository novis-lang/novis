- **A hand-built MySQL greeting must give `scramble_2` its trailing NUL, or the plugin name comes
  back one byte short.** `HandshakePacket::new` writes `auth_plugin_data_len = scramble_2.len() + 8`
  while the deserializer reads back `max(13, len - 8)` bytes, so a 12-byte tail round-trips as 13
  and eats the first character of `auth_plugin_name` — the refusal names `aching_sha2_password`,
  which reads like a typo in the driver. Pass `NONCE[8..]` plus a `0` byte;
  `HandshakePacket::nonce()` trims it back off. [until: gone crates/nvs-db/src/mysql.rs:HandshakePacket]
