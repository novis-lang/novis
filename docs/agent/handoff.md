# Handoff

## State

**Goal 22 stage 2 is complete: all five of its named checks are green.** `nvs-codegen` now has one
lowering walk and two `Module`s. `emit.rs` names no concrete module at all — `emit_function` and
`Emitter.module` take `&mut dyn Module` (`crates/nvs-codegen/src/emit.rs:137`,
`crates/nvs-codegen/src/emit.rs:362`), and the file's own `# One walk, two Module`s` section is the
home for why dynamic dispatch is affordable here.

`Jit` is now `UnitBuilder<M>` (`crates/nvs-codegen/src/lib.rs:529`), because it is no longer only a
JIT: `UnitBuilder<JITModule>::new`/`finish` is the in-process backend and
`UnitBuilder<ObjectModule>::for_object`/`finish_object` writes ADR 0042 § 2's relocatable object,
while `compile_all`, `compile_function` and every table are shared code on `impl<M: Module>`.
`pub fn compile_object(&Program) -> Vec<u8>` (`crates/nvs-codegen/src/lib.rs:489`) is the entry point
stage 3 will read from. `host_isa(is_pic)` (`crates/nvs-codegen/src/lib.rs:1045`) is the single home
for the ISA and its flags; `is_pic` is the only one the two backends disagree about, and the object
gets no symbol table of any kind — that is what leaves every helper and every `nvs_class_desc_*`
undefined.

Five unit tests in `src/lib.rs`'s `mod tests` carry stage 2, sharing one `BOTH_BACKENDS` fixture
list. `cranelift-object` and a `read`-only `object` dev-dependency are the only new crates; the lock
gained one package. Nothing is blocked.

## Next group

**Stage 3: the warm hit, and the ADR fold stage 2 has now earned.** One file set —
`docs/adr/0042-on-disk-artifact-cache-format.md`, `crates/nvs-cli/src/cache.rs`,
`crates/nvs-codegen/src/lib.rs`.

- [ ] **Fold the amendment into ADR 0042 §§ 2-3's own bodies** — the goal's standing decision says
      this goal opens no ADR number and folds into these two sections. § 2 must state that the
      payload is what `nvs_codegen::compile_object` writes — a host-format relocatable object whose
      undefined symbols are the runtime helpers and `nvs_class_desc_*` — and § 3 that a reader
      relocates a private writable mapping before `mprotect`, which is the amendment
      `crates/nvs-cli/src/cache.rs`'s *Known gaps* still owes.
      `docs/adr/0042-on-disk-artifact-cache-format.md:113` and
      `docs/adr/0042-on-disk-artifact-cache-format.md:124`.
- [ ] **`class_desc_symbol` becomes `pub`, and `cache.rs`'s writer stores a real payload** — the
      loader resolving a descriptor relocation lives in another crate and derives the same name.
      `crates/nvs-codegen/src/lib.rs:1623`, `crates/nvs-cli/src/cache.rs:147`.
- [ ] **Stage 3's read path**, `-p nvs-cli`: map private writable, relocate, then make the pages
      executable, and a failed verification is a miss and never an error. Six named tests, the first
      being `a_warm_hit_maps_private_writable_relocates_then_makes_the_pages_executable`
      (`docs/agent/loop-goal.toml:4273`). `crates/nvs-cli/src/cache.rs:115`,
      `crates/nvs-cli/src/cache.rs:259`, `crates/nvs-cli/src/cache.rs:970`.

The orientation pack sliced only ADR 0042's *In short*; the group above needs its §§ 2-3, so add
`0042` §§ 2 and 3 to `[context] adrs` in `docs/agent/loop-goal.toml`.

## Backlog

- Stage 4's `a_warm_start_is_faster_than_a_cold_one_by_the_margin_this_test_names` — the bench that
  names the margin, `docs/agent/loop-goal.toml:4293`.
- A JIT call between two functions of one unit is colocated, so it is PC-relative and
  `cranelift-jit` *panics* if the two land over 2 GB apart — seen once under the 10k-concurrent
  compile test, playbook § *Running things*. Nothing owns this yet; ADR 0042's loader will have to
  answer the same question for a mapped artifact.
- `UnitBuilder<ObjectModule>` binds no method tables: a `MethodRow` holds a code address and there is
  none until a loader places one — `finish_object`'s own doc comment says so, and stage 3 is where it
  becomes a question.
