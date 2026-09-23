- **`nvs_host::run_until_idle` returns while your task is still parked, and a server is the first
  caller for which that is wrong.** It breaks out as soon as one blocking poll wakes nothing — which
  a connection's socket reports on Windows once its owning task retired — so `spawn` + one
  `run_until_idle` serves one request and exits `0` silently. Loop while `RunReport::parked > 0`, as
  `crates/nvs-cli/src/serve.rs` does; the stale readiness is delivered once and does not spin.
  [until: reviewed 2026-09-06]
