- **A negative fixture naming call hierarchy fails a guard two files away.**
  `crates/nvs-lsp/tests/index.rs`'s `call_hierarchy_is_not_answered` greps every source file in the
  crate for `callHierarchy` and `CallHierarchy`, so `prepareCallHierarchy` written as the unknown
  request in `case.rs`'s closed-set test fails with a message about a second index nobody asked for.
  When a test needs a request name that will never be answered, pick a real LSP method no rule has an
  opinion on — `moniker` is what that test uses now — rather than the one name a rule guarantees will
  never be ours. [until: gone crates/nvs-lsp/tests/index.rs:fn call_hierarchy_is_not_answered]
