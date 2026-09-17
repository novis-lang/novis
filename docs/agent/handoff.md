# Handoff

## State

**Goal `decided-closures`, stage 4 — the library.** `python tools/owners.py --closes
decided-closures` names **5**, unchanged this session: `nvs-diagnostics/src/embedded.rs:30`,
`nvs-stdlib/src/command.rs:74`, `nvs-syntax/src/lib.rs:96`, `nvs-types/src/defaults.rs:58`,
`nvs-types/src/response.rs:29`. The other two stage-6 gates stay green.

**The blocker `command.rs` gap 1 names is not a gap — it is a stated bound, so that gap is one
slice and not two.** `crates/nvs-types/src/commands.rs` has no `# Known gaps` section at all: what
was its gap 1 is now § *What crosses in a row, and what stays* (`commands.rs:64-77`), where a row
carrying the *conversion* and never the type is permanent prose. Four citations of the number in
that module, and one in `command.rs`'s own gap 1, now name that section. A bound refusing a second
copy of the type lattice does not refuse the **reference** the `Decided:` sentence chose, so the
build below is the one the sheet decided and nothing about it is re-opened.

**Where the signature already is, which is what the next group otherwise re-derives.**
`nvs-runtime` has no `nvs-types` dependency and says so (`crates/nvs-runtime/src/commands.rs:20`),
so the reference cannot be a Rust one at the compiler's type. It does not need to be: the row
already holds it as `Command::handler`, `Class::method` (`crates/nvs-runtime/src/commands.rs:235`),
and `Ctx::class_desc` resolves a name against the program's own table first and the `Core` resolver
second (`crates/nvs-runtime/src/ctx/mod.rs:1267`), reaching `MethodRow`
(`crates/nvs-runtime/src/object.rs:625`). That row carries `arity`, `param_names` and `param_tags`
— **and no declared type**: a `Tag` nibble cannot tell `int` from `uint` and cannot name an enum,
which is exactly the page's missing sentence. So the one thing to build is a per-parameter declared
spelling on the signature side; the lookup path is already there.

## Next group

**Stage 4: the help page names its parameters' types** — one file set:
`crates/nvs-runtime/src/object.rs`, `crates/nvs-stdlib/src/command.rs`, and the two that fill a
`MethodRow`, `crates/nvs-types/src/layout.rs` and `crates/nvs-codegen/src/lib.rs:1617`.

- [ ] **Carry the declared spelling per parameter on `MethodRow` — `crates/nvs-runtime/src/object.rs:643`.**
      Beside `param_names`, on its exact terms: either `arity` names long or empty, where empty
      reads as "no declaration was read for this row". `nvs_types::layout` reads the declaration
      and `nvs-codegen` copies it down, which is `codec`'s path
      (`crates/nvs-runtime/src/object.rs:345`) and not a new
      seam. `rule:tooling/commands-are-compiled` is what the page owes; state what it spends per
      method per class in the field's own doc, per `rule:programs/memory-priority`.
- [ ] **Render it, and delete the gap — `crates/nvs-stdlib/src/command.rs:74`.** `help` splits
      `Command::handler` on `::`, asks `Ctx::class_desc`, and names each argument's type from the
      row it gets back; a row that answers empty renders exactly the page rendered before, which is
      every native and synthesized handler. The default is rendered with it, which is the half the
      gap says reads as absent alone.
- [ ] **Three `.nvst` cases, each a different question**, beside the page this one already pins —
      `tests/conformance/core/command-help-is-generated-from-the-compiled-table.nvst:1`. A `uint`
      option and an `int` one printing differently is the one a `Tag` could not have answered; an
      enum parameter naming its cases is the second; a handler whose row carries no spelling
      printing the older page is the boundary.

## Backlog

- `crates/nvs-diagnostics/src/embedded.rs:30` gap 1 — `autoload` probing; still owned by this goal.
- `crates/nvs-syntax/src/lib.rs:96` gap 1 — a bare inline shape type on a local declaration.
- `crates/nvs-types/src/defaults.rs:58` gap 1 — a named constant as a parameter default.
- `crates/nvs-types/src/response.rs:29` gap 1 — a mount's entry script.
- Per-parameter type on `MethodRow` is what `Core\Reflect\MethodInfo::parameters` would also answer;
  whether it takes it is `crates/nvs-stdlib/src/reflect.rs`'s call, not this group's.
