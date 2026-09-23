- **A TLS handshake tunnelled in another protocol's frames buffers on `write`, frames on `flush`,
  and hands out its off-switch first.** `rustls` writes a flight in several calls and flushes after
  each (`ConnectionCommon::complete_io`), so framing each `write` cuts records at arbitrary offsets,
  and `StreamOwned` hands nothing back once `NvsTls::over` has the adapter, so the switch is shared
  before the handshake. `crates/nvs-db/src/tds/prelogin.rs`'s `Tunnel` keeps the flag in an
  `Rc<Cell<bool>>` and the caller takes its `TunnelEnd` before the handshake.
  [until: reviewed 2026-09-06]
