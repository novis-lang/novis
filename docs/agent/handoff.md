# Handoff

## State

**ADR 0042's write path is on disk** as `crates/nvs-cli/src/cache.rs`, closing two of the driver's
six stage-5 tests: `the_cache_is_a_fan_out_of_immutable_content_addressed_files` and
`a_concurrent_write_resolves_by_rename_with_no_lock_file`. § 1's `<dir>/<key[0..2]>/<key[2..]>.nvsc`
fan-out, § 2's 78-byte header (`magic | format_version: u16 | env_hash | payload_len: u64 |
checksum`, little-endian because `env_hash` already covers the target triple), and § 4's temp-file →
`fsync` → one `rename` with the exists-check immediately before the rename. `nvs-cli` gained
`blake3` and `rand`. The module doc owns every one of those choices.

**The tests live in `src/cache.rs`'s own `#[cfg(test)] mod tests`, not in `tests/cache.rs`, because
`nvs-cli` has no library target** — its `tests/meta.rs` and `tests/openapi.rs` drive the built
binary, which is right for a command's output and impossible for a module with no command in front
of it. `cargo test -p nvs-cli` runs them, which is what the acceptance check spells. The module
carries `#![allow(dead_code)]` for the same reason its next slice exists, stated at the attribute.

**The blocker in front of the read path is not a decoder — it is that nothing can yet produce a
payload.** `cranelift_jit::JITModule` has no serialization (`crates/nvs-codegen/src/lib.rs:275`
holds it for the process's lifetime and § 7 of that crate doc says executable memory is never
freed), and the workspace has no `mmap` dependency at all — `crates/nvs-host/src/stack.rs:11` only
*mentions* `mmap`, for guard pages. So § 3's "verify fully, then `mprotect`" has neither the bytes
to map nor the call to make them executable. Writing
`an_artifact_is_verified_whole_before_any_page_is_executable` over an `fs::read` into a `Vec` would
pass while asserting nothing, which is the proxy-for-a-gate move the goal file forbids. The next
session decides what a payload is before it decodes one.

`orient.py`'s `[context] modules` still names `crates/nvs-host/src/budget.rs`, which never existed;
the pack warns every session and the manifest has not been fixed.

## Next group

**ADR 0042's read half, and the decision in front of it.** File set: `crates/nvs-cli/src/cache.rs`
(whole, ~395 lines — `Header::encode` is what a decoder inverts), with
`crates/nvs-codegen/src/lib.rs:275` and `crates/nvs-config/src/trust.rs:110` behind it.

- [ ] **Decide what a cache payload is, and record it** — the design call the next slice cannot
      start without. `cranelift_jit` does not serialize a module, so either the payload is
      `cranelift-object`'s relocatable object plus a relocation pass, or stage 5 ships its
      verify-and-map half over a payload ADR 0048's bundler already produces. Record it in
      `crates/nvs-cli/src/cache.rs`'s module doc under a *Known gaps* heading per the goal's
      standing decision, and put any redesign in `## Backlog` rather than starting one.
      Anchors: `crates/nvs-cli/src/cache.rs:1`, `crates/nvs-codegen/src/lib.rs:275`.
- [ ] **The read path, verified whole before a single page is executable** — ADR 0042 § 3, once the
      item above says what is being mapped: `mmap` `PROT_READ`, check magic / `format_version` /
      `env_hash`, `BLAKE3` the payload, and only then `mprotect`. A mismatch deletes the file and is
      a miss, never an error or a `FATAL`. Adding `memmap2` is a dependency call pre-authorized
      under ADR 0051 § 4. Closes `an_artifact_is_verified_whole_before_any_page_is_executable` and
      `a_tampered_artifact_is_rejected`.
      Anchors: `crates/nvs-cli/src/cache.rs:110`, `crates/nvs-cli/src/cache.rs:184`.
- [ ] **The cache directory's trust check, and eviction off the request path** — ADR 0042 §§ 5-6.
      `nvs_config::trust::check` is the same check ADR 0103 § 6 applies, so this is a call and not a
      second implementation; eviction is the probabilistic walk on the miss path only, so a warm hit
      never lists a directory. Closes `a_world_writable_cache_directory_is_refused` and
      `eviction_is_piggybacked_and_off_the_request_path`.
      Anchors: `crates/nvs-config/src/trust.rs:110`, `crates/nvs-cli/src/cache.rs:184`.

## Backlog

- What a serialized compiled unit is — the payload ADR 0042 § 2 carries and § 8 assumes.
- `[context] modules` names `crates/nvs-host/src/budget.rs`, which does not exist —
  `docs/agent/loop-goal.toml`.
- `tools/bench.py --warm-start` is stage 5's other check and has no cache to be warm against yet.
- Item 18's `Core\Secret::reveal()` is not in the registry — `crates/nvs-config/src/secret.rs`.
- `Live::admit`'s same-class check is asked of the answer, not the argument —
  `crates/nvs-runtime/src/graph.rs` § *Known gaps*.
- `gaps.py`, `holes.py` and `check-migration.py --report` are the worklists no session re-derives.
