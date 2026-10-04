- **A goal record's check can name a test its crate may not *write*, and `Cargo.toml` naming the
  right dependency does not settle it — read the `[lints]` block underneath.** A fixture that needs
  a leaked `ClassTable` carrying an `invoke` address (`allocation_policy.rs`'s `callable_of`) cannot
  compile under the workspace's `unsafe_code = "forbid"`, which `#[expect(unsafe_code)]` cannot
  open. `grep -rn unsafe_code crates/*/Cargo.toml` lists the crates that chose `deny`; then ask
  which crate holds the state the claim is about before moving the check.
  [until: gone tools/nv/cmd/orient.ts:const TRIAGE]
