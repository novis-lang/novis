- **Adding one LSP request fails a closed-list test in a file no goal's file set names.**
  `crates/nvs-lsp/tests/handshake.rs`'s `initialize_declares_exactly_the_capabilities_this_goal_ships`
  asserts the *exact* serialized capability key set, so one new provider field fails it with
  `Extra: [...]` from a suite the slice never opened. Its comment also names each absent request and
  why it is absent, so landing one leaves that comment arguing against the tree. Edit the expected
  set and the comment together with the capability.
  [until: gone crates/nvs-lsp/tests/handshake.rs:initialize_declares_exactly_the_capabilities]
