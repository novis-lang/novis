- **A TDS request counted in packets is not a request.** A case that splits `Script`'s recorded
  bytes at every header and counts requests passes on short values and fails the moment one is
  `nvarchar(max)`, because one `sp_prepexec` past the negotiated packet size leaves as several
  packets. Reassemble to `Status::EOM` before counting — `crates/nvs-db/src/tds/testing.rs`'s
  `flushed` is the helper — and keep the first packet's status, since that is the one
  `Status::RESET_CONNECTION` rides. [until: gone crates/nvs-db/src/tds/testing.rs:fn flushed]
