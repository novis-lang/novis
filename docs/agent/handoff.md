# Handoff

## State

**ADR 0042 § 3's loader is on disk and green.** `Verified::relocate`
(`crates/nvs-cli/src/cache.rs:322`) maps a private anonymous region of its own, copies each
allocatable section of the payload into it at that section's alignment, resolves every undefined
symbol through a caller-supplied `&dyn Fn(&str) -> Option<*const u8>`, and `make_exec`s — the
`MmapMut` that was the one writable view is consumed by that call, so W^X is a move rather than a
rule. § 3, the *In short* and the mapping *Investigation* bullet now say the running pages are the
**reader's, not the file's**, and why: a relocatable object's sections are laid out for a linker to
place, a section with no file bytes cannot be placed in place at all, and only a mapping the reader
owns has room for the landing area. `Cache::load`'s mapping stays read-only for its whole life.

**The >2 GB question is decided and recorded** (ADR 0042 § 3, and this module's § *§ 3's loader*):
a landing area inside the loader's own mapping — eight bytes holding a symbol's full address, and
`jmp qword ptr [rip + 0]` in front of them for a call — so every displacement written names a
target inside that mapping. On this host it stays empty, because COFF already reaches an import
through a `.rdata$.refptr` cell; it is ELF's `PltRelative`/`GotRelative` pair the area exists for.
`aarch64` is deliberately `Architecture::Unknown` (icache maintenance has no home yet), so every
artifact is a miss there.

Three of stage 3's six named tests are on disk and pass. Nothing is blocked.

## Next group

**Stage 3's wiring.** One file set — `crates/nvs-cli/src/main.rs`, `crates/nvs-cli/src/cache.rs`,
`crates/nvs-codegen/src/lib.rs`.

- [ ] **Decide where a warm hit's class descriptors come from, and fold it into ADR 0042 § 2.**
      This is the wiring's real question and it is not settled: § 2 leaves every `nvs_class_desc_*`
      undefined and § 3 resolves it against "the `ClassDesc` this process allocated", but a run that
      skips the compile has allocated none, and `nvs-codegen` publishes no table of them —
      `crates/nvs-codegen/src/lib.rs:1399` is where the JIT fills its own, privately. Either the
      descriptors are built without a codegen pass (from the layouts the front end already has, so
      a warm hit skips codegen but not the front end — which is what
      `a_second_run_of_the_same_program_does_not_compile_it` literally asks for), or § 2 has to
      carry them in the payload. Pick the first unless it does not hold up: it needs no format
      change and no serialized runtime type. `crates/nvs-codegen/src/lib.rs:1399`,
      `crates/nvs-cli/src/cache.rs:322`.
- [ ] **The wiring itself.** `crates/nvs-cli/src/main.rs:1128` is the `nvs_codegen::compile` a warm
      hit replaces: the `Cache::load` goes in front of it and the `store` behind it, both keyed on
      `artifact_key(content_hash(<the source bytes>), env_hash(config))` — not on the payload's own
      bytes, which is only what this module's tests key on for convenience. Every failure on that
      path is a miss that falls through to the compile. Drop `cache.rs`'s `#![allow(dead_code)]`
      when it lands. `crates/nvs-cli/src/main.rs:1128`, `crates/nvs-cli/src/cache.rs:920`.
- [ ] **The three named tests that need the wiring**, `-p nvs-cli --bin nvs`:
      `a_second_run_of_the_same_program_does_not_compile_it`,
      `an_absent_or_unwritable_cache_directory_is_a_miss_and_the_run_succeeds`,
      `an_edited_source_file_is_a_miss_on_the_next_run`. The tests module already has `object_for`,
      `this_process` and `output_of` to build on. `crates/nvs-cli/src/cache.rs:1102`,
      `crates/nvs-cli/src/main.rs:1128`.

## Backlog

- The test resolver hands an `nvs_class_desc_*` a stable dummy address — `cache.rs` *Known gaps*.
- `aarch64` needs instruction-cache maintenance before it can load — `cache.rs` *Known gaps*.
- `Unloadable`'s variants are distinguished only for tests; nothing logs one — ADR 0042 § 3.
