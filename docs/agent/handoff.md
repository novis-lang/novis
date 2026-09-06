# Handoff

## State

**ADR 0042 §§ 2-3 now state the amendment in their own bodies**, which is what stage 2 had earned.
§ 2 says the payload is one host-format relocatable object per unit — what
`nvs_codegen::compile_object` writes, defining this unit's own functions and leaving every runtime
helper and every `nvs_class_desc_*` undefined — and § 3 says a reader maps it `MAP_PRIVATE` and
writable, verifies, relocates, and only then `mprotect`s. Folded with them, so that nothing in the
ADR still describes a page dump: the *In short*, both *Investigation* bullets, one *Consequences*
bullet, one *Alternatives rejected* bullet, and `crates/nvs-cli/src/cache.rs`'s *Known gaps*, which
now points at those two sections instead of carrying the amendment itself.

`nvs_codegen::class_desc_symbol` is `pub` (`crates/nvs-codegen/src/lib.rs:1799`): the loader that
resolves a descriptor relocation is in another crate and must derive the identical name.
`crates/nvs-cli/src/cache.rs`'s `a_published_artifact_is_the_object_the_compiler_wrote` compiles a
program through this binary's own front end, stores `compile_object`'s bytes, reads them back and
finds `nvs_class_desc_Widget` *undefined* in the artifact — `object` is a new dev-dependency of
`nvs-cli` for that read. Stage 3's six named tests are all still unwritten; nothing is blocked.

Two facts the loader slice needs and no doc holds yet: every function is declared `Linkage::Local`
(`crates/nvs-codegen/src/lib.rs:1421`) as `nvs<index>_<sanitize(name)>`, so the entry frame is a
*static* symbol derived from `ENTRY_SCRIPT_LABEL` and a loader finds it by walking the symbol table
rather than by asking for an export; and `nvs_runtime::symbols()` / `nvs_stdlib::symbols()` are the
name-to-address tables the JIT resolves helpers through (`crates/nvs-codegen/src/lib.rs:1285`), both
of which `nvs-cli` already depends on.

## Next group

**Stage 3: the relocating read path.** One file set — `crates/nvs-cli/src/cache.rs`,
`crates/nvs-cli/Cargo.toml`, `crates/nvs-cli/src/main.rs`.

- [ ] **The loader: a `Verified` becomes executable pages.** `memmap2::MmapOptions::map_copy` is
      § 3's private writable mapping and `MmapMut::make_exec` its one-way transition;
      `object::File::parse` over the payload gives the sections, symbols and relocations, so `object`
      is promoted from a dev-dependency to a dependency. Resolve a helper through
      `nvs_runtime::symbols()`/`nvs_stdlib::symbols()` and an `nvs_class_desc_*` through this
      process's own descriptors — `crates/nvs-codegen/src/lib.rs:1399` is where the JIT publishes
      exactly that map. **Decide the >2 GB question first** (see the backlog): a PC-relative call from
      a mapped artifact to a helper in this image is the one relocation that can be unrepresentable,
      and the answer is either a stub table this loader emits or a placement constraint on the
      mapping. `crates/nvs-cli/src/cache.rs:246`, `crates/nvs-cli/src/cache.rs:417`.
- [ ] **The five tests that need only the loader**, `-p nvs-cli`, named by
      `docs/agent/loop-goal.toml:4271`:
      `a_warm_hit_maps_private_writable_relocates_then_makes_the_pages_executable`,
      `a_payload_whose_checksum_fails_is_a_miss_and_not_an_error`,
      `a_payload_written_by_a_different_toolchain_is_a_miss`,
      `an_absent_or_unwritable_cache_directory_is_a_miss_and_the_run_succeeds`. The last three are
      close to `a_tampered_artifact_is_rejected`'s ground but must be their own functions — the check
      matches names. `crates/nvs-cli/src/cache.rs:1066`, `crates/nvs-cli/src/cache.rs:1023`.
- [ ] **The wiring, and the two tests that need it**:
      `a_second_run_of_the_same_program_does_not_compile_it` and
      `an_edited_source_file_is_a_miss_on_the_next_run`. `nvs run` compiles at
      `crates/nvs-cli/src/main.rs:1128` with the key's inputs already in hand at
      `crates/nvs-cli/src/main.rs:1097`; `nvs_config::cache::{content_hash, artifact_key}` is the
      key, and `#![allow(dead_code)]` at `crates/nvs-cli/src/cache.rs:135` comes off when this lands.

## Backlog

- Stage 4's `a_warm_start_is_faster_than_a_cold_one_by_the_margin_this_test_names` — the bench that
  names the margin, `docs/agent/loop-goal.toml:4293`.
- A JIT call between two functions of one unit is colocated, so it is PC-relative and
  `cranelift-jit` *panics* if the two land over 2 GB apart — seen once under the 10k-concurrent
  compile test, playbook § *Running things*. The loader above has to answer the same question for a
  mapped artifact, and nothing owns it yet.
- `UnitBuilder<ObjectModule>` binds no method tables: a `MethodRow` holds a code address and there is
  none until a loader places one — `finish_object`'s own doc comment says so, and the loader slice is
  where it becomes a question.
