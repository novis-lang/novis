- **A fake server that parses the client's message with a needle the value may contain is a coin
  flip, and the tell is an acceptance failure that does not reproduce.** `crates/nvs-db/src/pg.rs`'s
  SCRAM fake took the client nonce with `rsplit_once("r=")`, and a nonce drawn from RFC 5802's
  printable set can itself contain `r=`, so on a rare run the fake echoed a suffix and the driver
  rightly refused its own exchange. Split on the attribute's own delimiter — `,r=` — which the
  grammar guarantees the value cannot hold. [until: reviewed 2026-09-06]
