# Handoff

## State

**ADR 0042 § 2's descriptor question is decided, folded and demonstrated.** A warm hit builds every
`ClassDesc` itself, out of the IR the front end just lowered — `nvs_codegen::Descriptors`
(`crates/nvs-codegen/src/lib.rs:723`) is that surface, and the payload carries no class metadata at all
— so **what a hit skips is codegen, not the front end**. §§ 2–3 now carry the order (build, place,
relocate, protect, then bind) and the reason it is not a cycle; *Alternatives rejected* holds the
serialized-`ClassDesc` option and *Revisiting* names the one number that would reopen it, which is
§ *Verification*'s warm-versus-cold margin — stage 4's test.

`cache.rs`'s `this_process` is now the whole of § 3's resolver rather than a stand-in: the dummy
descriptor is gone, and `a_class_in_a_payload_reaches_the_descriptor_this_process_built` allocates an
instance, names its class and reads a field back through a descriptor this process built. The rustdoc
gate is green again (`crates/nvs-cli/src/cache.rs:456` linked a `SectionIndex` it had not imported).

**The acceptance check's failure was the documented cranelift flake, not a regression** — see the
playbook's *Running things*: reproducible with `script::` and `cache::` in one binary, gone under
`--test-threads=1`, and the four named stage-3 tests that exist all pass. Nothing is blocked.

## Next group

**The wiring.** One file set — `crates/nvs-codegen/src/lib.rs`, `crates/nvs-cli/src/cache.rs`,
`crates/nvs-cli/src/main.rs`.

- [ ] **Bind a placed payload's method tables** — § 3's last step, and the one thing a loaded unit still
      cannot do (a fixture may not call a method on an instance). Mirror the JIT's
      `bind_method_tables` (`crates/nvs-codegen/src/lib.rs:1617`) with an address source the loader
      supplies, and give `Loaded` the label-to-address lookup it reads through
      (`crates/nvs-cli/src/cache.rs:712`, `crates/nvs-cli/src/cache.rs:732`). The spelling is
      `nvs<index>_<sanitize(label)>` at `crates/nvs-codegen/src/lib.rs:1418`, the index being the
      position in the very `nvs_ir::Program` the loader also holds; `is_function_symbol` recognises one
      and `sanitize` is private, so either scan the payload's symbols per label or export the
      derivation.
- [ ] **Assemble a `Unit` from placed pages.** `Unit::_module` is a `JITModule`
      (`crates/nvs-codegen/src/lib.rs:311`) and a loaded unit's code is owned by `Loaded` instead, so
      that field has to become an owner it can hold either way; `entries`, `shapes` and `statics` all
      come from the IR or the payload's own symbols, which `finish` shows
      (`crates/nvs-codegen/src/lib.rs:1516`).
- [ ] **Consult the cache at the compile site**, `crates/nvs-cli/src/main.rs:1128`, then the three
      named tests. One wrinkle found this session: `config::boot_snapshot` runs *after* that compile
      (`crates/nvs-cli/src/main.rs:1160`), so the `[opcache]` directives naming the cache directory are
      not available where the lookup wants to sit — decide whether the snapshot moves earlier or the
      directory comes from a pre-boot default. `Cache::new` is `crates/nvs-cli/src/cache.rs:868`.

## Backlog

- Stage 4's `a_warm_start_is_faster_than_a_cold_one_by_the_margin_this_test_names` — the measurement ADR
  0042 § *Verification* now names, `docs/agent/loop-goal.toml:4294`.
- Stage 3's `a_second_run_of_the_same_program_does_not_compile_it` and its two miss siblings wait on the
  wiring above, `docs/agent/loop-goal.toml:4270`.
- Every `-p nvs-cli` check in this goal will keep flaking about half the time until something reduces
  that test's 14.5 GB of reserved address space; the playbook's *Running things* holds the measurement.
