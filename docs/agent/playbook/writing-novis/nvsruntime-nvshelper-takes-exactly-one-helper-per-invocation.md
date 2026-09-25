- **`nvs_runtime::nvs_helper!` takes exactly one helper per invocation**, so a second `fn` written
  inside an existing block fails with `no rules expected #` pointing at that fn's own doc comment and
  at `$body:block` in `abi.rs`. Its one rule is `$(#[$meta])* fn $name(...) $body`, with no
  repetition around it, and the doc comment of the *second* fn is the first token it cannot match —
  which reads as a broken doc comment rather than as a block that should have been closed. Close the
  block after the helper you are writing beside and open a fresh `nvs_runtime::nvs_helper! {`.
  [until: gone crates/nvs-runtime/src/abi.rs:$body:block]
