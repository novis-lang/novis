- **A `cargo-named` acceptance check can report a test red that passes on every re-run, and the
  driver's own overlapped release prebuild is why.** `nvs-server`'s `serve` tests bind real loopback
  sockets and hold a connection to a memory ceiling, so they are timing-sensitive in a way the rest
  of the workspace is not, and a check run while `cargo build --release` saturates the machine can
  miss a frame that arrives every other time. Before treating a red *named* test as the regression
  that outranks your item, run that one test and then its whole crate suite — if both pass, it was
  load and the ledger's next line will not repeat it. [until: reviewed 2026-09-07]
