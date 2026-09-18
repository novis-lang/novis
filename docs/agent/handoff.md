# Handoff

## State

**Goal `test-doubles`, stage 2. The pick stage 2's prose left open is made**, and recorded where it
said to: `crates/nvs-stdlib/src/test.rs:1`'s module doc now holds two decisions — a double's
descriptor is built by `nvs-stdlib` at the call that asks for one, with `nvs-ir` learning nothing
about doubles, and a trampoline carries its slot in its own identity. The one runtime API that pick
needs is on disk with its test: `nvs_runtime::ClassTable::define_conforming`
(`crates/nvs-runtime/src/object.rs:1897`), which names a parent by address because an interface's
descriptor belongs to the compiled unit and not to the table holding the double's own class.

**No registry row yet, and the remaining three items are one landing, not three commits.**
`crates/nvs-stdlib/tests/conformance_coverage.rs:52`'s
`every_part_one_member_has_a_conformance_case` sweeps `registry::CLASSES` for a case spelling
`Core\Test::double<`, so a `double` row committed ahead of its case makes `cargo test -p nvs-stdlib`
red — the playbook's *Registering a `Core` member and writing its conformance case are one slice,
not two* is the general form, and it is why this session stopped short of the row rather than
landing half of it.

Two ABI facts the next session would otherwise re-derive. A `WRITTEN_CLASS_MEMBERS` row's helper is
handed the written class's descriptor as argument 0, the `array<…>` flag as 1 and the wire contract
as 2, ahead of everything including a receiver (`crates/nvs-stdlib/src/registry.rs:2999`), so
`double<T>($answers)` is `args: [4]`. And `nvs_runtime::run_helper` takes the arity as a *value*
(`crates/nvs-runtime/src/abi.rs:688`) where `nvs_helper!` passes a literal — a trampoline's arity is
its own row's and is not known until the receiver has been read, so it cannot be written with that
macro as it stands.

## Next group

**Stage 2, the rest, as one landing because the coverage gate makes it one** — one file set:
`crates/nvs-stdlib/src/test.rs`, `crates/nvs-stdlib/src/registry.rs`,
`crates/nvs-types/src/expr/args.rs`, `tests/conformance/core/`.

- [ ] **The trampoline table and the descriptor cache** — the module doc at
      `crates/nvs-stdlib/src/test.rs:1` is the specification: one native entry per slot against a
      ceiling, the `$`-prefixed field names, and a leaked table keyed by `(interface, real class,
      overridden names)` built through `crates/nvs-runtime/src/object.rs:1897`'s
      `define_conforming`. `crates/nvs-stdlib/src/instance.rs:588`'s `build`, `:715`'s `set_slot`
      and `:734`'s `slot` are the primitives. `rule:testing/doubles`.
- [ ] **The `double` row, its card, its `address()` arm and its helper** —
      `crates/nvs-stdlib/src/test.rs:321`'s `CLASS` gains a row whose parameter and return type are
      `CoreTy::Written("T")` (`crates/nvs-stdlib/src/registry.rs:554`), and
      `crates/nvs-stdlib/src/test.rs:1952`'s `address` its arm. `rule:testing/doubles`.
- [ ] **The interface's descriptor reaches the helper** — `crates/nvs-stdlib/src/registry.rs:3030`'s
      `WRITTEN_CLASS_MEMBERS` gains the pair, and `crates/nvs-types/src/expr/args.rs:1671`'s
      `written_class_of` has to admit an interface where it admits a class. `rule:testing/doubles`.
- [ ] **The three `core/` cases**, which are what makes the row legal to commit —
      `tests/conformance/core/a-double-is-passed-where-its-interface-is-taken.nvst` and its two
      siblings, with ADR 0079 § 10's worked example as the first one's body. The gate counting them
      is `crates/nvs-stdlib/tests/conformance_coverage.rs:52` and the shape to copy is
      `tests/conformance/core/json-derive-encodes-declared-fields.nvst:1`. `rule:testing/doubles`.

## Backlog

- `a_double_records_every_call_it_answers`, stage 2's other check — writable the moment the
  trampolines exist, against `crates/nvs-stdlib/src/test.rs`'s own test module.
- `partial`'s delegating rows, and its line in
  `crates/nvs-stdlib/tests/spec-members-compiler-facing-outstanding.txt:30`.
- Stage 3's four refusals — `docs/agent/loop-goal.md` § *Stage 3*.
