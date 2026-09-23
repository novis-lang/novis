- **`nvs_helper!` takes exactly one function per invocation, and a second one inside the block fails
  naming the *next* member's doc comment rather than yours.** An added member written above an
  existing one produces ``error: no rules expected `#` `` pointing at that member's `///` line, with
  nothing pointing at what you wrote; the macro's rule in `crates/nvs-runtime/src/abi.rs` is a
  single `fn`, no repetition. Close your block and open a new `nvs_runtime::nvs_helper! { … }`,
  which is why `cli.rs` has several invocations rather than one. [until: reviewed 2026-09-06]
