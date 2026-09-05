# Handoff

## State

**Goal 22 stage 2 is half landed: the descriptor half of the keystone.** `nvs-codegen` no longer
bakes a class-descriptor address in as an `iconst`. All three sites go through one helper,
`Emitter::class_desc_value` (`crates/nvs-codegen/src/emit.rs:2172`), which declares a
`Linkage::Import` data symbol and emits `symbol_value` — an import is undefined by this unit, so
every use of it leaves the relocation record ADR 0042 § 2's object payload needs. Under `JITModule`
the symbol resolves through a `symbol_lookup_fn` closure `Jit::new` installs over `Jit::desc_symbols`,
which `compile_all` fills from `Classes::descriptors()` before any body is emitted. `is_pic` is off,
so a symbol value is the same absolute `movabs` the `iconst` was: **the whole `-p nvs-codegen` suite
passes unchanged**, which is the hot-path claim's evidence. `Classes`' own doc comment
(`crates/nvs-codegen/src/lib.rs:534`) is the one home for why.

**`method_address` was never a third site.** Its statically resolved target is already a `func_addr`
against a `FuncId` (`crates/nvs-codegen/src/emit.rs:2142`), which is a relocation whichever `Module`
finalizes it. Nothing there needs changing — only the check that names it, which is worth writing
where an object product's relocation table can be read rather than guessed at.

Two of stage 2's five named checks are green:
`a_class_descriptor_address_is_a_relocation_not_an_immediate` and
`an_instanceof_target_is_a_relocation_not_an_immediate`, both `-p nvs-codegen` lib unit tests,
because they assert on `Jit`'s private tables and nothing outside `src/lib.rs` can reach those.
Nothing is blocked.

## Next group

**Stage 2's remaining half: the second `Module`.** One file set — `crates/nvs-codegen/src/lib.rs`,
`crates/nvs-codegen/src/emit.rs`, `Cargo.toml`, `crates/nvs-codegen/Cargo.toml`.

- [ ] **The lowering walk becomes generic over `M: Module`** — the goal's standing decision forbids a
      second lowering, so this is the whole keystone. Three concrete signatures hold `JITModule`:
      `crates/nvs-codegen/src/emit.rs:351` (`Emitter.module`),
      `crates/nvs-codegen/src/emit.rs:127` (`compile_function`'s parameter) and
      `crates/nvs-codegen/src/lib.rs:1288` (`Signatures::new`). Everything they call —
      `declare_data`, `declare_data_in_func`, `declare_func_in_func`, `make_signature` — is on the
      `Module` trait already, so this is a parameter change, not a redesign. `&mut dyn Module` is the
      cheaper spelling if a generic bound turns viral.
- [ ] **`cranelift-object` behind that same walk** — add `cranelift-object = "0.135"` beside its four
      siblings in the workspace manifest's `[workspace.dependencies]` (the four `cranelift*` lines,
      `rg -n 'cranelift-jit' Cargo.toml`) and at `crates/nvs-codegen/Cargo.toml:17`, then an `ObjectModule`
      built with the *same* ISA flags as `crates/nvs-codegen/src/lib.rs:998` except that `is_pic`
      must be **on** for a relocatable object, and with no `symbol_lookup_fn`: leaving every
      `nvs_class_desc_*` undefined is the entire point of the previous slice. Pins
      `the_object_module_emits_every_program_the_jit_module_does`.
- [ ] **The two remaining named checks** — `a_statically_resolved_call_target_is_a_relocation_not_an_immediate`
      reads the object product's relocations for the callee symbol, and
      `the_two_modules_answer_the_same_for_every_lowering_fixture` walks the same fixtures both ways.
      Both want the object module first; `crates/nvs-codegen/src/lib.rs:1500` is where the JIT-side
      helpers already sit.

## Backlog

- Stage 3's warm-hit read path relocates a private writable mapping — `crates/nvs-cli/src/cache.rs`'s
  "Known gaps" states the amendment ADR 0042 § 3 still owes.
- The ADR 0042 §§ 2-3 fold itself, once stage 2 proves reachable — the goal's standing decisions.
- `class_desc_symbol` is `pub(crate)`; the stage-3 loader in `nvs-cli` will need it public.
