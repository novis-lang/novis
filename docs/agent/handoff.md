# Handoff

## State

**Goal `decided-closures`, stage 4 — the library.** `Core\Reflect\EnumInfo` is built end to end, so
`rule:enums/reflection` is `shipped` rather than `designed` and names the five conformance cases
that pin it. `EnumInfo::of(Status::class)` answers a description carrying the enum's name, its
closed case list and which of `rule:enums/one-backing-type`'s two integer types the cases are
constants of; `valueOf` reads one constant and throws `LogicError` for a name the enum does not
declare.

**The shape had to be carried, because an enum has no descriptor.**
`rule:enums/representation` makes a case *be* the integer behind it, so nothing at run time can be
asked what enum it came from. The channel is `nvs_types::enums::EnumTable::iter` →
`nvs_ir::ir::Program::enums` → `nvs_runtime::ClassTable::define_enum` → `Ctx::enum_desc`, one list
beside the descriptors rather than a field on them, and `Core` enums ride it too because the
checker's table is seeded with them. `reflect.rs`'s module doc states what each end spends.

**`reflect.rs` gap 1 was stale in two ways, both in the direction the playbook's known-gap bullet
warns about.** `ParameterInfo` was already registered and `nvs_runtime::MethodRow::param_names`
already carried parameter names, so the gap's own stated reason — *"a parameter's name is in no
descriptor at all"* — was false about the crate it named. Rewritten to what the code does, under the
goal's standing decision that tested code beats a record: two classes are left, `ConstantInfo` and
`AttributeInfo`, and each needs class-constant or attribute data no `ClassDesc` carries.

**The gap is narrowed, not closed**, so `python tools/owners.py --closes decided-closures` still
names 17. `valueOf`'s `int|uint` return is the one place this class costs a caller anything — the
module doc owns why answering `int` always would be worse.

## Next group

**Stage 4: `Core\Reflect`'s remaining two gaps** — one file set:
`crates/nvs-stdlib/src/reflect.rs`, `crates/nvs-runtime/src/object.rs`, `crates/nvs-ir/src/ir.rs`,
`crates/nvs-types/src/layout.rs` and the cases under `tests/conformance/core/`. Gap 2 is the
smaller of the two and is the same carriage this session just built, one bit wide.

- [ ] **`crates/nvs-stdlib/src/reflect.rs:166` — gap 2: a `protected` member is not reached
      reflectively from a subclass's body.** `crates/nvs-stdlib/src/reflect.rs:1883` is the compare
      that is wrong — `site_class(args[3]).is_some_and(|site| site == class)`, repeated at `:1615`,
      `:1953`, `:2021` and `:2093`. What closes it is a second bit beside
      `crates/nvs-types/src/layout.rs:79`'s `public_fields`, carried through
      `crates/nvs-ir/src/ir.rs:193` to a `field_is_protected` next to
      `crates/nvs-runtime/src/object.rs:1278`, plus the same bit on `MethodRow`; the test then
      becomes per-member rather than once per call, since a `private` row still wants equality
      (`rule:security/reflection-enforces-visibility`).
- [ ] **`crates/nvs-stdlib/src/reflect.rs:152` — gap 1's remainder: `ConstantInfo` and
      `AttributeInfo`.** `crates/nvs-stdlib/src/reflect.rs:914`'s `PARAMETER_INFO` and this
      session's `ENUM_INFO` beside it are the two shapes to copy — a row class and a
      described-by-name class — and `crates/nvs-stdlib/src/reflect.rs:2580`'s `NOT_YET_BUILT` is the
      machine-checked worklist both come off (`rule:tooling/reflection-and-source-parsing-are-core-features`).

## Backlog

- The cross-request compiled-pattern caches are bracketed by nothing — `crates/nvs-stdlib/src/regex.rs:69` gap 1, waiting on the request arena.
- 15 further `decided-closures` gaps, one per line of `python tools/owners.py --closes decided-closures`.
- `crates/nvs-stdlib/src/test.rs:94` and `:102` — two gaps in one file, the next coherent group after `reflect.rs`.
- `crates/nvs-types/src/defaults.rs:58` gap 1 — a named constant as a parameter default.
