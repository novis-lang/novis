- **A test-only helper in `nvs-stdlib` goes inside the module's existing `#[cfg(test)] mod tests`, never under a second `#[cfg(test)]` of its own.**
  `crates/nvs-stdlib/tests/capability.rs` and scans such as `cache::tests`' carrier count read a
  source file only up to its first `#[cfg(test)]` line, so a second one fails the gate scan by name
  and makes the module's own scans count zero sites, which then poisons every other test that shares
  the module's lock. Make the test module `pub(crate)` and reach the helper through it, as
  `crate::cache::tests::close_shared` does. `until: gone crates/nvs-stdlib/tests/capability.rs:can no longer stop at the first one`
