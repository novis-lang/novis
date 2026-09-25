- **A `.nvst` case can carry its own `nvs.toml`, so the corpus already pins how a command behaves
  under a configuration — grep `--FILE nvs.toml--` before making one read the tree.** Every case
  spawns the real binary with the case directory as its working directory
  (`crates/nvs-test/src/run.rs`), so hoisting a catchable runtime refusal to compile time is a
  language change the corpus notices:
  `tests/conformance/core/db-open-asks-the-grant-about-the-host-and-then-the-address.nvst` expects
  the runtime refusal its program catches. [until: gone tests/conformance/core/db-open-asks-the-grant-about-the-host-and-then-the-address.nvst:--FILE nvs.toml--]
